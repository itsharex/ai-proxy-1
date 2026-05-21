use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct GeminiParser;

impl FormatParser for GeminiParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"].as_str().unwrap_or("unknown").to_string();

        let mut messages = Vec::new();

        if let Some(si) = body.get("systemInstruction") {
            let text = si["parts"]
                .as_array()
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            if !text.is_empty() {
                messages.push(IrMessage {
                    role: IrRole::System,
                    content: vec![IrContentPart::Text { text }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        if let Some(contents) = body["contents"].as_array() {
            for c in contents {
                let role = match c["role"].as_str().unwrap_or("user") {
                    "user" => IrRole::User,
                    "model" => IrRole::Assistant,
                    _ => IrRole::User,
                };

                let parts = c["parts"]
                    .as_array()
                    .map(|p_arr| {
                        p_arr
                            .iter()
                            .filter_map(|p| {
                                if let Some(text) = p["text"].as_str() {
                                    Some(IrContentPart::Text {
                                        text: text.to_string(),
                                    })
                                } else if let Some(fc) = p.get("functionCall") {
                                    Some(IrContentPart::ToolUse {
                                        id: fc
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        name: fc["name"].as_str().unwrap_or("").to_string(),
                                        input: fc
                                            .get("args")
                                            .cloned()
                                            .unwrap_or(Value::Null),
                                    })
                                } else if let Some(fr) = p.get("functionResponse") {
                                    Some(IrContentPart::ToolResult {
                                        tool_use_id: fr
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        content: fr
                                            .get("response")
                                            .map(|r| r.to_string())
                                            .unwrap_or_default(),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                messages.push(IrMessage {
                    role,
                    content: parts,
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| {
                let mut all_tools = Vec::new();
                for tool in arr {
                    if let Some(fd_arr) = tool
                        .get("functionDeclarations")
                        .and_then(|f| f.as_array())
                    {
                        for fd in fd_arr {
                            all_tools.push(IrTool {
                                name: fd["name"].as_str().unwrap_or("").to_string(),
                                description: fd
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                input_schema: fd
                                    .get("parameters")
                                    .cloned()
                                    .unwrap_or(Value::Null),
                            });
                        }
                    }
                }
                all_tools
            })
        });
        let tools = if tools.as_ref().map_or(false, |v| v.is_empty()) {
            None
        } else {
            tools
        };

        Ok(IrRequest {
            model,
            messages,
            tools,
            tool_choice: body.get("toolConfig").cloned(),
            temperature: body
                .get("generationConfig")
                .and_then(|gc| gc["temperature"].as_f64())
                .map(|v| v as f32),
            top_p: body
                .get("generationConfig")
                .and_then(|gc| gc["topP"].as_f64())
                .map(|v| v as f32),
            max_tokens: body
                .get("generationConfig")
                .and_then(|gc| gc["maxOutputTokens"].as_u64())
                .map(|v| v as u32),
            stream: false,
            stop_sequences: body
                .get("generationConfig")
                .and_then(|gc| gc["stopSequences"].as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                }),
            response_format: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data.is_empty() {
            return Ok(None);
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let delta_content = chunk["candidates"]
            .get(0)
            .and_then(|c| c["content"]["parts"].get(0))
            .and_then(|p| p["text"].as_str())
            .map(String::from);

        let finish_reason = chunk["candidates"]
            .get(0)
            .and_then(|c| c["finishReason"].as_str())
            .map(String::from);

        Ok(Some(IrStreamChunk {
            id: None,
            model: None,
            delta_content,
            delta_tool_calls: None,
            finish_reason,
            usage: chunk.get("usageMetadata").map(|u| IrUsage {
                prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            }),
        }))
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let candidate = body["candidates"]
            .get(0)
            .ok_or(ProxyError::Parse("no candidates".into()))?;
        let text = candidate["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        Ok(IrResponse {
            id: None,
            model: None,
            message: IrMessage {
                role: IrRole::Assistant,
                content: vec![IrContentPart::Text { text }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason: candidate["finishReason"].as_str().map(String::from),
            usage: body
                .get("usageMetadata")
                .map(|u| IrUsage {
                    prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                    completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                    total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
                })
                .unwrap_or(IrUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                }),
        })
    }
}
