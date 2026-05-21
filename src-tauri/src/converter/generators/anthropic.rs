use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct AnthropicGenerator;

impl FormatGenerator for AnthropicGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let system: Vec<String> = ir
            .messages
            .iter()
            .filter(|m| m.role == IrRole::System)
            .filter_map(|m| {
                m.content.iter().find_map(|p| match p {
                    IrContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                })
            })
            .collect();

        let messages: Vec<Value> = ir
            .messages
            .iter()
            .filter(|m| m.role != IrRole::System)
            .map(|m| {
                let role = match m.role {
                    IrRole::User => "user",
                    _ => "assistant",
                };

                let content: Vec<Value> = m
                    .content
                    .iter()
                    .map(|p| match p {
                        IrContentPart::Text { text } => json!({"type": "text", "text": text}),
                        IrContentPart::Image {
                            url: _,
                            data,
                            media_type,
                        } => {
                            let mut source = json!({"type": "base64"});
                            if let Some(mt) = media_type {
                                source["media_type"] = json!(mt);
                            }
                            if let Some(d) = data {
                                source["data"] = json!(d);
                            }
                            json!({"type": "image", "source": source})
                        }
                        IrContentPart::ToolUse { id, name, input } => json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input,
                        }),
                        IrContentPart::ToolResult {
                            tool_use_id,
                            content: c,
                        } => json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": c,
                        }),
                    })
                    .collect();

                let mut msg = json!({"role": role, "content": content});
                if m.content.len() == 1 {
                    if let IrContentPart::Text { text } = &m.content[0] {
                        msg["content"] = json!(text);
                    }
                }
                msg
            })
            .collect();

        let mut body = json!({
            "model": ir.model,
            "messages": messages,
            "max_tokens": ir.max_tokens.unwrap_or(4096),
        });

        if !system.is_empty() {
            body["system"] = if system.len() == 1 {
                json!(system[0])
            } else {
                json!(system
                    .iter()
                    .map(|s| json!({"type": "text", "text": s}))
                    .collect::<Vec<_>>())
            };
        }
        if let Some(t) = ir.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(p) = ir.top_p {
            body["top_p"] = json!(p);
        }
        if ir.stream {
            body["stream"] = json!(true);
        }
        if let Some(stop) = &ir.stop_sequences {
            body["stop_sequences"] = json!(stop);
        }
        if let Some(tools) = &ir.tools {
            body["tools"] = json!(tools
                .iter()
                .filter_map(|t| {
                    Some(json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                    }))
                })
                .collect::<Vec<_>>());
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        if chunk.finish_reason.is_some() {
            let msg_stop = json!({
                "type": "message_stop",
            });
            return format!(
                "data: {}\n\n",
                serde_json::to_string(&msg_stop).unwrap_or_default()
            );
        }

        let delta = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": chunk.delta_content.clone().unwrap_or_default(),
            }
        });
        format!(
            "data: {}\n\n",
            serde_json::to_string(&delta).unwrap_or_default()
        )
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let text = ir
            .message
            .content
            .iter()
            .find_map(|p| match p {
                IrContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();

        Ok(json!({
            "id": ir.id.clone().unwrap_or_default(),
            "type": "message",
            "role": "assistant",
            "model": ir.model.clone().unwrap_or_default(),
            "content": [{"type": "text", "text": text}],
            "stop_reason": ir.finish_reason.clone().unwrap_or_else(|| "end_turn".into()),
            "stop_sequence": null,
            "usage": {
                "input_tokens": ir.usage.prompt_tokens,
                "output_tokens": ir.usage.completion_tokens,
            },
        }))
    }
}
