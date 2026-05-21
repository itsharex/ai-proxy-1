use crate::db::pool::get_pool;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelRoute {
    pub id: String,
    pub model_pattern: String,
    pub alias: Option<String>,
    pub provider_id: String,
    pub target_model: String,
    pub target_format: String,
    pub fallback_provider_id: Option<String>,
    pub priority: i64,
}

#[command]
pub async fn get_routes() -> Result<Vec<ModelRoute>, String> {
    let pool = get_pool().await;
    let rows = sqlx::query_as::<_, DbRoute>(
        "SELECT id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority FROM model_routes ORDER BY priority DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| ModelRoute {
            id: r.id,
            model_pattern: r.model_pattern,
            alias: r.alias,
            provider_id: r.provider_id,
            target_model: r.target_model,
            target_format: r.target_format,
            fallback_provider_id: r.fallback_provider_id,
            priority: r.priority,
        })
        .collect())
}

#[command]
pub async fn create_route(
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: i64,
) -> Result<String, String> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO model_routes (id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
        .bind(&id)
        .bind(&model_pattern)
        .bind(&alias)
        .bind(&provider_id)
        .bind(&target_model)
        .bind(&target_format)
        .bind(&fallback_provider_id)
        .bind(priority)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[command]
pub async fn delete_route(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM model_routes WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct DbRoute {
    id: String,
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: i64,
}
