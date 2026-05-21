use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct CompletionsGenerator;

impl FormatGenerator for CompletionsGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let messages: Vec<Value> = ir
            .messages
            .iter()
            .map(|m| {
                let content = if m.content.len() == 1 {
                    match &m.content[0] {
                        IrContentPart::Text { text } => json!(text),
                        _ => json!(""),
                    }
                } else {
                    let parts: Vec<Value> = m
                        .content
                        .iter()
                        .map(|p| match p {
                            IrContentPart::Text { text } => {
                                json!({"type": "text", "text": text})
                            }
                            IrContentPart::Image {
                                url,
                                data,
                                media_type,
                            } => {
                                let mut img = serde_json::Map::new();
                                if let Some(url) = url {
                                    img.insert("url".into(), json!(url));
                                }
                                if let Some(data) = data {
                                    img.insert("data".into(), json!(data));
                                }
                                if let Some(mt) = media_type {
                                    img.insert("media_type".into(), json!(mt));
                                }
                                json!({"type": "image_url", "image_url": img})
                            }
                            _ => json!({"type": "text", "text": ""}),
                        })
                        .collect();
                    json!(parts)
                };

                let mut msg = json!({
                    "role": match m.role {
                        IrRole::System => "system",
                        IrRole::User => "user",
                        IrRole::Assistant => "assistant",
                        IrRole::Tool => "tool",
                    },
                    "content": content,
                });

                if let Some(name) = &m.name {
                    msg["name"] = json!(name);
                }
                if let Some(tool_call_id) = &m.tool_call_id {
                    msg["tool_call_id"] = json!(tool_call_id);
                }
                if let Some(tool_calls) = &m.tool_calls {
                    msg["tool_calls"] = json!(tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments,
                                }
                            })
                        })
                        .collect::<Vec<_>>());
                }
                msg
            })
            .collect();

        let mut body = json!({
            "model": ir.model,
            "messages": messages,
            "stream": ir.stream,
        });

        if let Some(tools) = &ir.tools {
            body["tools"] = json!(tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect::<Vec<_>>());
        }
        if let Some(tc) = &ir.tool_choice {
            body["tool_choice"] = tc.clone();
        }
        if let Some(t) = ir.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(p) = ir.top_p {
            body["top_p"] = json!(p);
        }
        if let Some(mt) = ir.max_tokens {
            body["max_tokens"] = json!(mt);
        }
        if let Some(stop) = &ir.stop_sequences {
            body["stop"] = if stop.len() == 1 {
                json!(&stop[0])
            } else {
                json!(stop)
            };
        }
        if let Some(rf) = &ir.response_format {
            body["response_format"] = rf.clone();
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        let mut c = json!({
            "id": chunk.id.clone().unwrap_or_default(),
            "object": "chat.completion.chunk",
            "model": chunk.model.clone().unwrap_or_default(),
            "choices": [{}],
        });

        let mut delta = json!({});
        if let Some(content) = &chunk.delta_content {
            delta["content"] = json!(content);
        }
        if let Some(tool_calls) = &chunk.delta_tool_calls {
            delta["tool_calls"] = json!(tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "index": tc.index,
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect::<Vec<_>>());
        }
        c["choices"][0]["delta"] = delta;

        if let Some(finish) = &chunk.finish_reason {
            c["choices"][0]["finish_reason"] = json!(finish);
        }
        if let Some(usage) = &chunk.usage {
            c["usage"] = json!({
                "prompt_tokens": usage.prompt_tokens,
                "completion_tokens": usage.completion_tokens,
                "total_tokens": usage.total_tokens,
            });
        }

        format!(
            "data: {}\n\n",
            serde_json::to_string(&c).unwrap_or_default()
        )
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let content = ir
            .message
            .content
            .iter()
            .find_map(|p| match p {
                IrContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let mut msg = json!({
            "id": ir.id.clone().unwrap_or_default(),
            "object": "chat.completion",
            "model": ir.model.clone().unwrap_or_default(),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content,
                },
                "finish_reason": ir.finish_reason.clone().unwrap_or_else(|| "stop".into()),
            }],
            "usage": {
                "prompt_tokens": ir.usage.prompt_tokens,
                "completion_tokens": ir.usage.completion_tokens,
                "total_tokens": ir.usage.total_tokens,
            },
        });

        if let Some(tool_calls) = &ir.message.tool_calls {
            msg["choices"][0]["message"]["tool_calls"] = json!(tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": { "name": tc.name, "arguments": tc.arguments }
                    })
                })
                .collect::<Vec<_>>());
        }

        Ok(msg)
    }
}
