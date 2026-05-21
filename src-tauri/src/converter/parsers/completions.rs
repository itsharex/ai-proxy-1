use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct CompletionsParser;

impl FormatParser for CompletionsParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"]
            .as_str()
            .ok_or_else(|| ProxyError::Parse("missing model".into()))?
            .to_string();

        let messages: Vec<Value> = body["messages"]
            .as_array()
            .ok_or_else(|| ProxyError::Parse("missing messages".into()))?
            .clone();

        let ir_messages: Result<Vec<IrMessage>, ProxyError> = messages
            .iter()
            .map(|m| {
                let role_str = m["role"].as_str().unwrap_or("user");
                let role = match role_str {
                    "system" => IrRole::System,
                    "user" => IrRole::User,
                    "assistant" => IrRole::Assistant,
                    "tool" => IrRole::Tool,
                    _ => {
                        return Err(ProxyError::Parse(format!(
                            "unknown role: {}",
                            role_str
                        )))
                    }
                };

                let content = match m["content"].is_string() {
                    true => vec![IrContentPart::Text {
                        text: m["content"].as_str().unwrap().to_string(),
                    }],
                    false if m["content"].is_array() => {
                        let parts: Result<Vec<IrContentPart>, ProxyError> =
                            m["content"].as_array().unwrap().iter().map(|part| {
                                match part["type"].as_str() {
                                    Some("text") => Ok(IrContentPart::Text {
                                        text: part
                                            .get("text")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    }),
                                    Some("image_url") => Ok(IrContentPart::Image {
                                        url: part["image_url"]
                                            .get("url")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                        data: None,
                                        media_type: None,
                                    }),
                                    other => Err(ProxyError::Parse(format!(
                                        "unknown content type: {:?}",
                                        other
                                    ))),
                                }
                            }).collect();
                        parts?
                    }
                    _ => vec![IrContentPart::Text {
                        text: String::new(),
                    }],
                };

                let tool_calls = m.get("tool_calls").and_then(|tc| {
                    tc.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|tc| {
                                Some(IrToolCall {
                                    id: tc.get("id")?.as_str()?.to_string(),
                                    name: tc
                                        .get("function")?
                                        .get("name")?
                                        .as_str()?
                                        .to_string(),
                                    arguments: tc
                                        .get("function")?
                                        .get("arguments")?
                                        .as_str()?
                                        .to_string(),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                });

                let tool_calls =
                    if tool_calls.as_ref().map_or(false, |v| v.is_empty()) {
                        None
                    } else {
                        tool_calls
                    };

                Ok(IrMessage {
                    role,
                    content,
                    name: None,
                    tool_call_id: m
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    tool_calls,
                })
            })
            .collect();

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|tool| {
                        Some(IrTool {
                            name: tool
                                .get("function")?
                                .get("name")?
                                .as_str()?
                                .to_string(),
                            description: tool
                                .get("function")
                                .and_then(|f| f.get("description"))
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            input_schema: tool
                                .get("function")
                                .and_then(|f| f.get("parameters"))
                                .cloned()
                                .unwrap_or(Value::Null),
                        })
                    })
                    .collect()
            })
        });

        let mut metadata = std::collections::HashMap::new();
        if let Some(user) = body.get("user") {
            metadata.insert("user".into(), user.clone());
        }
        if let Some(seed) = body.get("seed") {
            metadata.insert("seed".into(), seed.clone());
        }

        Ok(IrRequest {
            model,
            messages: ir_messages?,
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
            stop_sequences: body.get("stop").and_then(|v| {
                if v.is_string() {
                    Some(vec![v.as_str()?.to_string()])
                } else {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .collect()
                    })
                }
            }),
            response_format: body.get("response_format").cloned(),
            metadata,
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data == "[DONE]" {
            return Ok(Some(IrStreamChunk {
                id: None,
                model: None,
                delta_content: None,
                delta_tool_calls: None,
                finish_reason: Some("stop".into()),
                usage: None,
            }));
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let choice = chunk["choices"].get(0);
        let delta = choice.and_then(|c| c.get("delta"));

        let delta_content = delta
            .and_then(|d| d["content"].as_str())
            .map(String::from);

        let delta_tool_calls =
            delta
                .and_then(|d| d["tool_calls"].as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|tc| {
                            Some(IrToolCallDelta {
                                index: tc.get("index")?.as_u64()? as u32,
                                id: tc
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                name: tc
                                    .get("function")?
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                arguments: tc
                                    .get("function")?
                                    .get("arguments")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                            })
                        })
                        .collect()
                });

        let finish_reason = choice
            .and_then(|c| c["finish_reason"].as_str())
            .map(String::from);

        Ok(Some(IrStreamChunk {
            id: chunk["id"].as_str().map(String::from),
            model: chunk["model"].as_str().map(String::from),
            delta_content,
            delta_tool_calls,
            finish_reason,
            usage: chunk.get("usage").map(|u| IrUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }),
        }))
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let choice = body["choices"]
            .get(0)
            .ok_or(ProxyError::Parse("no choices".into()))?;
        let msg = &choice["message"];

        let message = IrMessage {
            role: IrRole::Assistant,
            content: vec![IrContentPart::Text {
                text: msg["content"].as_str().unwrap_or("").to_string(),
            }],
            name: None,
            tool_call_id: None,
            tool_calls: msg.get("tool_calls").and_then(|tc| {
                tc.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|tc| {
                            Some(IrToolCall {
                                id: tc.get("id")?.as_str()?.to_string(),
                                name: tc
                                    .get("function")?
                                    .get("name")?
                                    .as_str()?
                                    .to_string(),
                                arguments: tc
                                    .get("function")?
                                    .get("arguments")?
                                    .as_str()?
                                    .to_string(),
                            })
                        })
                        .collect::<Vec<_>>()
                })
            }),
        };

        Ok(IrResponse {
            id: body["id"].as_str().map(String::from),
            model: body["model"].as_str().map(String::from),
            message,
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            usage: body
                .get("usage")
                .map(|u| IrUsage {
                    prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                    completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                    total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
                })
                .unwrap_or(IrUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                }),
        })
    }
}
