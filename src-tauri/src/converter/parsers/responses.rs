use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct ResponsesParser;

impl FormatParser for ResponsesParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"]
            .as_str()
            .ok_or_else(|| ProxyError::Parse("missing model".into()))?
            .to_string();

        let input = body["input"]
            .as_str()
            .ok_or_else(|| ProxyError::Parse("missing input".into()))?;

        let instructions = body["instructions"].as_str();

        let mut messages = Vec::new();

        if let Some(inst) = instructions {
            messages.push(IrMessage {
                role: IrRole::System,
                content: vec![IrContentPart::Text {
                    text: inst.to_string(),
                }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            });
        }

        messages.push(IrMessage {
            role: IrRole::User,
            content: vec![IrContentPart::Text {
                text: input.to_string(),
            }],
            name: None,
            tool_call_id: None,
            tool_calls: None,
        });

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|tool| {
                        Some(IrTool {
                            name: tool.get("name")?.as_str()?.to_string(),
                            description: tool
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            input_schema: tool
                                .get("parameters")
                                .cloned()
                                .unwrap_or(Value::Null),
                        })
                    })
                    .collect()
            })
        });

        Ok(IrRequest {
            model,
            messages,
            tools,
            tool_choice: body.get("tool_choice").cloned(),
            temperature: body.get("temperature").and_then(|v| v.as_f64()).map(|v| v as f32),
            top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32),
            max_tokens: body
                .get("max_output_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            stream: body
                .get("stream")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            stop_sequences: None,
            response_format: body.get("text").cloned(),
            metadata: std::collections::HashMap::new(),
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data.is_empty() || data == "[DONE]" {
            return Ok(None);
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let event_type = chunk["type"].as_str().unwrap_or("");

        match event_type {
            "response.output_text.delta" => Ok(Some(IrStreamChunk {
                id: chunk["response_id"].as_str().map(String::from),
                model: None,
                delta_content: chunk["delta"].as_str().map(String::from),
                delta_tool_calls: None,
                finish_reason: None,
                usage: None,
            })),
            "response.completed" => {
                let response = &chunk["response"];
                Ok(Some(IrStreamChunk {
                    id: response["id"].as_str().map(String::from),
                    model: response["model"].as_str().map(String::from),
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason: Some("stop".into()),
                    usage: response.get("usage").map(|u| IrUsage {
                        prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                        completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                        total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
                    }),
                }))
            }
            _ => Ok(None),
        }
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let output = &body["output"];
        let text = output
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|item| item["type"] == "message")
                    .and_then(|msg| msg["content"].as_array())
                    .and_then(|content| {
                        content
                            .iter()
                            .find(|c| c["type"] == "output_text")
                            .and_then(|ot| ot["text"].as_str())
                    })
            })
            .unwrap_or("");

        Ok(IrResponse {
            id: body["id"].as_str().map(String::from),
            model: body["model"].as_str().map(String::from),
            message: IrMessage {
                role: IrRole::Assistant,
                content: vec![IrContentPart::Text {
                    text: text.to_string(),
                }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason: body["status"].as_str().map(|s| {
                if s == "completed" {
                    "stop".into()
                } else {
                    s.to_string()
                }
            }),
            usage: body
                .get("usage")
                .map(|u| IrUsage {
                    prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                    completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                    total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
                })
                .unwrap_or(IrUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                }),
        })
    }
}
