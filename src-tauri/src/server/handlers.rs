use std::collections::HashMap;

use axum::extract::{Request, Path};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::stream::StreamExt;
use reqwest::Client;
use serde_json::Value;
use tracing::{error, info};

use crate::converter::generators::anthropic::AnthropicGenerator;
use crate::converter::generators::completions::CompletionsGenerator;
use crate::converter::generators::gemini::GeminiGenerator;
use crate::converter::generators::responses::ResponsesGenerator;
use crate::converter::ir::ClientFormat;
use crate::converter::parsers::anthropic::AnthropicParser;
use crate::converter::parsers::completions::CompletionsParser;
use crate::converter::parsers::gemini::GeminiParser;
use crate::converter::parsers::responses::ResponsesParser;
use crate::converter::{FormatGenerator, FormatParser};
use crate::error::ProxyError;
use crate::interceptor::engine::InterceptorEngine;
use crate::key::rotation::{KeyRotation, RotationStrategy};
use crate::key::store::decrypt_api_key;
use crate::logging::store::log_request;
use crate::provider::manager::ProviderManager;
use crate::usage::tracker::UsageTracker;

pub async fn handle_completions(request: Request) -> Response {
    handle_proxy(request, ClientFormat::Completions, None, false).await
}

pub async fn handle_responses(request: Request) -> Response {
    handle_proxy(request, ClientFormat::Responses, None, false).await
}

pub async fn handle_anthropic(request: Request) -> Response {
    handle_proxy(request, ClientFormat::Anthropic, None, false).await
}

pub async fn handle_gemini(Path(model_segment): Path<String>, request: Request) -> Response {
    let (model, is_stream) = parse_gemini_model_segment(&model_segment);
    handle_proxy(request, ClientFormat::Gemini, Some(model), is_stream).await
}

fn parse_gemini_model_segment(segment: &str) -> (String, bool) {
    let is_stream = segment.contains("streamGenerateContent");
    let model = segment
        .split(':')
        .next()
        .unwrap_or(segment)
        .to_string();
    (model, is_stream)
}

