use crate::db::pool::get_pool;
use crate::interceptor::rules::{InterceptorRule, RulePhase};
use tauri::command;

#[command]
pub async fn get_rules() -> Result<Vec<InterceptorRule>, String> {
    let pool = get_pool().await;
    let rows = sqlx::query_as::<_, DbRule>(
        "SELECT id, name, phase, rule_type, condition_json, action_json, priority, enabled FROM interceptor_rules ORDER BY priority ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|r| {
            let condition =
                serde_json::from_str(&r.condition_json).map_err(|e| e.to_string())?;
            let action =
                serde_json::from_str(&r.action_json).map_err(|e| e.to_string())?;
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
        .collect()
}

#[command]
pub async fn create_rule(
    name: String,
    phase: String,
    condition_json: String,
    action_json: String,
    priority: i64,
) -> Result<String, String> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO interceptor_rules (id, name, phase, rule_type, condition_json, action_json, priority) VALUES (?, ?, ?, 'custom', ?, ?, ?)"
    )
        .bind(&id)
        .bind(&name)
        .bind(&phase)
        .bind(&condition_json)
        .bind(&action_json)
        .bind(priority)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[command]
pub async fn update_rule(id: String, enabled: bool) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query("UPDATE interceptor_rules SET enabled = ? WHERE id = ?")
        .bind(enabled as i64)
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn delete_rule(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM interceptor_rules WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
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
