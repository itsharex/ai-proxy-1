use crate::db::get_pool;
use crate::interceptor::rules::InterceptorRule;
use serde::Deserialize;
use uuid::Uuid;

#[tauri::command]
pub async fn get_rules() -> Result<Vec<InterceptorRule>, String> {
    let pool = get_pool().await;

    let rows: Vec<(String, String, String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT id, name, phase, rule_type, condition_json, action_json, priority, enabled FROM interceptor_rules ORDER BY priority DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut rules = Vec::new();
    for (id, name, phase, _rule_type, condition_json, action_json, priority, enabled) in rows {
        use crate::interceptor::rules::{RuleAction, RuleCondition, RulePhase};

        let rule_phase = RulePhase::from_str(&phase).unwrap_or(RulePhase::Pre);
        let condition: RuleCondition =
            serde_json::from_str(&condition_json).unwrap_or(RuleCondition::Always);
        let action: RuleAction = serde_json::from_str(&action_json).unwrap_or(
            RuleAction::SetHeader {
                name: "x-no-op".into(),
                value: "true".into(),
            },
        );

        rules.push(InterceptorRule {
            id,
            name,
            phase: rule_phase,
            condition,
            action,
            priority,
            enabled: enabled != 0,
        });
    }

    Ok(rules)
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleInput {
    pub name: String,
    pub phase: String,
    pub condition: serde_json::Value,
    pub action: serde_json::Value,
    pub priority: Option<i64>,
    pub enabled: Option<bool>,
}

#[tauri::command]
pub async fn create_rule(input: CreateRuleInput) -> Result<InterceptorRule, String> {
    let pool = get_pool().await;
    let id = Uuid::new_v4().to_string();
    let priority = input.priority.unwrap_or(0);
    let enabled = input.enabled.unwrap_or(true) as i32;

    let condition_json = serde_json::to_string(&input.condition)
        .map_err(|e| e.to_string())?;
    let action_json = serde_json::to_string(&input.action)
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO interceptor_rules (id, name, phase, rule_type, condition_json, action_json, priority, enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.phase)
    .bind("custom")
    .bind(&condition_json)
    .bind(&action_json)
    .bind(priority)
    .bind(enabled)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    use crate::interceptor::rules::{RuleAction, RuleCondition, RulePhase};

    let rule_phase = RulePhase::from_str(&input.phase).unwrap_or(RulePhase::Pre);
    let condition: RuleCondition =
        serde_json::from_value(input.condition).unwrap_or(RuleCondition::Always);
    let action: RuleAction =
        serde_json::from_value(input.action).unwrap_or(RuleAction::SetHeader {
            name: "x-no-op".into(),
            value: "true".into(),
        });

    Ok(InterceptorRule {
        id,
        name: input.name,
        phase: rule_phase,
        condition,
        action,
        priority,
        enabled: enabled != 0,
    })
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleInput {
    pub id: String,
    pub name: Option<String>,
    pub phase: Option<String>,
    pub condition: Option<serde_json::Value>,
    pub action: Option<serde_json::Value>,
    pub priority: Option<i64>,
    pub enabled: Option<bool>,
}

#[tauri::command]
pub async fn update_rule(input: UpdateRuleInput) -> Result<(), String> {
    let pool = get_pool().await;

    let current: (String, String, String, String, i64, i64) = sqlx::query_as(
        "SELECT name, phase, condition_json, action_json, priority, enabled FROM interceptor_rules WHERE id = ?",
    )
    .bind(&input.id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let name = input.name.unwrap_or(current.0);
    let phase = input.phase.unwrap_or(current.1);

    let condition_json = input
        .condition
        .map(|c| serde_json::to_string(&c).unwrap_or_default())
        .unwrap_or(current.2);

    let action_json = input
        .action
        .map(|a| serde_json::to_string(&a).unwrap_or_default())
        .unwrap_or(current.3);

    let priority = input.priority.unwrap_or(current.4);
    let enabled = input
        .enabled
        .map(|e| e as i32)
        .unwrap_or(current.5 as i32);

    sqlx::query(
        "UPDATE interceptor_rules SET name = ?, phase = ?, condition_json = ?, action_json = ?, priority = ?, enabled = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&phase)
    .bind(&condition_json)
    .bind(&action_json)
    .bind(priority)
    .bind(enabled)
    .bind(&input.id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_rule(id: String) -> Result<(), String> {
    let pool = get_pool().await;

    sqlx::query("DELETE FROM interceptor_rules WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
