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

        let messages = body["messages"]
            .as_array()
            .ok_or_else(|| ProxyError::Parse("missing messages".into()))?;

        let ir_messages: Vec<IrMessage> = messages
            .iter()
            .map(|m| {
                let role = match m["role"].as_str().unwrap_or("user") {
                    "system" => IrRole::System,
                    "user" => IrRole::User,
                    "assistant" => IrRole::Assistant,
                    "tool" => IrRole::Tool,
                    r => return Err(ProxyError::Parse(format!("unknown role: {}", r))),
                };

                let content = if m["content"].is_string() {
                    vec![IrContentPart::Text {
                        text: m["content"].as_str().unwrap().to_string(),
                    }]
                } else if let Some(arr) = m["content"].as_array() {
                    arr.iter()
                        .filter_map(|p| match p["type"].as_str() {
                            Some("text") => Some(IrContentPart::Text {
                                text: p.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            }),
                            Some("image_url") => Some(IrContentPart::Image {
                                url: p["image_url"].get("url").and_then(|v| v.as_str()).map(String::from),
                                data: None,
                                media_type: None,
                            }),
                            _ => None,
                        })
                        .collect()
                } else {
                    vec![]
                };

                let tool_calls = m.get("tool_calls").and_then(|tc| {
                    let calls: Vec<IrToolCall> = tc
                        .as_array()?
                        .iter()
                        .filter_map(|tc| {
                            let func = tc.get("function")?;
                            Some(IrToolCall {
                                id: tc.get("id")?.as_str()?.to_string(),
                                name: func.get("name")?.as_str()?.to_string(),
                                arguments: func.get("arguments")?.as_str()?.to_string(),
                            })
                        })
                        .collect();
                    if calls.is_empty() { None } else { Some(calls) }
                });

                Ok(IrMessage {
                    role,
                    content,
                    name: None,
                    tool_call_id: m.get("tool_call_id").and_then(|v| v.as_str()).map(String::from),
                    tool_calls,
                })
            })
            .collect::<Result<_, _>>()?;

        let tools = body.get("tools").and_then(|t| {
            Some(
                t.as_array()?
                    .iter()
                    .filter_map(|tool| {
                        let func = tool.get("function")?;
                        Some(IrTool {
                            name: func.get("name")?.as_str()?.to_string(),
                            description: func.get("description").and_then(|v| v.as_str()).map(String::from),
                            input_schema: func.get("parameters").cloned().unwrap_or(Value::Null),
                        })
                    })
                    .collect::<Vec<_>>(),
            )
        });

        let mut metadata = std::collections::HashMap::new();
        if let Some(user) = body.get("user") {
            metadata.insert("user".into(), user.clone());
        }

        Ok(IrRequest {
            model,
            messages: ir_messages,
            tools,
            tool_choice: body.get("tool_choice").cloned(),
            temperature: body.get("temperature").and_then(|v| v.as_f64()).map(|v| v as f32),
            top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32),
            max_tokens: body.get("max_tokens").and_then(|v| v.as_u64()).map(|v| v as u32),
            stream: body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false),
            stop_sequences: body.get("stop").and_then(|v| {
                if v.is_string() {
                    Some(vec![v.as_str()?.to_string()])
                } else {
                    v.as_array().map(|arr| {
                        arr.iter().filter_map(|s| s.as_str().map(String::from)).collect()
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

        Ok(Some(IrStreamChunk {
            id: chunk["id"].as_str().map(String::from),
            model: chunk["model"].as_str().map(String::from),
            delta_content: delta.and_then(|d| d["content"].as_str()).map(String::from),
            delta_tool_calls: delta.and_then(|d| d["tool_calls"].as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        Some(IrToolCallDelta {
                            index: tc.get("index")?.as_u64()? as u32,
                            id: tc.get("id").and_then(|v| v.as_str()).map(String::from),
                            name: tc.get("function")?.get("name").and_then(|v| v.as_str()).map(String::from),
                            arguments: tc.get("function")?.get("arguments").and_then(|v| v.as_str()).map(String::from),
                        })
                    })
                    .collect()
            }),
            finish_reason: choice.and_then(|c| c["finish_reason"].as_str()).map(String::from),
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

        let tool_calls = msg.get("tool_calls").and_then(|tc| {
            let calls: Vec<IrToolCall> = tc.as_array()?.iter().filter_map(|tc| {
                let func = tc.get("function")?;
                Some(IrToolCall {
                    id: tc.get("id")?.as_str()?.to_string(),
                    name: func.get("name")?.as_str()?.to_string(),
                    arguments: func.get("arguments")?.as_str()?.to_string(),
                })
            }).collect();
            if calls.is_empty() { None } else { Some(calls) }
        });

        Ok(IrResponse {
            id: body["id"].as_str().map(String::from),
            model: body["model"].as_str().map(String::from),
            message: IrMessage {
                role: IrRole::Assistant,
                content: vec![IrContentPart::Text {
                    text: msg["content"].as_str().unwrap_or("").to_string(),
                }],
                name: None,
                tool_call_id: None,
                tool_calls,
            },
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            usage: body.get("usage").map(|u| IrUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }).unwrap_or(IrUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
