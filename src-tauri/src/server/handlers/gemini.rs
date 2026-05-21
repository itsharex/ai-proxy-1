use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response, Sse};
use crate::converter::parsers::completions::CompletionsParser;
use crate::converter::parsers::gemini::GeminiParser;
use crate::converter::generators::gemini::GeminiGenerator;
use crate::converter::generators::completions::CompletionsGenerator;
use crate::converter::generators::responses::ResponsesGenerator;
use crate::converter::generators::anthropic::AnthropicGenerator;
use crate::converter::{FormatGenerator, FormatParser};
use crate::db::pool::get_pool;
use crate::error::ProxyError;
use crate::interceptor::InterceptorEngine;
use crate::key::rotation::{KeyRotation, Strategy};
use crate::key::store::KeyStore;
use crate::logging::LogStore;
use crate::provider::ProviderManager;
use crate::usage::UsageTracker;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Instant;

pub async fn handle_gemini(request: Request) -> Response {
    let start = Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return ProxyError::Parse(format!("Failed to read body: {}", e)).into_response();
        }
    };

    let mut body_value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            return ProxyError::Parse(format!("Invalid JSON: {}", e)).into_response();
        }
    };

    // Extract model from path: /v1beta/models/{model}
    let path = parts.uri.path().to_string();
    let model_from_path = path
        .split('/')
        .nth(3)
        .unwrap_or("unknown")
        .to_string();

    if body_value["model"].is_null() {
        body_value["model"] = serde_json::json!(model_from_path);
    }

    let parser = GeminiParser;
    let mut ir = match parser.parse_request(&body_value) {
        Ok(ir) => ir,
        Err(e) => return e.into_response(),
    };

    let original_model = ir.model.clone();
    let is_stream = ir.stream;

    let pool = get_pool().await;

    let mut headers_map = extract_headers(&parts.headers);
    if let Err(e) = InterceptorEngine::execute_pre_rules(&mut ir, &path, &mut headers_map).await {
        tracing::warn!("Interceptor error: {}", e);
    }

    let route = match ProviderManager::find_for_model(pool, &ir.model).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return ProxyError::ModelNotFound(ir.model.clone()).into_response();
        }
        Err(e) => return e.into_response(),
    };

    let (_key_id, encrypted_key, nonce) =
        match KeyRotation::get_next_key(&route.provider_name, Strategy::LeastUsed)
            .await
        {
            Ok(k) => k,
            Err(e) => return e.into_response(),
        };

    let master_key = KeyStore::derive_key();
    let key_store = KeyStore::new(&master_key);
    let api_key = match key_store.decrypt(&encrypted_key, &nonce) {
        Ok(k) => k,
        Err(e) => {
            return ProxyError::KeyManagement(format!("Key decryption failed: {}", e))
                .into_response();
        }
    };

    ir.model = route.target_model.clone();

    let target_body = match generate_target_request(&route.target_format, &ir) {
        Ok(b) => b,
        Err(e) => return e.into_response(),
    };

    let upstream_url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        route.base_url.trim_end_matches('/'),
        route.target_model,
        api_key
    );

    let client = reqwest::Client::new();
    let req_builder = client
        .post(&upstream_url)
        .json(&target_body)
        .header("Content-Type", "application/json");

    let resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = LogStore::insert(
                &request_id,
                "gemini",
                &route.provider_name,
                &route.target_format,
                &original_model,
                is_stream,
                None,
                Some(start.elapsed().as_millis() as i64),
            )
            .await;
            return ProxyError::Network(format!("Upstream request failed: {}", e)).into_response();
        }
    };

    let status = resp.status();
    let duration_ms = start.elapsed().as_millis() as i64;

    let _ = LogStore::insert(
        &request_id,
        "gemini",
        &route.provider_name,
        &route.target_format,
        &original_model,
        is_stream,
        Some(status.as_u16()),
        Some(duration_ms),
    )
    .await;

    if is_stream {
        handle_streaming_response(resp, &route.target_format, status).await
    } else {
        handle_non_streaming_response(resp, &route.target_format, status).await
    }
}

