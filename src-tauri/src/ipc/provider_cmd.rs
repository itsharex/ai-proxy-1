use crate::db::pool::get_pool;
use crate::provider::manager::ProviderManager;
use tauri::command;

#[command]
pub async fn get_providers() -> Result<Vec<crate::provider::endpoint::Provider>, String> {
    let pool = get_pool().await;
    ProviderManager::list(pool).await.map_err(|e| e.to_string())
}

#[command]
pub async fn create_provider(
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: String,
    endpoints: Vec<EndpointInput>,
) -> Result<String, String> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO providers (id, name, base_url, auth_type, auth_header) VALUES (?, ?, ?, ?, ?)"
    )
        .bind(&id)
        .bind(&name)
        .bind(&base_url)
        .bind(&auth_type)
        .bind(&auth_header)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    for ep in endpoints {
        sqlx::query(
            "INSERT INTO endpoints (id, provider_id, format, path) VALUES (?, ?, ?, ?)"
        )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&id)
            .bind(&ep.format)
            .bind(&ep.path)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(id)
}

#[command]
pub async fn update_provider(id: String, name: String, base_url: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query(
        "UPDATE providers SET name = ?, base_url = ?, updated_at = datetime('now') WHERE id = ?"
    )
        .bind(&name)
        .bind(&base_url)
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn delete_provider(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct EndpointInput {
    pub format: String,
    pub path: String,
}