async fn handle_proxy(
    request: Request,
    client_format: ClientFormat,
    override_model: Option<String>,
    force_stream: bool,
) -> Response {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    let (parts, body) = request.into_parts();

    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return ProxyError::Parse(format!("failed to read body: {}", e)).into_response();
        }
    };

    let body_value: Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return ProxyError::Parse(format!("invalid JSON: {}", e)).into_response();
        }
    };

    let parser = get_parser(&client_format);
    let generator = get_generator(&client_format);

    let mut ir_request = match parser.parse_request(&body_value) {
        Ok(r) => {
            info!("Parsed request: model={}, stream={}", r.model, r.stream);
            r
        }
        Err(e) => {
            error!("Parse request error: {}", e);
            return e.into_response();
        }
    };

    if let Some(model) = override_model {
        ir_request.model = model;
    }
    if force_stream {
        ir_request.stream = true;
    }

    let mut extra_headers: HashMap<String, String> = HashMap::new();
    extract_headers(&parts.headers, &mut extra_headers);

    let path = parts.uri.path().to_string();

    if let Err(e) = InterceptorEngine::execute_pre_rules(&mut ir_request, &path, &mut extra_headers).await {
        error!("Interceptor error: {}", e);
    }

    let route = match ProviderManager::find_for_model(&ir_request.model).await {
        Ok(r) => {
            info!("Route found: model={} -> {} ({:?} via {})", ir_request.model, r.target_model, r.target_format, r.provider_name);
            r
        }
        Err(e) => {
            error!("Route not found for model '{}': {}", ir_request.model, e);
            return e.into_response();
        }
    };

    let selected_key = match KeyRotation::get_next_key(&route.provider_id, &RotationStrategy::LeastUsed).await {
        Ok(k) => k,
        Err(e) => {
            return e.into_response();
        }
    };

    let nonce_slice: Vec<u8> = selected_key.nonce;
    let mut nonce_array = [0u8; 12];
    if nonce_slice.len() == 12 {
        nonce_array.copy_from_slice(&nonce_slice);
    } else {
        return ProxyError::KeyManagement("invalid nonce length".into()).into_response();
    }

    let api_key = match decrypt_api_key(&selected_key.encrypted_key, &nonce_array) {
        Ok(k) => k,
        Err(e) => {
            return e.into_response();
        }
    };

    let target_model = route.target_model.clone();
    ir_request.model = target_model.clone();

    let target_generator = get_generator(&route.target_format);

    let mut ir_request_for_upstream = ir_request.clone();

    if client_format == ClientFormat::Gemini && ir_request.stream {
        ir_request_for_upstream.stream = true;
    }

    let target_body = match target_generator.generate_request(&ir_request_for_upstream) {
        Ok(b) => b,
        Err(e) => {
            return e.into_response();
        }
    };

    let mut url = format!("{}{}", route.base_url.trim_end_matches('/'), route.endpoint_path);

    if client_format == ClientFormat::Gemini && ir_request.stream {
        url = url.replace(":generateContent", ":streamGenerateContent");
    }

    info!("Upstream request: {} {}", "POST", url);

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    let mut req_builder = http_client
        .post(&url)
        .json(&target_body)
        .header("Content-Type", "application/json");

    match route.target_format {
        ClientFormat::Anthropic => {
            req_builder = req_builder.header("x-api-key", &api_key);
        }
        _ => {
            req_builder = req_builder.bearer_auth(&api_key);
        }
    }

    for (key, value) in &extra_headers {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            let err = ProxyError::Network(format!("request to provider failed: {}", e));
            let _ = log_request_entry(
                &request_id,
                &client_format,
                &route.provider_name,
                &route.target_format,
                &ir_request.model,
                ir_request.stream,
                start.elapsed().as_millis() as i64,
                Some(err.to_string().as_str()),
                0,
                0,
                0,
                None,
            )
            .await;
            return err.into_response();
        }
    };

    let status = resp.status();
    let is_stream = ir_request.stream;

    if !status.is_success() {
        let status_code = status.as_u16();
        let resp_body = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return ProxyError::Network(format!("failed to read error response: {}", e)).into_response();
            }
        };
        let body_text = String::from_utf8_lossy(&resp_body).into_owned();
        error!("Upstream error {}: {}", status_code, body_text);
        let mut response = body_text.into_response();
        *response.status_mut() = status;
        return response;
    }

    if !is_stream {
        let resp_body = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return ProxyError::Network(format!("failed to read response: {}", e)).into_response();
            }
        };

        let resp_value: Value = match serde_json::from_slice(&resp_body) {
            Ok(v) => v,
            Err(e) => {
                return ProxyError::Parse(format!("invalid response JSON: {}", e)).into_response();
            }
        };

        let target_parser = get_parser(&route.target_format);
        let ir_response = match target_parser.parse_response(&resp_value) {
            Ok(r) => r,
            Err(e) => {
                return e.into_response();
            }
        };

        let client_response = match generator.generate_response(&ir_response) {
            Ok(r) => r,
            Err(e) => {
                return e.into_response();
            }
        };

        let prompt_tokens = ir_response.usage.prompt_tokens as i64;
        let completion_tokens = ir_response.usage.completion_tokens as i64;
        let cached_tokens = ir_response.usage.cached_tokens as i64;

        let _ = log_request_entry(
            &request_id,
            &client_format,
            &route.provider_name,
            &route.target_format,
            &ir_request.model,
            false,
            start.elapsed().as_millis() as i64,
            None,
            prompt_tokens,
            completion_tokens,
            cached_tokens,
            Some(start.elapsed().as_millis() as i64),
        )
        .await;

        let _ = UsageTracker::record(
            &target_model,
            &route.provider_name,
            prompt_tokens,
            completion_tokens,
        )
        .await;

        let mut response = axum::Json(client_response).into_response();
        *response.status_mut() = status;
        response
    } else {
        let target_parser = get_parser(&route.target_format);

        let stream = resp.bytes_stream();

        let response_id = uuid::Uuid::new_v4().to_string();
        let model_name = ir_request.model.clone();

        let sse_stream = async_stream::stream! {
            let mut buffer = String::new();
            let mut total_prompt = 0u32;
            let mut total_completion = 0u32;
            let mut total_cached = 0u32;
            let mut reader = stream;
            let mut ttft_ms: Option<i64> = None;

            // State machine for Responses SSE events
            let mut output_index: usize = 0;
            let mut text_item_open = false;
            let mut func_call_open = false;
            let mut finished = false;
            let mut current_call_id = String::new();
            let mut current_func_name = String::new();
            let mut accumulated_text = String::new();
            let mut accumulated_args = String::new();

            // Emit response.created
            let created = serde_json::json!({
                "type": "response.created",
                "response": {
                    "id": response_id,
                    "object": "response",
                    "status": "in_progress",
                    "model": model_name,
                    "output": [],
                }
            });
            yield Ok::<_, std::convert::Infallible>(Bytes::from(format!("data: {}\n\n", created)));

            while let Some(chunk_result) = reader.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Stream error: {}", e);
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let ir_chunk = match target_parser.parse_stream_chunk(trimmed) {
                        Ok(Some(c)) => c,
                        Ok(None) => continue,
                        Err(e) => {
                            error!("Stream chunk parse error: {}", e);
                            continue;
                        }
                    };

                    if let Some(usage) = &ir_chunk.usage {
                        total_prompt += usage.prompt_tokens;
                        total_completion += usage.completion_tokens;
                        total_cached += usage.cached_tokens;
                    }

                    if ttft_ms.is_none() && (ir_chunk.delta_content.is_some() || ir_chunk.delta_tool_calls.is_some()) {
                        ttft_ms = Some(start.elapsed().as_millis() as i64);
                    }

                    // Handle tool call deltas
                    if let Some(tool_calls) = &ir_chunk.delta_tool_calls {
                        if let Some(tc) = tool_calls.first() {
                            // New tool call: close text item first, then open function_call
                            if tc.id.is_some() && tc.name.is_some() {
                                if text_item_open {
                                    let text_done = serde_json::json!({
                                        "type": "response.output_text.done",
                                        "output_index": output_index - 1,
                                        "content_index": 0,
                                        "text": accumulated_text,
                                    });
                                    let item_done = serde_json::json!({
                                        "type": "response.output_item.done",
                                        "output_index": output_index - 1,
                                        "item": {
                                            "type": "message",
                                            "id": "msg_proxy",
                                            "role": "assistant",
                                            "content": [{"type": "output_text", "text": accumulated_text}],
                                            "status": "completed",
                                        }
                                    });
                                    yield Ok::<_, std::convert::Infallible>(Bytes::from(
                                        format!("data: {}\n\ndata: {}\n\n", text_done, item_done)
                                    ));
                                    text_item_open = false;
                                    accumulated_text.clear();
                                }

                                current_call_id = tc.id.as_deref().unwrap_or("").to_string();
                                current_func_name = tc.name.as_deref().unwrap_or("").to_string();
                                let added = serde_json::json!({
                                    "type": "response.output_item.added",
                                    "output_index": output_index,
                                    "item": {
                                        "type": "function_call",
                                        "id": format!("fc_{}", current_call_id),
                                        "call_id": current_call_id,
                                        "name": current_func_name,
                                        "arguments": "",
                                    }
                                });
                                yield Ok::<_, std::convert::Infallible>(Bytes::from(format!("data: {}\n\n", added)));
                                func_call_open = true;
                                output_index += 1;
                            }
                            // Arguments delta
                            if let Some(args) = &tc.arguments {
                                accumulated_args.push_str(args);
                                let delta_event = serde_json::json!({
                                    "type": "response.function_call_arguments.delta",
                                    "output_index": output_index - 1,
                                    "item_id": format!("fc_{}", current_call_id),
                                    "call_id": current_call_id,
                                    "delta": args,
                                });
                                yield Ok::<_, std::convert::Infallible>(Bytes::from(format!("data: {}\n\n", delta_event)));
                            }
                        }
                    }

                    // Handle text content
                    if let Some(content) = &ir_chunk.delta_content {
                        if content.is_empty() || func_call_open || finished {
                            // skip
                        } else {
                            if !text_item_open {
                                let item_added = serde_json::json!({
                                    "type": "response.output_item.added",
                                    "output_index": output_index,
                                    "item": {
                                        "type": "message",
                                        "id": "msg_proxy",
                                        "role": "assistant",
                                        "content": [],
                                        "status": "in_progress",
                                    }
                                });
                                let part_added = serde_json::json!({
                                    "type": "response.content_part.added",
                                    "output_index": output_index,
                                    "content_index": 0,
                                    "part": {"type": "output_text", "text": ""},
                                });
                                yield Ok::<_, std::convert::Infallible>(Bytes::from(
                                    format!("data: {}\n\ndata: {}\n\n", item_added, part_added)
                                ));
                                text_item_open = true;
                                output_index += 1;
                            }
                            accumulated_text.push_str(content);
                            let delta_event = serde_json::json!({
                                "type": "response.output_text.delta",
                                "output_index": output_index - 1,
                                "content_index": 0,
                                "delta": content,
                            });
                            yield Ok::<_, std::convert::Infallible>(Bytes::from(format!("data: {}\n\n", delta_event)));
                        }
                    }

                    // Handle finish
                    if ir_chunk.finish_reason.is_some() && !finished {
                        finished = true;
                        // Close function_call if open
                        if func_call_open {
                            let args_done = serde_json::json!({
                                "type": "response.function_call_arguments.done",
                                "output_index": output_index - 1,
                                "item_id": format!("fc_{}", current_call_id),
                                "call_id": current_call_id,
                                "arguments": accumulated_args,
                            });
                            let fc_done = serde_json::json!({
                                "type": "response.output_item.done",
                                "output_index": output_index - 1,
                                "item": {
                                    "type": "function_call",
                                    "id": format!("fc_{}", current_call_id),
                                    "call_id": current_call_id,
                                    "name": current_func_name,
                                    "arguments": accumulated_args,
                                }
                            });
                            yield Ok::<_, std::convert::Infallible>(Bytes::from(
                                format!("data: {}\n\ndata: {}\n\n", args_done, fc_done)
                            ));
                            func_call_open = false;
                        }
                        // Close text message if open
                        if text_item_open {
                            let text_done = serde_json::json!({
                                "type": "response.output_text.done",
                                "output_index": output_index - 1,
                                "content_index": 0,
                                "text": accumulated_text,
                            });
                            let item_done = serde_json::json!({
                                "type": "response.output_item.done",
                                "output_index": output_index - 1,
                                "item": {
                                    "type": "message",
                                    "id": "msg_proxy",
                                    "role": "assistant",
                                    "content": [{"type": "output_text", "text": accumulated_text}],
                                    "status": "completed",
                                }
                            });
                            yield Ok::<_, std::convert::Infallible>(Bytes::from(
                                format!("data: {}\n\ndata: {}\n\n", text_done, item_done)
                            ));
                            text_item_open = false;
                        }
                        // response.completed
                        let completed = serde_json::json!({
                            "type": "response.completed",
                            "response": {
                                "id": ir_chunk.id.as_deref().unwrap_or(&response_id),
                                "object": "response",
                                "status": "completed",
                                "output": [],
                            }
                        });
                        yield Ok::<_, std::convert::Infallible>(Bytes::from(format!("data: {}\n\n", completed)));
                    }
                }
            }

            // Safety: close any remaining open items if stream ended without finish_reason
            if func_call_open {
                let fc_done = serde_json::json!({
                    "type": "response.output_item.done",
                    "output_index": output_index - 1,
                    "item": {
                        "type": "function_call",
                        "id": format!("fc_{}", current_call_id),
                        "call_id": current_call_id,
                        "name": current_func_name,
                        "arguments": accumulated_args,
                    }
                });
                yield Ok::<_, std::convert::Infallible>(Bytes::from(format!("data: {}\n\n", fc_done)));
            }
            if text_item_open {
                let text_done = serde_json::json!({
                    "type": "response.output_text.done",
                    "output_index": output_index - 1,
                    "content_index": 0,
                    "text": accumulated_text,
                });
                let item_done = serde_json::json!({
                    "type": "response.output_item.done",
                    "output_index": output_index - 1,
                    "item": {
                        "type": "message",
                        "id": "msg_proxy",
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": accumulated_text}],
                        "status": "completed",
                    }
                });
                yield Ok::<_, std::convert::Infallible>(Bytes::from(format!("data: {}\n\ndata: {}\n\n", text_done, item_done)));
            }

            let elapsed = start.elapsed().as_millis() as i64;
            let pt = total_prompt as i64;
            let ct = total_completion as i64;
            let cache_t = total_cached as i64;

            let _ = log_request_entry(
                &request_id,
                &client_format,
                &route.provider_name,
                &route.target_format,
                &target_model,
                true,
                elapsed,
                None,
                pt,
                ct,
                cache_t,
                ttft_ms,
            )
            .await;

            let _ = UsageTracker::record(
                &target_model,
                &route.provider_name,
                pt,
                ct,
            )
            .await;
        };

        let body_stream = axum::body::Body::from_stream(sse_stream);

        Response::builder()
            .status(status)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body_stream)
            .unwrap()
    }
}