pub async fn handle_gemini_action(request: Request) -> Response {
    handle_gemini(request).await
}

fn generate_target_request(
    target_format: &str,
    ir: &crate::converter::ir::IrRequest,
) -> Result<serde_json::Value, ProxyError> {
    match target_format {
        "completions" => CompletionsGenerator.generate_request(ir),
        "responses" => ResponsesGenerator.generate_request(ir),
        "anthropic" => AnthropicGenerator.generate_request(ir),
        "gemini" => GeminiGenerator.generate_request(ir),
        _ => Err(ProxyError::Config(format!(
            "Unknown target format: {}",
            target_format
        ))),
    }
}

async fn handle_non_streaming_response(
    resp: reqwest::Response,
    target_format: &str,
    status: StatusCode,
) -> Response {
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            return ProxyError::Network(format!("Failed to read upstream response: {}", e))
                .into_response();
        }
    };

    let body_value: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            return (status, body).into_response();
        }
    };

    let ir_response = match parse_target_response(target_format, &body_value) {
        Ok(r) => r,
        Err(_) => {
            return (status, axum::Json(body_value)).into_response();
        }
    };

    let _ = UsageTracker::record(
        ir_response.model.as_deref().unwrap_or("unknown"),
        "upstream",
        &ir_response.usage,
    )
    .await;

    let generator = GeminiGenerator;
    match generator.generate_response(&ir_response) {
        Ok(client_response) => (status, axum::Json(client_response)).into_response(),
        Err(_) => (status, axum::Json(body_value)).into_response(),
    }
}

async fn handle_streaming_response(
    resp: reqwest::Response,
    target_format: &str,
    status: StatusCode,
) -> Response {
    use async_stream::stream;

    let parser: Option<Box<dyn FormatParser + Send>> = match target_format {
        "completions" => Some(Box::new(CompletionsParser)),
        "responses" => Some(Box::new(
            crate::converter::parsers::responses::ResponsesParser,
        )),
        "anthropic" => Some(Box::new(
            crate::converter::parsers::anthropic::AnthropicParser,
        )),
        "gemini" => Some(Box::new(GeminiParser)),
        _ => None,
    };

    let generator = GeminiGenerator;

    let stream = stream! {
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let text = match std::str::from_utf8(&chunk) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    buffer.push_str(text);

                    while let Some(pos) = buffer.find("\n\n") {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        for line in line.lines() {
                            let line = line.trim().to_string();
                            if line.is_empty() {
                                continue;
                            }

                            if let Some(ref parser) = parser {
                                match parser.parse_stream_chunk(&line) {
                                    Ok(Some(ir_chunk)) => {
                                        let client_chunk = generator.generate_stream_chunk(&ir_chunk);
                                        yield Ok::<_, Infallible>(client_chunk);
                                    }
                                    Ok(None) => {}
                                    Err(e) => {
                                        tracing::warn!("Stream chunk parse error: {}", e);
                                    }
                                }
                            } else {
                                yield Ok(line);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Stream error: {}", e);
                    break;
                }
            }
        }
    };

    let sse = Sse::new(stream.map(|item| {
        item.map(|data| {
            use axum::response::sse::Event;
            Event::default().data(data)
        })
    }));

    (
        status,
        [
            ("Content-Type", "text/event-stream"),
            ("Cache-Control", "no-cache"),
            ("Connection", "keep-alive"),
        ],
        sse,
    )
        .into_response()
}

fn parse_target_response(
    target_format: &str,
    body: &serde_json::Value,
) -> Result<crate::converter::ir::IrResponse, ProxyError> {
    match target_format {
        "completions" => CompletionsParser.parse_response(body),
        "responses" => {
            crate::converter::parsers::responses::ResponsesParser.parse_response(body)
        }
        "anthropic" => {
            crate::converter::parsers::anthropic::AnthropicParser.parse_response(body)
        }
        "gemini" => GeminiParser.parse_response(body),
        _ => Err(ProxyError::Config(format!(
            "Unknown target format: {}",
            target_format
        ))),
    }
}

fn extract_headers(headers: &HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            map.insert(name.to_string(), v.to_string());
        }
    }
    map
}
