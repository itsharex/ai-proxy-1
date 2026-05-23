use crate::converter::ir::{
    IrContentPart, IrRequest, IrResponse, IrRole, IrStreamChunk,
};
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct CompletionsGenerator;

impl FormatGenerator for CompletionsGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let mut messages = Vec::new();

        for msg in &ir.messages {
            let mut message = json!({
                "role": match msg.role {
                    IrRole::System => "system",
                    IrRole::Developer => "developer",
                    IrRole::User => "user",
                    IrRole::Assistant => "assistant",
                    IrRole::Tool => "tool",
                },
            });

            if let Some(name) = &msg.name {
                message["name"] = json!(name);
            }

            if let Some(tool_call_id) = &msg.tool_call_id {
                message["tool_call_id"] = json!(tool_call_id);
            }

            if msg.content.len() == 1 {
                if let Some(IrContentPart::Text { text }) = msg.content.first() {
                    message["content"] = json!(text);
                } else {
                    message["content"] = json!(serialize_content_parts(&msg.content));
                }
            } else if msg.content.is_empty() {
                message["content"] = json!(null);
            } else {
                message["content"] = json!(serialize_content_parts(&msg.content));
            }

            if let Some(tool_calls) = &msg.tool_calls {
                let calls: Vec<Value> = tool_calls
                    .iter()
                    .enumerate()
                    .map(|(i, tc)| {
                        json!({
                            "index": i,
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments,
                            }
                        })
                    })
                    .collect();
                message["tool_calls"] = json!(calls);
            }

            messages.push(message);
        }

        let mut body = json!({
            "model": ir.model,
            "messages": messages,
            "stream": ir.stream,
        });

        if let Some(tools) = &ir.tools {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    let mut func = json!({
                        "name": t.name,
                        "parameters": t.input_schema,
                    });
                    if let Some(desc) = &t.description {
                        func["description"] = json!(desc);
                    }
                    if let Some(strict) = t.strict {
                        func["strict"] = json!(strict);
                    }
                    json!({
                        "type": "function",
                        "function": func,
                    })
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

        if let Some(max_tokens) = ir.max_tokens {
            if ir.thinking.as_ref().map_or(false, |t| t.enabled) {
                body["max_completion_tokens"] = json!(max_tokens);
            } else {
                body["max_tokens"] = json!(max_tokens);
            }
        }

        if let Some(stop) = &ir.stop_sequences {
            body["stop"] = json!(stop);
        }

        if let Some(response_format) = &ir.response_format {
            body["response_format"] = response_format.clone();
        }

        if let Some(pp) = ir.presence_penalty {
            body["presence_penalty"] = json!(pp);
        }

        if let Some(fp) = ir.frequency_penalty {
            body["frequency_penalty"] = json!(fp);
        }

        if let Some(seed) = ir.seed {
            body["seed"] = json!(seed);
        }

        if let Some(thinking) = &ir.thinking {
            if thinking.enabled {
                if let Some(budget) = thinking.budget_tokens {
                    let effort = if budget <= 5000 { "low" }
                        else if budget <= 15000 { "medium" }
                        else { "high" };
                    body["reasoning"] = json!({ "effort": effort });
                }
            }
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        let id = chunk.id.as_deref().unwrap_or("chatcmpl-proxy");

        let mut delta = json!({});

        if let Some(content) = &chunk.delta_content {
            delta["content"] = json!(content);
        }

        if let Some(thinking) = &chunk.delta_thinking {
            delta["reasoning_content"] = json!(thinking);
        }

        if let Some(tool_calls) = &chunk.delta_tool_calls {
            let calls: Vec<Value> = tool_calls
                .iter()
                .map(|tc| {
                    let mut call = json!({
                        "index": tc.index,
                    });
                    if let Some(id) = &tc.id {
                        call["id"] = json!(id);
                        call["type"] = json!("function");
                    }
                    if let Some(name) = &tc.name {
                        call["function"] = json!({});
                        call["function"]["name"] = json!(name);
                    }
                    if let Some(args) = &tc.arguments {
                        if call.get("function").is_none() {
                            call["function"] = json!({});
                        }
                        call["function"]["arguments"] = json!(args);
                    }
                    call
                })
                .collect();
            delta["tool_calls"] = json!(calls);
        }

        let mut choice = json!({
            "index": 0,
            "delta": delta,
        });

        if let Some(reason) = &chunk.finish_reason {
            choice["finish_reason"] = json!(reason);
        } else {
            choice["finish_reason"] = json!(null);
        }

        let chunk_data = json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": chrono::Utc::now().timestamp(),
            "choices": [choice],
        });

        format!("data: {}\n\n", chunk_data)
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let id = ir.id.as_deref().unwrap_or("chatcmpl-proxy");

        let mut message = json!({
            "role": "assistant",
        });

        if ir.message.content.len() == 1 {
            if let Some(IrContentPart::Text { text }) = ir.message.content.first() {
                message["content"] = json!(text);
            } else {
                message["content"] = json!(serialize_content_parts(&ir.message.content));
            }
        } else if ir.message.content.is_empty() {
            message["content"] = json!(null);
        } else {
            message["content"] = json!(serialize_content_parts(&ir.message.content));
        }

        if let Some(tool_calls) = &ir.message.tool_calls {
            let calls: Vec<Value> = tool_calls
                .iter()
                .enumerate()
                .map(|(i, tc)| {
                    json!({
                        "index": i,
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect();
            message["tool_calls"] = json!(calls);
        }

        Ok(json!({
            "id": id,
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": ir.model.as_deref().unwrap_or(""),
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": ir.finish_reason.as_deref().unwrap_or("stop"),
            }],
            "usage": {
                "prompt_tokens": ir.usage.prompt_tokens,
                "completion_tokens": ir.usage.completion_tokens,
                "total_tokens": ir.usage.total_tokens,
            }
        }))
    }
}

fn serialize_content_parts(parts: &[IrContentPart]) -> Vec<Value> {
    parts
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
                if let Some(image_url) = url {
                    json!({
                        "type": "image_url",
                        "image_url": { "url": image_url },
                    })
                } else if let Some(b64) = data {
                    let mt = media_type.as_deref().unwrap_or("image/png");
                    json!({
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{};base64,{}", mt, b64),
                        },
                    })
                } else {
                    json!({ "type": "image_url", "image_url": { "url": "" } })
                }
            }
            IrContentPart::ToolUse { id, name, input } => json!({
                "type": "function",
                "id": id,
                "function": {
                    "name": name,
                    "arguments": input.to_string(),
                },
            }),
            IrContentPart::ToolResult { tool_use_id, content, .. } => json!({
                "type": "function",
                "tool_call_id": tool_use_id,
                "content": content,
            }),
        })
        .collect()
}
