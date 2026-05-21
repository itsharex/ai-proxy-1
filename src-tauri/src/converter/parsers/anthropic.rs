use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct AnthropicParser;

impl FormatParser for AnthropicParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"]
            .as_str()
            .ok_or_else(|| ProxyError::Parse("missing model".into()))?
            .to_string();

        let mut messages: Vec<IrMessage> = Vec::new();

        if let Some(system) = body.get("system") {
            let system_text = if system.is_string() {
                system.as_str().unwrap().to_string()
            } else if system.is_array() {
                system
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|s| s["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                String::new()
            };
            if !system_text.is_empty() {
                messages.push(IrMessage {
                    role: IrRole::System,
                    content: vec![IrContentPart::Text { text: system_text }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        if let Some(msg_array) = body["messages"].as_array() {
            for m in msg_array {
                let role = match m["role"].as_str().unwrap_or("user") {
                    "user" => IrRole::User,
                    "assistant" => IrRole::Assistant,
                    _ => IrRole::User,
                };

                let content = if m["content"].is_string() {
                    vec![IrContentPart::Text {
                        text: m["content"].as_str().unwrap().to_string(),
                    }]
                } else if m["content"].is_array() {
                    m["content"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|part| match part["type"].as_str() {
                            Some("text") => IrContentPart::Text {
                                text: part["text"].as_str().unwrap_or("").to_string(),
                            },
                            Some("image") => IrContentPart::Image {
                                url: None,
                                data: part["source"]
                                    .get("data")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                media_type: part["source"]
                                    .get("media_type")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                            },
                            Some("tool_use") => IrContentPart::ToolUse {
                                id: part["id"].as_str().unwrap_or("").to_string(),
                                name: part["name"].as_str().unwrap_or("").to_string(),
                                input: part["input"].clone(),
                            },
                            Some("tool_result") => IrContentPart::ToolResult {
                                tool_use_id: part["tool_use_id"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                                content: part["content"].as_str().unwrap_or("").to_string(),
                            },
                            _ => IrContentPart::Text {
                                text: String::new(),
                            },
                        })
                        .collect()
                } else {
                    vec![]
                };

                let tool_calls = m["content"].as_array().map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| {
                            if p["type"] == "tool_use" {
                                Some(IrToolCall {
                                    id: p["id"].as_str()?.to_string(),
                                    name: p["name"].as_str()?.to_string(),
                                    arguments: serde_json::to_string(&p["input"]).unwrap_or_default(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                });

                let tool_calls =
                    if tool_calls.as_ref().map_or(false, |v| v.is_empty()) {
                        None
                    } else {
                        tool_calls
                    };

                messages.push(IrMessage {
                    role,
                    content,
                    name: None,
                    tool_call_id: None,
                    tool_calls,
                });
            }
        }

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|tool| {
                        let name = tool["name"].as_str()?;
                        Some(IrTool {
                            name: name.to_string(),
                            description: tool
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            input_schema: tool["input_schema"].clone(),
                        })
                    })
                    .collect()
            })
        });

        Ok(IrRequest {
            model,
            messages,
            tools,
            tool_choice: body.get("tool_choice").cloned(),
            temperature: body
                .get("temperature")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32),
            top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32),
            max_tokens: body
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            stream: body
                .get("stream")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            stop_sequences: body
                .get("stop_sequences")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                }),
            response_format: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data.is_empty() {
            return Ok(None);
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let event_type = chunk["type"].as_str().unwrap_or("");

        match event_type {
            "content_block_delta" => {
                let delta = &chunk["delta"];
                let delta_content = delta["text"].as_str().map(String::from);
                let delta_tool_calls =
                    if delta["type"] == "input_json_delta" {
                        Some(vec![IrToolCallDelta {
                            index: chunk["index"].as_u64().unwrap_or(0) as u32,
                            id: None,
                            name: None,
                            arguments: delta["partial_json"]
                                .as_str()
                                .map(String::from),
                        }])
                    } else {
                        None
                    };

                Ok(Some(IrStreamChunk {
                    id: None,
                    model: None,
                    delta_content,
                    delta_tool_calls,
                    finish_reason: None,
                    usage: None,
                }))
            }
            "message_delta" => {
                let delta = &chunk["delta"];
                Ok(Some(IrStreamChunk {
                    id: None,
                    model: None,
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason: delta["stop_reason"].as_str().map(String::from),
                    usage: chunk.get("usage").map(|u| IrUsage {
                        prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                        completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                        total_tokens: (u["input_tokens"].as_u64().unwrap_or(0)
                            + u["output_tokens"].as_u64().unwrap_or(0))
                            as u32,
                    }),
                }))
            }
            "message_stop" => Ok(Some(IrStreamChunk {
                id: None,
                model: None,
                delta_content: None,
                delta_tool_calls: None,
                finish_reason: Some("end_turn".into()),
                usage: None,
            })),
            _ => Ok(None),
        }
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let content_text = body["content"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        Ok(IrResponse {
            id: body["id"].as_str().map(String::from),
            model: body["model"].as_str().map(String::from),
            message: IrMessage {
                role: IrRole::Assistant,
                content: vec![IrContentPart::Text { text: content_text }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason: body["stop_reason"].as_str().map(String::from),
            usage: body
                .get("usage")
                .map(|u| IrUsage {
                    prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                    completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                    total_tokens: (u["input_tokens"].as_u64().unwrap_or(0)
                        + u["output_tokens"].as_u64().unwrap_or(0))
                        as u32,
                })
                .unwrap_or(IrUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                }),
        })
    }
}
