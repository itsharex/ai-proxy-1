use serde_json::Value;

use crate::converter::ir::{
    IrContentPart, IrMessage, IrRequest, IrResponse, IrRole, IrStreamChunk, IrTool,
    IrToolCall, IrToolCallDelta, IrUsage,
};
use crate::converter::FormatParser;
use crate::error::ProxyError;

pub struct ResponsesParser;

impl FormatParser for ResponsesParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"]
            .as_str()
            .ok_or_else(|| ProxyError::Parse("missing 'model' field".into()))?
            .to_string();

        let mut messages = Vec::new();

        if let Some(instructions) = body["instructions"].as_str() {
            if !instructions.is_empty() {
                messages.push(IrMessage {
                    role: IrRole::System,
                    content: vec![IrContentPart::Text {
                        text: instructions.to_string(),
                    }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        if let Some(input) = body.get("input") {
            if let Some(text) = input.as_str() {
                messages.push(IrMessage {
                    role: IrRole::User,
                    content: vec![IrContentPart::Text {
                        text: text.to_string(),
                    }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            } else if let Some(arr) = input.as_array() {
                for item in arr {
                    if let Some(msg) = parse_input_item(item)? {
                        messages.push(msg);
                    }
                }
            }
        }

        let tools = body["tools"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let tool_type = t["type"].as_str().unwrap_or("");
                    if tool_type != "function" {
                        return None;
                    }
                    let name = t["name"].as_str()?.to_string();
                    Some(IrTool {
                        name,
                        description: t["description"].as_str().map(String::from),
                        input_schema: t
                            .get("parameters")
                            .cloned()
                            .unwrap_or(Value::Object(serde_json::Map::new())),
                    })
                })
                .collect::<Vec<_>>()
        });

        let temperature = body["temperature"].as_f64().map(|v| v as f32);
        let top_p = body["top_p"].as_f64().map(|v| v as f32);
        let max_tokens = body["max_output_tokens"].as_u64().map(|v| v as u32);
        let stream = body["stream"].as_bool().unwrap_or(false);

        let stop_sequences = body["stop"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        });

        let tool_choice = body.get("tool_choice").cloned();
        let response_format = body.get("response_format").cloned();

        Ok(IrRequest {
            model,
            messages,
            tools,
            tool_choice,
            temperature,
            top_p,
            max_tokens,
            stream,
            stop_sequences,
            response_format,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Ok(None);
        }

        if !trimmed.starts_with("data: ") {
            return Ok(None);
        }

        let data = trimmed.strip_prefix("data: ").unwrap().trim();

        if data == "[DONE]" {
            return Ok(None);
        }

        let event: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("failed to parse SSE chunk: {}", e)))?;

        let event_type = event["type"].as_str().unwrap_or("");

        match event_type {
            "response.output_text.delta" => {
                let delta = event["delta"].as_str().unwrap_or("");
                Ok(Some(IrStreamChunk {
                    id: event["response_id"].as_str().map(String::from),
                    model: None,
                    delta_content: Some(delta.to_string()),
                    delta_tool_calls: None,
                    finish_reason: None,
                    usage: None,
                }))
            }
            "response.output_item.done" => {
                let item = &event["item"];
                let item_type = item["type"].as_str().unwrap_or("");

                if item_type == "function_call" {
                    let call_id = item["call_id"].as_str().unwrap_or("");
                    let name = item["name"].as_str().unwrap_or("");
                    let arguments = item["arguments"].as_str().unwrap_or("{}");

                    return Ok(Some(IrStreamChunk {
                        id: event["response_id"].as_str().map(String::from),
                        model: None,
                        delta_content: None,
                        delta_tool_calls: Some(vec![IrToolCallDelta {
                            index: 0,
                            id: Some(call_id.to_string()),
                            name: Some(name.to_string()),
                            arguments: Some(arguments.to_string()),
                        }]),
                        finish_reason: None,
                        usage: None,
                    }));
                }

                Ok(None)
            }
            "response.completed" => {
                let response = &event["response"];
                let usage = response.get("usage").and_then(|u| {
                    Some(IrUsage {
                        prompt_tokens: u["input_tokens"].as_u64()? as u32,
                        completion_tokens: u["output_tokens"].as_u64()? as u32,
                        total_tokens: u["input_tokens"].as_u64()? as u32
                            + u["output_tokens"].as_u64()? as u32,
                    })
                });

                let finish_reason = response["status"]
                    .as_str()
                    .filter(|s| *s != "in_progress")
                    .map(String::from)
                    .or(Some("completed".to_string()));

                Ok(Some(IrStreamChunk {
                    id: response["id"].as_str().map(String::from),
                    model: response["model"].as_str().map(String::from),
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason,
                    usage,
                }))
            }
            _ => Ok(None),
        }
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let id = body["id"].as_str().map(String::from);
        let model = body["model"].as_str().map(String::from);

        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();

        if let Some(outputs) = body["output"].as_array() {
            for output in outputs {
                let output_type = output["type"].as_str().unwrap_or("");

                match output_type {
                    "message" => {
                        if let Some(parts) = output["content"].as_array() {
                            for part in parts {
                                if part["type"].as_str() == Some("output_text") {
                                    if let Some(text) = part["text"].as_str() {
                                        content_parts.push(IrContentPart::Text {
                                            text: text.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "function_call" => {
                        let call_id = output["call_id"].as_str().unwrap_or("");
                        let name = output["name"].as_str().unwrap_or("");
                        let arguments = output["arguments"].as_str().unwrap_or("{}");

                        tool_calls.push(IrToolCall {
                            id: call_id.to_string(),
                            name: name.to_string(),
                            arguments: arguments.to_string(),
                        });
                    }
                    _ => {}
                }
            }
        }

        let message = IrMessage {
            role: IrRole::Assistant,
            content: content_parts,
            name: None,
            tool_call_id: None,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
        };

        let finish_reason = body["status"]
            .as_str()
            .filter(|s| *s != "in_progress")
            .map(String::from);

        let usage = body
            .get("usage")
            .and_then(|u| {
                Some(IrUsage {
                    prompt_tokens: u["input_tokens"].as_u64()? as u32,
                    completion_tokens: u["output_tokens"].as_u64()? as u32,
                    total_tokens: u["input_tokens"].as_u64()? as u32
                        + u["output_tokens"].as_u64()? as u32,
                })
            })
            .unwrap_or(IrUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            });

        Ok(IrResponse {
            id,
            model,
            message,
            finish_reason,
            usage,
        })
    }
}

fn parse_input_item(item: &Value) -> Result<Option<IrMessage>, ProxyError> {
    let role_str = item["role"].as_str().unwrap_or("");

    let role = match role_str {
        "user" => IrRole::User,
        "assistant" => IrRole::Assistant,
        "system" => IrRole::System,
        "" => {
            let item_type = item["type"].as_str().unwrap_or("message");
            if item_type == "message" {
                item["role"]
                    .as_str()
                    .and_then(|r| match r {
                        "user" => Some(IrRole::User),
                        "assistant" => Some(IrRole::Assistant),
                        "system" => Some(IrRole::System),
                        _ => None,
                    })
                    .ok_or_else(|| ProxyError::Parse("unknown role in input item".into()))?
            } else {
                return Ok(None);
            }
        }
        other => {
            return Err(ProxyError::Parse(format!(
                "unknown role in input: {}",
                other
            )));
        }
    };

    let mut content_parts = Vec::new();

    if let Some(text) = item.get("content").and_then(|c| c.as_str()) {
        if !text.is_empty() {
            content_parts.push(IrContentPart::Text {
                text: text.to_string(),
            });
        }
    }

    if content_parts.is_empty() {
        if let Some(arr) = item["content"].as_array() {
            for part in arr {
                if let Some(text) = part["text"].as_str() {
                    content_parts.push(IrContentPart::Text {
                        text: text.to_string(),
                    });
                }
            }
        }
    }

    if content_parts.is_empty() {
        if let Some(text) = item["input"].as_str() {
            content_parts.push(IrContentPart::Text {
                text: text.to_string(),
            });
        }
    }

    Ok(Some(IrMessage {
        role,
        content: content_parts,
        name: None,
        tool_call_id: None,
        tool_calls: None,
    }))
}
