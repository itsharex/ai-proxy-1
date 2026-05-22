use serde_json::Value;

use crate::converter::ir::{
    IrContentPart, IrMessage, IrRequest, IrResponse, IrRole, IrStreamChunk, IrTool,
    IrToolCall, IrToolCallDelta, IrUsage,
};
use crate::converter::FormatParser;
use crate::error::ProxyError;

pub struct AnthropicParser;

impl FormatParser for AnthropicParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"]
            .as_str()
            .ok_or_else(|| ProxyError::Parse("missing 'model' field".into()))?
            .to_string();

        let mut messages = Vec::new();

        if let Some(system) = body["system"].as_str() {
            if !system.is_empty() {
                messages.push(IrMessage {
                    role: IrRole::System,
                    content: vec![IrContentPart::Text {
                        text: system.to_string(),
                    }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        } else if let Some(system_arr) = body["system"].as_array() {
            let mut system_texts = Vec::new();
            for part in system_arr {
                if part["type"].as_str() == Some("text") {
                    if let Some(text) = part["text"].as_str() {
                        system_texts.push(text.to_string());
                    }
                }
            }
            if !system_texts.is_empty() {
                messages.push(IrMessage {
                    role: IrRole::System,
                    content: system_texts
                        .into_iter()
                        .map(|text| IrContentPart::Text { text })
                        .collect(),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        let msg_array = body["messages"]
            .as_array()
            .ok_or_else(|| ProxyError::Parse("missing 'messages' field".into()))?;

        for msg in msg_array {
            messages.push(parse_anthropic_message(msg)?);
        }

        let tools = body["tools"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let name = t["name"].as_str()?.to_string();
                    Some(IrTool {
                        name,
                        description: t["description"].as_str().map(String::from),
                        input_schema: t
                            .get("input_schema")
                            .cloned()
                            .unwrap_or(Value::Object(serde_json::Map::new())),
                    })
                })
                .collect::<Vec<_>>()
        });

        let temperature = body["temperature"].as_f64().map(|v| v as f32);
        let top_p = body["top_p"].as_f64().map(|v| v as f32);
        let max_tokens = body["max_tokens"].as_u64().map(|v| v as u32);
        let stream = body["stream"].as_bool().unwrap_or(false);

        let stop_sequences = body["stop_sequences"].as_array().map(|arr| {
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

        let event: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("failed to parse SSE chunk: {}", e)))?;

        let event_type = event["type"].as_str().unwrap_or("");

        match event_type {
            "content_block_delta" => {
                let delta = &event["delta"];
                let delta_type = delta["type"].as_str().unwrap_or("");

                match delta_type {
                    "text_delta" => {
                        let text = delta["text"].as_str().unwrap_or("");
                        Ok(Some(IrStreamChunk {
                            id: None,
                            model: None,
                            delta_content: Some(text.to_string()),
                            delta_tool_calls: None,
                            finish_reason: None,
                            usage: None,
                        }))
                    }
                    "input_json_delta" => {
                        let partial_json = delta["partial_json"].as_str().unwrap_or("");
                        let index = event["index"].as_u64().unwrap_or(0) as u32;
                        Ok(Some(IrStreamChunk {
                            id: None,
                            model: None,
                            delta_content: None,
                            delta_tool_calls: Some(vec![IrToolCallDelta {
                                index,
                                id: None,
                                name: None,
                                arguments: Some(partial_json.to_string()),
                            }]),
                            finish_reason: None,
                            usage: None,
                        }))
                    }
                    _ => Ok(None),
                }
            }
            "content_block_start" => {
                let content_block = &event["content_block"];
                let block_type = content_block["type"].as_str().unwrap_or("");

                if block_type == "tool_use" {
                    let id = content_block["id"].as_str().unwrap_or("").to_string();
                    let name = content_block["name"].as_str().unwrap_or("").to_string();
                    let index = event["index"].as_u64().unwrap_or(0) as u32;

                    return Ok(Some(IrStreamChunk {
                        id: None,
                        model: None,
                        delta_content: None,
                        delta_tool_calls: Some(vec![IrToolCallDelta {
                            index,
                            id: Some(id),
                            name: Some(name),
                            arguments: None,
                        }]),
                        finish_reason: None,
                        usage: None,
                    }));
                }

                Ok(None)
            }
            "message_delta" => {
                let delta = &event["delta"];
                let stop_reason = delta["stop_reason"].as_str().map(String::from);

                let usage = event.get("usage").and_then(|u| {
                    Some(IrUsage {
                        prompt_tokens: 0,
                        completion_tokens: u["output_tokens"].as_u64()? as u32,
                        total_tokens: u["output_tokens"].as_u64()? as u32,
                    })
                });

                Ok(Some(IrStreamChunk {
                    id: None,
                    model: None,
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason: stop_reason,
                    usage,
                }))
            }
            "message_start" => {
                let message = &event["message"];
                let id = message["id"].as_str().map(String::from);
                let model = message["model"].as_str().map(String::from);

                let usage = message.get("usage").and_then(|u| {
                    Some(IrUsage {
                        prompt_tokens: u["input_tokens"].as_u64()? as u32,
                        completion_tokens: 0,
                        total_tokens: u["input_tokens"].as_u64()? as u32,
                    })
                });

                Ok(Some(IrStreamChunk {
                    id,
                    model,
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason: None,
                    usage,
                }))
            }
            "message_stop" => Ok(Some(IrStreamChunk {
                id: None,
                model: None,
                delta_content: None,
                delta_tool_calls: None,
                finish_reason: Some("stop".to_string()),
                usage: None,
            })),
            _ => Ok(None),
        }
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let id = body["id"].as_str().map(String::from);
        let model = body["model"].as_str().map(String::from);

        let stop_reason = body["stop_reason"].as_str().map(String::from);

        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();

        if let Some(content) = body["content"].as_array() {
            for block in content {
                let block_type = block["type"].as_str().unwrap_or("");

                match block_type {
                    "text" => {
                        if let Some(text) = block["text"].as_str() {
                            content_parts.push(IrContentPart::Text {
                                text: text.to_string(),
                            });
                        }
                    }
                    "tool_use" => {
                        let call_id = block["id"].as_str().unwrap_or("").to_string();
                        let name = block["name"].as_str().unwrap_or("").to_string();
                        let input = block["input"].clone();

                        tool_calls.push(IrToolCall {
                            id: call_id,
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        });
                    }
                    "image" => {
                        let source = &block["source"];
                        let media_type = source["media_type"].as_str().map(String::from);
                        let data = source["data"].as_str().map(String::from);

                        content_parts.push(IrContentPart::Image {
                            url: None,
                            data,
                            media_type,
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

        let usage = body
            .get("usage")
            .map(|u| IrUsage {
                prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32
                    + u["output_tokens"].as_u64().unwrap_or(0) as u32,
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
            finish_reason: stop_reason,
            usage,
        })
    }
}

fn parse_anthropic_message(msg: &Value) -> Result<IrMessage, ProxyError> {
    let role_str = msg["role"]
        .as_str()
        .ok_or_else(|| ProxyError::Parse("message missing 'role'".into()))?;

    let role = match role_str {
        "user" => IrRole::User,
        "assistant" => IrRole::Assistant,
        other => {
            return Err(ProxyError::Parse(format!("unknown role: {}", other)));
        }
    };

    let mut content_parts = Vec::new();
    let mut tool_calls = Vec::new();

    if let Some(content) = msg.get("content") {
        if let Some(text) = content.as_str() {
            if !text.is_empty() {
                content_parts.push(IrContentPart::Text {
                    text: text.to_string(),
                });
            }
        } else if let Some(arr) = content.as_array() {
            for part in arr {
                let part_type = part["type"].as_str().unwrap_or("text");

                match part_type {
                    "text" => {
                        if let Some(text) = part["text"].as_str() {
                            content_parts.push(IrContentPart::Text {
                                text: text.to_string(),
                            });
                        }
                    }
                    "image" => {
                        let source = &part["source"];
                        let source_type = source["type"].as_str().unwrap_or("");

                        match source_type {
                            "base64" => {
                                content_parts.push(IrContentPart::Image {
                                    url: None,
                                    data: source["data"].as_str().map(String::from),
                                    media_type: source["media_type"].as_str().map(String::from),
                                });
                            }
                            "url" => {
                                content_parts.push(IrContentPart::Image {
                                    url: source["url"].as_str().map(String::from),
                                    data: None,
                                    media_type: None,
                                });
                            }
                            _ => {}
                        }
                    }
                    "tool_use" => {
                        let call_id = part["id"].as_str().unwrap_or("").to_string();
                        let name = part["name"].as_str().unwrap_or("").to_string();
                        let input = part["input"].clone();

                        tool_calls.push(IrToolCall {
                            id: call_id,
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        });
                    }
                    "tool_result" => {
                        let tool_use_id = part["tool_use_id"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        let result_content = if let Some(text) = part["content"].as_str() {
                            text.to_string()
                        } else if let Some(arr) = part["content"].as_array() {
                            arr.iter()
                                .filter_map(|p| {
                                    if p["type"].as_str() == Some("text") {
                                        p["text"].as_str().map(String::from)
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("")
                        } else {
                            String::new()
                        };

                        content_parts.push(IrContentPart::ToolResult {
                            tool_use_id,
                            content: result_content,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(IrMessage {
        role,
        content: content_parts,
        name: None,
        tool_call_id: None,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
    })
}
