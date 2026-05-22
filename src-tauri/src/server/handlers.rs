use std::collections::HashMap;

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::stream::StreamExt;
use reqwest::Client;
use serde_json::Value;
use tracing::error;

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
    handle_proxy(request, ClientFormat::Completions).await
}

pub async fn handle_responses(request: Request) -> Response {
    handle_proxy(request, ClientFormat::Responses).await
}

pub async fn handle_anthropic(request: Request) -> Response {
    handle_proxy(request, ClientFormat::Anthropic).await
}

pub async fn handle_gemini(request: Request) -> Response {
    handle_proxy(request, ClientFormat::Gemini).await
}

async fn handle_proxy(request: Request, client_format: ClientFormat) -> Response {
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
        Ok(r) => r,
        Err(e) => {
            return e.into_response();
        }
    };

    let mut extra_headers: HashMap<String, String> = HashMap::new();
    extract_headers(&parts.headers, &mut extra_headers);

    let path = parts.uri.path().to_string();

    if let Err(e) = InterceptorEngine::execute_pre_rules(&mut ir_request, &path, &mut extra_headers).await {
        error!("Interceptor error: {}", e);
    }

    let route = match ProviderManager::find_for_model(&ir_request.model).await {
        Ok(r) => r,
        Err(e) => {
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

    let target_body = match target_generator.generate_request(&ir_request) {
        Ok(b) => b,
        Err(e) => {
            return e.into_response();
        }
    };

    let url = format!("{}{}", route.base_url.trim_end_matches('/'), route.endpoint_path);

    let http_client = Client::new();
    let mut req_builder = http_client
        .post(&url)
        .json(&target_body)
        .header("Content-Type", "application/json");

    if route.auth_type == "bearer" {
        req_builder = req_builder.bearer_auth(&api_key);
    } else {
        req_builder = req_builder.header(&route.auth_header, &api_key);
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
            )
            .await;
            return err.into_response();
        }
    };

    let status = resp.status();
    let is_stream = ir_request.stream;

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

        let sse_stream = async_stream::stream! {
            let mut buffer = String::new();
            let mut total_prompt = 0u32;
            let mut total_completion = 0u32;
            let mut reader = stream;

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

                    match target_parser.parse_stream_chunk(trimmed) {
                        Ok(Some(ir_chunk)) => {
                            if let Some(usage) = &ir_chunk.usage {
                                total_prompt += usage.prompt_tokens;
                                total_completion += usage.completion_tokens;
                            }

                            let client_sse = generator.generate_stream_chunk(&ir_chunk);
                            if !client_sse.is_empty() {
                                yield Ok::<_, std::convert::Infallible>(Bytes::from(client_sse));
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            error!("Stream chunk parse error: {}", e);
                        }
                    }
                }
            }

            let elapsed = start.elapsed().as_millis() as i64;
            let pt = total_prompt as i64;
            let ct = total_completion as i64;

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

fn extract_headers(header_map: &axum::http::HeaderMap, headers: &mut HashMap<String, String>) {
    for (name, value) in header_map.iter() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.as_str().to_lowercase(), v.to_string());
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
    )
    .await
}
