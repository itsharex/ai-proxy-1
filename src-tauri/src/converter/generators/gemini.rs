use crate::converter::ir::{IrContentPart, IrRequest, IrResponse, IrRole, IrStreamChunk};
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct GeminiGenerator;

impl FormatGenerator for GeminiGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let mut system_instruction: Option<Value> = None;
        let mut contents: Vec<Value> = Vec::new();

        for msg in &ir.messages {
            match msg.role {
                IrRole::System => {
                    let text = extract_text_parts(&msg.content);
                    system_instruction = Some(json!({
                        "parts": [{ "text": text }]
                    }));
                }
                _ => {
                    let role = match msg.role {
                        IrRole::User => "user",
                        IrRole::Assistant => "model",
                        IrRole::Tool => "user",
                        _ => "user",
                    };

                    let parts = convert_gemini_parts(&msg.content);

                    contents.push(json!({
                        "role": role,
                        "parts": parts,
                    }));
                }
            }
        }

        let mut body = json!({
            "contents": contents,
        });

        if let Some(si) = system_instruction {
            body["systemInstruction"] = si;
        }

        let mut generation_config = json!({});
        let mut has_config = false;

        if let Some(temperature) = ir.temperature {
            generation_config["temperature"] = json!(temperature);
            has_config = true;
        }

        if let Some(top_p) = ir.top_p {
            generation_config["topP"] = json!(top_p);
            has_config = true;
        }

        if let Some(max_tokens) = ir.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
            has_config = true;
        }

        if let Some(stop) = &ir.stop_sequences {
            generation_config["stopSequences"] = json!(stop);
            has_config = true;
        }

        if let Some(response_format) = &ir.response_format {
            if let Some(rt) = response_format.get("type").and_then(|t| t.as_str()) {
                generation_config["responseMimeType"] = match rt {
                    "json_object" => json!("application/json"),
                    _ => json!("text/plain"),
                };
                has_config = true;
            }
        }

        if has_config {
            body["generationConfig"] = generation_config;
        }

        if let Some(tools) = &ir.tools {
            let function_declarations: Vec<Value> = tools
                .iter()
                .map(|t| {
                    let mut decl = json!({
                        "name": t.name,
                        "parameters": t.input_schema,
                    });
                    if let Some(desc) = &t.description {
                        decl["description"] = json!(desc);
                    }
                    decl
                })
                .collect();
            body["tools"] = json!([{
                "function_declarations": function_declarations,
            }]);
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        let mut candidate = json!({
            "content": {
                "role": "model",
                "parts": [],
            },
        });

        if let Some(content) = &chunk.delta_content {
            candidate["content"]["parts"] = json!([{ "text": content }]);
        }

        if let Some(reason) = &chunk.finish_reason {
            let gemini_reason = match reason.as_str() {
                "stop" | "end_turn" => "STOP",
                "max_tokens" | "length" => "MAX_TOKENS",
                "tool_calls" | "tool_use" => "STOP",
                _ => "STOP",
            };
            candidate["finishReason"] = json!(gemini_reason);
        }

        let mut chunk_data = json!({
            "candidates": [candidate],
        });

        if let Some(usage) = &chunk.usage {
            chunk_data["usageMetadata"] = json!({
                "promptTokenCount": usage.prompt_tokens,
                "candidatesTokenCount": usage.completion_tokens,
                "totalTokenCount": usage.total_tokens,
            });
        }

        format!("data: {}\n\n", chunk_data)
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let text = extract_text_parts(&ir.message.content);

        let mut candidate = json!({
            "content": {
                "role": "model",
                "parts": [{ "text": text }],
            },
        });

        if let Some(reason) = &ir.finish_reason {
            let gemini_reason = match reason.as_str() {
                "stop" | "end_turn" => "STOP",
                "max_tokens" | "length" => "MAX_TOKENS",
                "tool_calls" | "tool_use" => "STOP",
                _ => "STOP",
            };
            candidate["finishReason"] = json!(gemini_reason);
        } else {
            candidate["finishReason"] = json!("STOP");
        }

        if let Some(tool_calls) = &ir.message.tool_calls {
            let parts: Vec<Value> = tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "functionCall": {
                            "name": tc.name,
                            "args": serde_json::from_str::<Value>(&tc.arguments)
                                .unwrap_or(json!({})),
                        }
                    })
                })
                .collect();
            candidate["content"]["parts"] = json!(parts);
        }

        Ok(json!({
            "candidates": [candidate],
            "usageMetadata": {
                "promptTokenCount": ir.usage.prompt_tokens,
                "candidatesTokenCount": ir.usage.completion_tokens,
                "totalTokenCount": ir.usage.total_tokens,
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

fn convert_gemini_parts(parts: &[IrContentPart]) -> Vec<Value> {
    parts
        .iter()
        .map(|part| match part {
            IrContentPart::Text { text } => json!({ "text": text }),
            IrContentPart::Image { url, data, media_type } => {
                if let Some(image_url) = url {
                    json!({
                        "file_data": {
                            "file_uri": image_url,
                            "mime_type": media_type.as_deref().unwrap_or("image/png"),
                        }
                    })
                } else if let Some(b64) = data {
                    json!({
                        "inline_data": {
                            "mime_type": media_type.as_deref().unwrap_or("image/png"),
                            "data": b64,
                        }
                    })
                } else {
                    json!({ "text": "" })
                }
            }
            IrContentPart::ToolUse { name, input, .. } => {
                json!({
                    "functionCall": {
                        "name": name,
                        "args": input,
                    }
                })
            }
            IrContentPart::ToolResult { content, .. } => {
                json!({
                    "functionResponse": {
                        "response": {
                            "result": content,
                        }
                    }
                })
            }
        })
        .collect()
}
