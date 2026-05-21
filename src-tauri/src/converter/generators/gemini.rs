use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct GeminiGenerator;

impl FormatGenerator for GeminiGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for m in &ir.messages {
            if m.role == IrRole::System {
                let parts: Vec<Value> = m
                    .content
                    .iter()
                    .filter_map(|p| match p {
                        IrContentPart::Text { text } => Some(json!({"text": text})),
                        _ => None,
                    })
                    .collect();
                system_instruction = Some(json!({"parts": parts}));
            } else {
                let role = match m.role {
                    IrRole::User => "user",
                    IrRole::Assistant => "model",
                    _ => "user",
                };
                let parts: Vec<Value> = m
                    .content
                    .iter()
                    .map(|p| match p {
                        IrContentPart::Text { text } => json!({"text": text}),
                        _ => json!({"text": ""}),
                    })
                    .collect();
                contents.push(json!({"role": role, "parts": parts}));
            }
        }

        let mut body = json!({"contents": contents});

        if let Some(si) = system_instruction {
            body["systemInstruction"] = si;
        }

        let mut generation_config = json!({});
        if let Some(t) = ir.temperature {
            generation_config["temperature"] = json!(t);
        }
        if let Some(p) = ir.top_p {
            generation_config["topP"] = json!(p);
        }
        if let Some(mt) = ir.max_tokens {
            generation_config["maxOutputTokens"] = json!(mt);
        }
        if generation_config
            .as_object()
            .map_or(false, |m| !m.is_empty())
        {
            body["generationConfig"] = generation_config;
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        let c = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": chunk.delta_content.clone().unwrap_or_default()}]
                },
                "finishReason": chunk.finish_reason.clone(),
            }],
            "usageMetadata": chunk.usage.as_ref().map(|u| json!({
                "promptTokenCount": u.prompt_tokens,
                "candidatesTokenCount": u.completion_tokens,
                "totalTokenCount": u.total_tokens,
            })),
        });
        format!(
            "data: {}\n\n",
            serde_json::to_string(&c).unwrap_or_default()
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
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": text}],
                },
                "finishReason": ir.finish_reason.clone().unwrap_or_else(|| "STOP".into()),
            }],
            "usageMetadata": {
                "promptTokenCount": ir.usage.prompt_tokens,
                "candidatesTokenCount": ir.usage.completion_tokens,
                "totalTokenCount": ir.usage.total_tokens,
            },
        }))
    }
}
