use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct ResponsesGenerator;

impl FormatGenerator for ResponsesGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let instructions = ir.messages.iter()
            .find(|m| m.role == IrRole::System)
            .and_then(|m| {
                m.content.iter().find_map(|p| match p {
                    IrContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                })
            });

        let input = ir
            .messages
            .iter()
            .find(|m| m.role == IrRole::User)
            .and_then(|m| {
                m.content.iter().find_map(|p| match p {
                    IrContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                })
            })
            .unwrap_or_default();

        let mut body = json!({
            "model": ir.model,
            "input": input,
            "stream": ir.stream,
        });

        if let Some(inst) = instructions {
            body["instructions"] = json!(inst);
        }
        if let Some(tools) = &ir.tools {
            body["tools"] = json!(tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    })
                })
                .collect::<Vec<_>>());
        }
        if let Some(t) = ir.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(p) = ir.top_p {
            body["top_p"] = json!(p);
        }
        if let Some(mt) = ir.max_tokens {
            body["max_output_tokens"] = json!(mt);
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        if chunk.finish_reason.is_some() {
            let completed = json!({
                "type": "response.completed",
                "response": {
                    "id": chunk.id.clone().unwrap_or_default(),
                    "object": "response",
                    "model": chunk.model.clone().unwrap_or_default(),
                    "status": "completed",
                    "usage": chunk.usage.as_ref().map(|u| json!({
                        "input_tokens": u.prompt_tokens,
                        "output_tokens": u.completion_tokens,
                        "total_tokens": u.total_tokens,
                    })),
                }
            });
            return format!(
                "data: {}\n\n",
                serde_json::to_string(&completed).unwrap_or_default()
            );
        }

        let delta = json!({
            "type": "response.output_text.delta",
            "response_id": chunk.id.clone().unwrap_or_default(),
            "delta": chunk.delta_content.clone().unwrap_or_default(),
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
            "object": "response",
            "model": ir.model.clone().unwrap_or_default(),
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": text,
                }],
            }],
            "usage": {
                "input_tokens": ir.usage.prompt_tokens,
                "output_tokens": ir.usage.completion_tokens,
                "total_tokens": ir.usage.total_tokens,
            },
        }))
    }
}