fn get_parser(format: &ClientFormat) -> Box<dyn FormatParser> {
    match format {
        ClientFormat::Completions => Box::new(CompletionsParser),
        ClientFormat::Responses => Box::new(ResponsesParser),
        ClientFormat::Anthropic => Box::new(AnthropicParser),
        ClientFormat::Gemini => Box::new(GeminiParser),
    }
}

fn get_generator(format: &ClientFormat) -> Box<dyn FormatGenerator> {
    match format {
        ClientFormat::Completions => Box::new(CompletionsGenerator),
        ClientFormat::Responses => Box::new(ResponsesGenerator),
        ClientFormat::Anthropic => Box::new(AnthropicGenerator),
        ClientFormat::Gemini => Box::new(GeminiGenerator),
    }
}

use serde::Serialize;
use serde_json::json;

pub async fn handle_list_models() -> Response {
    let models = match query_model_routes().await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    let data: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({
                "id": m.model_name,
                "object": "model",
                "created": 0,
                "owned_by": m.provider_name,
            })
        })
        .collect();

    let body = json!({
        "object": "list",
        "data": data,
    });

    axum::Json(body).into_response()
}

pub async fn handle_get_model(Path(model): Path<String>) -> Response {
    let models = match query_model_routes().await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    let found = models.iter().find(|m| m.model_name == model);

    match found {
        Some(m) => {
            let body = json!({
                "id": m.model_name,
                "object": "model",
                "created": 0,
                "owned_by": m.provider_name,
            });
            axum::Json(body).into_response()
        }
        None => ProxyError::ModelNotFound(format!("model '{}' not found", model)).into_response(),
    }
}

