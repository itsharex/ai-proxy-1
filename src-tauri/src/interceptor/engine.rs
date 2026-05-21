use super::rules::*;
use crate::converter::ir::IrRequest;
use crate::db::pool::get_pool;
use crate::error::ProxyError;
use std::collections::HashMap;

pub struct InterceptorEngine;

impl InterceptorEngine {
    pub async fn load_rules(phase: RulePhase) -> Result<Vec<InterceptorRule>, ProxyError> {
        let pool = get_pool().await;
        let phase_str = match phase {
            RulePhase::Pre => "pre",
            RulePhase::Post => "post",
        };

        let rows = sqlx::query_as::<_, DbRule>(
            "SELECT id, name, phase, rule_type, condition_json, action_json, priority, enabled FROM interceptor_rules WHERE phase = ? AND enabled = 1 ORDER BY priority ASC",
        )
        .bind(phase_str)
        .fetch_all(pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let condition: RuleCondition = serde_json::from_str(&r.condition_json)?;
                let action: RuleAction = serde_json::from_str(&r.action_json)?;
                Ok(InterceptorRule {
                    id: r.id,
                    name: r.name,
                    phase: if r.phase == "pre" {
                        RulePhase::Pre
                    } else {
                        RulePhase::Post
                    },
                    condition,
                    action,
                    priority: r.priority,
                    enabled: r.enabled != 0,
                })
            })
            .collect::<Result<Vec<_>, ProxyError>>()
    }

    pub fn check_condition(
        condition: &RuleCondition,
        request: &IrRequest,
        path: &str,
        headers: &HashMap<String, String>,
    ) -> bool {
        match condition {
            RuleCondition::ModelMatches { pattern } => {
                if pattern.contains('*') {
                    let p = pattern.replace('*', "");
                    request.model.contains(&p)
                } else {
                    request.model == *pattern
                }
            }
            RuleCondition::PathContains { substring } => path.contains(substring.as_str()),
            RuleCondition::HeaderExists { name } => headers.contains_key(name.as_str()),
            RuleCondition::Always => true,
        }
    }

    pub fn apply_action(
        action: &RuleAction,
        request: &mut IrRequest,
        headers: &mut HashMap<String, String>,
    ) {
        match action {
            RuleAction::ReplaceModel { new_model } => {
                request.model = new_model.clone();
            }
            RuleAction::SetHeader { name, value } => {
                headers.insert(name.clone(), value.clone());
            }
            RuleAction::RemoveHeader { name } => {
                headers.remove(name.as_str());
            }
            RuleAction::InjectSystemPrompt { text } => {
                request.messages.insert(
                    0,
                    crate::converter::ir::IrMessage {
                        role: crate::converter::ir::IrRole::System,
                        content: vec![crate::converter::ir::IrContentPart::Text {
                            text: text.clone(),
                        }],
                        name: None,
                        tool_call_id: None,
                        tool_calls: None,
                    },
                );
            }
            RuleAction::OverrideParameter { key, value } => match key.as_str() {
                "temperature" => {
                    if let Some(v) = value.as_f64() {
                        request.temperature = Some(v as f32);
                    }
                }
                "top_p" => {
                    if let Some(v) = value.as_f64() {
                        request.top_p = Some(v as f32);
                    }
                }
                "max_tokens" => {
                    if let Some(v) = value.as_u64() {
                        request.max_tokens = Some(v as u32);
                    }
                }
                _ => {
                    request.metadata.insert(key.clone(), value.clone());
                }
            },
            RuleAction::FilterResponse { .. } => {}
        }
    }

    pub async fn execute_pre_rules(
        request: &mut IrRequest,
        path: &str,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), ProxyError> {
        let rules = Self::load_rules(RulePhase::Pre).await?;
        for rule in rules {
            if Self::check_condition(&rule.condition, request, path, headers) {
                Self::apply_action(&rule.action, request, headers);
            }
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct DbRule {
    id: String,
    name: String,
    phase: String,
    rule_type: String,
    condition_json: String,
    action_json: String,
    priority: i64,
    enabled: i64,
}

impl From<serde_json::Error> for ProxyError {
    fn from(e: serde_json::Error) -> Self {
        ProxyError::Parse(format!("JSON error: {}", e))
    }
}
