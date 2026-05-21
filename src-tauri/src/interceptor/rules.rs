use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RulePhase {
    Pre,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RuleCondition {
    ModelMatches { pattern: String },
    PathContains { substring: String },
    HeaderExists { name: String },
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RuleAction {
    ReplaceModel { new_model: String },
    SetHeader { name: String, value: String },
    RemoveHeader { name: String },
    InjectSystemPrompt { text: String },
    OverrideParameter { key: String, value: Value },
    FilterResponse { pattern: String, replacement: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptorRule {
    pub id: String,
    pub name: String,
    pub phase: RulePhase,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub priority: i64,
    pub enabled: bool,
}
