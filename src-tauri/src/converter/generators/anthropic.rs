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
                IrRole::System => {
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
            body["tool_choice"] = tool_choice.clone();
        }

        if let Some(temperature) = ir.temperature {
            body["temperature"] = json!(temperature);
        }

        if let Some(top_p) = ir.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(stop) = &ir.stop_sequences {
            body["stop_sequences"] = json!(stop);
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        if let Some(_reason) = &chunk.finish_reason {
            let stop_event = json!({
                "type": "message_stop",
            });
            return format!("event: message_stop\ndata: {}\n\n", stop_event);
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

        if content.is_empty() {
            content.push(json!({
                "type": "text",
                "text": "",
            }));
        }

        let stop_reason = match ir.finish_reason.as_deref() {
            Some("stop") | Some("end_turn") => "end_turn",
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
            IrContentPart::ToolResult { tool_use_id, content } => {
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
