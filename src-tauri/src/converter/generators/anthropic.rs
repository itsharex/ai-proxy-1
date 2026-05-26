use crate::converter::ir::{IrContentPart, IrRequest, IrResponse, IrRole, IrStreamChunk};
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct AnthropicGenerator;

impl FormatGenerator for AnthropicGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let mut system_text: Option<String> = None;
        let mut messages: Vec<Value> = Vec::new();

        for msg in &ir.messages {
            match msg.role {
                IrRole::System | IrRole::Developer => {
                    let text = extract_text_parts(&msg.content);
                    system_text = Some(text);
                }
                _ => {
                    let role_str = match msg.role {
                        IrRole::User => "user",
                        IrRole::Assistant => "assistant",
                        IrRole::Tool => "user",
                        _ => "user",
                    };

                    let mut message = json!({
                        "role": role_str,
                    });

                    let content = convert_anthropic_content(&msg.content, &msg.role);
                    message["content"] = content;

                    messages.push(message);
                }
            }
        }

        let mut body = json!({
            "model": ir.model,
            "messages": messages,
            "max_tokens": ir.max_tokens.unwrap_or(4096),
            "stream": ir.stream,
        });

        if let Some(system) = system_text {
            body["system"] = json!(system);
        }

        if let Some(tools) = &ir.tools {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    let mut tool = json!({
                        "name": t.name,
                        "input_schema": t.input_schema,
                    });
                    if let Some(desc) = &t.description {
                        tool["description"] = json!(desc);
                    }
                    tool
                })
                .collect();
            body["tools"] = json!(tool_defs);
        }

        if let Some(tool_choice) = &ir.tool_choice {
            let converted = match tool_choice {
                Value::String(s) => match s.as_str() {
                    "auto" => json!({"type": "auto"}),
                    "required" => json!({"type": "any"}),
                    "none" => json!({"type": "none"}),
                    _ => json!({"type": "auto"}),
                },
                Value::Object(map) => {
                    if let Some(t) = map.get("type").and_then(|v| v.as_str()) {
                        match t {
                            "function" => {
                                if let Some(name) = map.get("function")
                                    .and_then(|f| f.get("name"))
                                    .and_then(|n| n.as_str())
                                {
                                    json!({"type": "tool", "name": name})
                                } else {
                                    json!({"type": "auto"})
                                }
                            }
                            other => json!({"type": other}),
                        }
                    } else {
                        tool_choice.clone()
                    }
                }
                _ => json!({"type": "auto"}),
            };
            body["tool_choice"] = converted;
        }

        if let Some(temperature) = ir.temperature {
            body["temperature"] = json!(temperature);
        }

        if let Some(top_p) = ir.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(top_k) = ir.top_k {
            body["top_k"] = json!(top_k);
        }

        if let Some(stop) = &ir.stop_sequences {
            body["stop_sequences"] = json!(stop);
        }

        if let Some(thinking) = &ir.thinking {
            if thinking.enabled {
                body["thinking"] = json!({
                    "type": "enabled",
                    "budget_tokens": thinking.budget_tokens.unwrap_or(10000),
                });
                if ir.max_tokens.is_none() {
                    body["max_tokens"] = json!(16000);
                }
            }
        }

        if ir.extra.contains_key("has_cache_control") {
            // cache_control is per-element, not top-level
        }

        Ok(body)
    }

    fn generate_stream_start(&self, response_id: &str, model: &str) -> Option<String> {
        let event = json!({
            "type": "message_start",
            "message": {
                "id": response_id,
                "type": "message",
                "role": "assistant",
                "model": model,
                "content": [],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 0,
                    "output_tokens": 0,
                }
            }
        });
        Some(format!("event: message_start\ndata: {}\n\n", event))
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        if let Some(reason) = &chunk.finish_reason {
            let stop_reason = match reason.as_str() {
                "stop" | "end_turn" | "completed" => "end_turn",
                "tool_calls" | "tool_use" => "tool_use",
                r => r,
            };

            let usage = chunk.usage.as_ref().map(|u| json!({
                "output_tokens": u.completion_tokens,
            })).unwrap_or(json!({ "output_tokens": 0 }));

            let message_delta = json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": stop_reason,
                    "stop_sequence": null,
                },
                "usage": usage,
            });

            let message_stop = json!({
                "type": "message_stop",
            });

            return format!(
                "event: message_delta\ndata: {}\n\nevent: message_stop\ndata: {}\n\n",
                message_delta, message_stop
            );
        }

        if let Some(thinking) = &chunk.delta_thinking {
            let delta_event = json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {
                    "type": "thinking_delta",
                    "thinking": thinking,
                }
            });
            return format!("event: content_block_delta\ndata: {}\n\n", delta_event);
        }

        if let Some(content) = &chunk.delta_content {
            let delta_event = json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {
                    "type": "text_delta",
                    "text": content,
                }
            });
            return format!("event: content_block_delta\ndata: {}\n\n", delta_event);
        }

        if let Some(tool_calls) = &chunk.delta_tool_calls {
            if let Some(tc) = tool_calls.first() {
                if let Some(args) = &tc.arguments {
                    let delta_event = json!({
                        "type": "content_block_delta",
                        "index": tc.index,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": args,
                        }
                    });
                    return format!("event: content_block_delta\ndata: {}\n\n", delta_event);
                }
                if tc.id.is_some() {
                    let start_event = json!({
                        "type": "content_block_start",
                        "index": tc.index,
                        "content_block": {
                            "type": "tool_use",
                            "id": tc.id,
                            "name": tc.name,
                            "input": {},
                        }
                    });
                    return format!("event: content_block_start\ndata: {}\n\n", start_event);
                }
            }
        }

        String::new()
    }

    fn generate_stream_end(&self) -> Option<String> {
        None
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let id = ir.id.as_deref().unwrap_or("msg-proxy");

        let mut content: Vec<Value> = Vec::new();

        for part in &ir.message.content {
            match part {
                IrContentPart::Text { text } => {
                    content.push(json!({
                        "type": "text",
                        "text": text,
                    }));
                }
                IrContentPart::Thinking { text } => {
                    content.push(json!({
                        "type": "thinking",
                        "thinking": text,
                    }));
                }
                IrContentPart::ToolUse { id, name, input } => {
                    content.push(json!({
                        "type": "tool_use",
                        "id": id,
                        "name": name,
                        "input": input,
                    }));
                }
                _ => {}
            }
        }

        if content.is_empty() && ir.message.tool_calls.is_none() {
            content.push(json!({
                "type": "text",
                "text": "",
            }));
        }

        // Add tool_calls as tool_use content blocks
        if let Some(tool_calls) = &ir.message.tool_calls {
            for tc in tool_calls {
                let input: Value = serde_json::from_str(&tc.arguments).unwrap_or(Value::Object(serde_json::Map::new()));
                content.push(json!({
                    "type": "tool_use",
                    "id": tc.id,
                    "name": tc.name,
                    "input": input,
                }));
            }
        }

        let stop_reason = match ir.finish_reason.as_deref() {
            Some("stop") | Some("end_turn") | Some("completed") => "end_turn",
            Some("tool_calls") | Some("tool_use") => "tool_use",
            Some(r) => r,
            None => "end_turn",
        };

        Ok(json!({
            "id": id,
            "type": "message",
            "role": "assistant",
            "model": ir.model.as_deref().unwrap_or(""),
            "content": content,
            "stop_reason": stop_reason,
            "stop_sequence": null,
            "usage": {
                "input_tokens": ir.usage.prompt_tokens,
                "output_tokens": ir.usage.completion_tokens,
            }
        }))
    }
}

