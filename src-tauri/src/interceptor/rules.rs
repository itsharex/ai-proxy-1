use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RulePhase {
    Pre,
    Post,
}

impl RulePhase {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pre" => Some(Self::Pre),
            "post" => Some(Self::Post),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pre => "pre",
            Self::Post => "post",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleCondition {
    ModelMatches { pattern: String },
    PathContains { substring: String },
    HeaderExists { name: String },
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleAction {
    ReplaceModel {
        model: String,
    },
    SetHeader {
        name: String,
        value: String,
    },
    RemoveHeader {
        name: String,
    },
    InjectSystemPrompt {
        prompt: String,
    },
    OverrideParameter {
        parameter: String,
        value: serde_json::Value,
    },
    FilterResponse {
        patterns: Vec<String>,
    },
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
