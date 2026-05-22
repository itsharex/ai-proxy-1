use crate::converter::ir::{IrContentPart, IrRequest, IrResponse, IrRole, IrStreamChunk};
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct ResponsesGenerator;

impl FormatGenerator for ResponsesGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let mut instructions: Option<String> = None;
        let mut input_items: Vec<Value> = Vec::new();

        for msg in &ir.messages {
            match msg.role {
                IrRole::System => {
                    let text = extract_text_content(&msg.content);
                    instructions = Some(text);
                }
                IrRole::User => {
                    let content = convert_message_content(&msg.content);
                    input_items.push(json!({
                        "role": "user",
                        "content": content,
                    }));
                }
                IrRole::Assistant => {
                    let content = convert_message_content(&msg.content);
                    input_items.push(json!({
                        "role": "assistant",
                        "content": content,
                    }));
                }
                IrRole::Tool => {
                    if let Some(IrContentPart::ToolResult {
                        tool_use_id,
                        content,
                    }) = msg.content.first()
                    {
                        input_items.push(json!({
                            "type": "function_call_output",
                            "call_id": tool_use_id,
                            "output": content,
                        }));
                    }
                }
            }
        }

        let mut body = json!({
            "model": ir.model,
            "input": input_items,
        });

        if let Some(instructions) = instructions {
            body["instructions"] = json!(instructions);
        }

        if let Some(tools) = &ir.tools {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    let mut tool = json!({
                        "type": "function",
                        "name": t.name,
                        "parameters": t.input_schema,
                    });
                    if let Some(desc) = &t.description {
                        tool["description"] = json!(desc);
                    }
                    tool
                })
                .collect();
            body["tools"] = json!(tool_defs);
        }

        if let Some(temperature) = ir.temperature {
            body["temperature"] = json!(temperature);
        }

        if let Some(top_p) = ir.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(max_tokens) = ir.max_tokens {
            body["max_output_tokens"] = json!(max_tokens);
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        let response_id = chunk.id.as_deref().unwrap_or("resp-proxy");

        if let Some(_reason) = &chunk.finish_reason {
            let completed = json!({
                "type": "response.completed",
                "response": {
                    "id": response_id,
                    "object": "response",
                    "status": "completed",
                    "output": [],
                }
            });
            return format!("data: {}\n\n", completed);
        }

        if let Some(content) = &chunk.delta_content {
            let delta_event = json!({
                "type": "response.output_text.delta",
                "delta": content,
                "response_id": response_id,
            });
            return format!("data: {}\n\n", delta_event);
        }

        let empty = json!({
            "type": "response.output_text.delta",
            "delta": "",
            "response_id": response_id,
        });
        format!("data: {}\n\n", empty)
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let id = ir.id.as_deref().unwrap_or("resp-proxy");

        let text = extract_text_content(&ir.message.content);

        let output = json!([{
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "output_text",
                "text": text,
            }],
        }]);

        Ok(json!({
            "id": id,
            "object": "response",
            "status": "completed",
            "output": output,
            "usage": {
                "input_tokens": ir.usage.prompt_tokens,
                "output_tokens": ir.usage.completion_tokens,
            },
            "model": ir.model.as_deref().unwrap_or(""),
        }))
    }
}

fn extract_text_content(parts: &[IrContentPart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            IrContentPart::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn convert_message_content(parts: &[IrContentPart]) -> Value {
    if parts.len() == 1 {
        if let Some(IrContentPart::Text { text }) = parts.first() {
            return json!(text);
        }
    }

    let items: Vec<Value> = parts
        .iter()
        .map(|part| match part {
            IrContentPart::Text { text } => json!({
                "type": "input_text",
                "text": text,
            }),
            IrContentPart::Image { url, data, media_type } => {
                if let Some(image_url) = url {
                    json!({
                        "type": "input_image",
                        "image_url": image_url,
                    })
                } else if let Some(b64) = data {
                    let mt = media_type.as_deref().unwrap_or("image/png");
                    json!({
                        "type": "input_image",
                        "image_url": format!("data:{};base64,{}", mt, b64),
                    })
                } else {
                    json!({ "type": "input_image", "image_url": "" })
                }
            }
            IrContentPart::ToolUse { id, name, input } => json!({
                "type": "function_call",
                "call_id": id,
                "name": name,
                "arguments": input.to_string(),
            }),
            IrContentPart::ToolResult { tool_use_id, content } => json!({
                "type": "function_call_output",
                "call_id": tool_use_id,
                "output": content,
            }),
        })
        .collect();

    json!(items)
}
