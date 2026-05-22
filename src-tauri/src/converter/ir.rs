use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrRequest {
    pub model: String,
    pub messages: Vec<IrMessage>,
    pub tools: Option<Vec<IrTool>>,
    pub tool_choice: Option<Value>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub stop_sequences: Option<Vec<String>>,
    pub response_format: Option<Value>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrMessage {
    pub role: IrRole,
    pub content: Vec<IrContentPart>,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Vec<IrToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IrRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IrContentPart {
    Text { text: String },
    Image { url: Option<String>, data: Option<String>, media_type: Option<String> },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrStreamChunk {
    pub id: Option<String>,
    pub model: Option<String>,
    pub delta_content: Option<String>,
    pub delta_tool_calls: Option<Vec<IrToolCallDelta>>,
    pub finish_reason: Option<String>,
    pub usage: Option<IrUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrToolCallDelta {
    pub index: u32,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub message: IrMessage,
    pub finish_reason: Option<String>,
    pub usage: IrUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClientFormat {
    Completions,
    Responses,
    Anthropic,
    Gemini,
}

impl ClientFormat {
    #[allow(dead_code)]
    pub fn from_path(path: &str) -> Option<Self> {
        if path.contains("/v1/chat/completions") {
            Some(Self::Completions)
        } else if path.contains("/v1/responses") {
            Some(Self::Responses)
        } else if path.contains("/v1/messages") {
            Some(Self::Anthropic)
        } else if path.contains("/v1beta/models") {
            Some(Self::Gemini)
        } else {
            None
        }
    }
}