pub async fn handle_gemini_list_models() -> Response {
    let models = match query_model_routes().await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    let gemini_models: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({
                "name": format!("models/{}", m.model_name),
                "displayName": m.model_name,
                "supportedGenerationMethods": ["generateContent", "streamGenerateContent"],
            })
        })
        .collect();

    let body = json!({
        "models": gemini_models,
    });

    axum::Json(body).into_response()
}

pub async fn handle_gemini_get_model(Path(model): Path<String>) -> Response {
    let models = match query_model_routes().await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    let model_name = model.split(':').next().unwrap_or(&model);

    let found = models.iter().find(|m| m.model_name == model_name);

    match found {
        Some(m) => {
            let body = json!({
                "name": format!("models/{}", m.model_name),
                "displayName": m.model_name,
                "supportedGenerationMethods": ["generateContent", "streamGenerateContent"],
            });
            axum::Json(body).into_response()
        }
        None => ProxyError::ModelNotFound(format!("model '{}' not found", model_name)).into_response(),
    }
}

#[derive(Serialize)]
struct ModelRouteInfo {
    model_name: String,
    provider_name: String,
    target_model: Option<String>,
    format: String,
}

async fn query_model_routes() -> Result<Vec<ModelRouteInfo>, ProxyError> {
    let pool = crate::db::get_pool().await;

    let rows = sqlx::query_as::<_, (String, String, Option<String>, String)>(
        "SELECT pm.model_name, p.name, pm.target_model, p.format \
         FROM provider_models pm \
         JOIN providers p ON pm.provider_id = p.id \
         WHERE pm.enabled = 1 \
         ORDER BY pm.model_name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ProxyError::Database(e))?;

    Ok(rows
        .into_iter()
        .map(|(model_name, provider_name, target_model, format)| {
            ModelRouteInfo {
                model_name,
                provider_name,
                target_model,
                format,
            }
        })
        .collect())
}

fn extract_headers(header_map: &axum::http::HeaderMap, headers: &mut HashMap<String, String>) {
    let skip = [
        "content-length",
        "content-type",
        "host",
        "transfer-encoding",
        "connection",
        "authorization",
    ];
    for (name, value) in header_map.iter() {
        let key = name.as_str().to_lowercase();
        if skip.contains(&key.as_str()) {
            continue;
        }
        if let Ok(v) = value.to_str() {
            headers.insert(key, v.to_string());
        }
    }
}

async fn log_request_entry(
    request_id: &str,
    client_format: &ClientFormat,
    provider_name: &str,
    provider_format: &ClientFormat,
    model: &str,
    stream: bool,
    duration_ms: i64,
    error_message: Option<&str>,
    prompt_tokens: i64,
    completion_tokens: i64,
    cached_tokens: i64,
    ttft_ms: Option<i64>,
) -> Result<(), ProxyError> {
    log_request(
        request_id,
        &format!("{:?}", client_format).to_lowercase(),
        provider_name,
        &format!("{:?}", provider_format).to_lowercase(),
        model,
        stream,
        duration_ms,
        error_message,
        prompt_tokens,
        completion_tokens,
        cached_tokens,
        ttft_ms,
    )
    .await
}
