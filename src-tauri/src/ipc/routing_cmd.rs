use crate::db::get_pool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[tauri::command]
pub async fn get_routes() -> Result<Vec<ModelRoute>, String> {
    let pool = get_pool().await;

    let rows: Vec<(String, String, Option<String>, String, String, String, Option<String>, i64)> = sqlx::query_as(
        "SELECT id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority FROM model_routes ORDER BY priority DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let routes = rows
        .into_iter()
        .map(|(id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority)| {
            ModelRoute {
                id,
                model_pattern,
                alias,
                provider_id,
                target_model,
                target_format,
                fallback_provider_id,
                priority,
            }
        })
        .collect();

    Ok(routes)
}

#[derive(Debug, Deserialize)]
pub struct CreateRouteInput {
    pub model_pattern: String,
    pub alias: Option<String>,
    pub provider_id: String,
    pub target_model: String,
    pub target_format: String,
    pub fallback_provider_id: Option<String>,
    pub priority: Option<i64>,
}

#[tauri::command]
pub async fn create_route(input: CreateRouteInput) -> Result<ModelRoute, String> {
    let pool = get_pool().await;
    let id = Uuid::new_v4().to_string();
    let priority = input.priority.unwrap_or(0);

    sqlx::query(
        "INSERT INTO model_routes (id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.model_pattern)
    .bind(&input.alias)
    .bind(&input.provider_id)
    .bind(&input.target_model)
    .bind(&input.target_format)
    .bind(&input.fallback_provider_id)
    .bind(priority)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ModelRoute {
        id,
        model_pattern: input.model_pattern,
        alias: input.alias,
        provider_id: input.provider_id,
        target_model: input.target_model,
        target_format: input.target_format,
        fallback_provider_id: input.fallback_provider_id,
        priority,
    })
}

#[tauri::command]
pub async fn delete_route(id: String) -> Result<(), String> {
    let pool = get_pool().await;

    sqlx::query("DELETE FROM model_routes WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