fn extract_text_parts(parts: &[IrContentPart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            IrContentPart::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn convert_anthropic_content(parts: &[IrContentPart], role: &IrRole) -> Value {
    if parts.len() == 1 {
        if let Some(IrContentPart::Text { text }) = parts.first() {
            return json!(text);
        }
    }

    let items: Vec<Value> = parts
        .iter()
        .map(|part| match part {
            IrContentPart::Text { text } => json!({
                "type": "text",
                "text": text,
            }),
            IrContentPart::Thinking { text } => json!({
                "type": "text",
                "text": text,
            }),
            IrContentPart::Image { url, data, media_type } => {
                let source = if let Some(image_url) = url {
                    json!({
                        "type": "url",
                        "url": image_url,
                    })
                } else if let Some(b64) = data {
                    let mt = media_type.as_deref().unwrap_or("image/png");
                    json!({
                        "type": "base64",
                        "media_type": mt,
                        "data": b64,
                    })
                } else {
                    json!({ "type": "url", "url": "" })
                };
                json!({
                    "type": "image",
                    "source": source,
                })
            }
            IrContentPart::ToolUse { id, name, input } => json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": input,
            }),
            IrContentPart::ToolResult { tool_use_id, content, .. } => {
                let _ = role;
                json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": content,
                })
            }
        })
        .collect();

    json!(items)
}
