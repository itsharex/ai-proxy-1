use crate::db::get_pool;
use crate::key::store::encrypt_api_key;
use crate::provider::endpoint::ApiKeyInfo;
use tauri::command;

#[command]
pub async fn get_api_keys(provider_id: String) -> Result<Vec<ApiKeyInfo>, String> {
    let pool = get_pool().await;
    let rows = sqlx::query_as::<_, (String, String, i64, i64, Option<String>, String)>(
        "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ? ORDER BY created_at DESC"
    )
    .bind(&provider_id)
    .fetch_all(pool)
    .await.map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|(id, label, is_active, usage_count, last_used_at, created_at)| ApiKeyInfo {
        id, label, is_active: is_active != 0, usage_count, last_used_at, created_at,
    }).collect())
}

#[command]
pub async fn create_api_key(provider_id: String, label: String, plaintext_key: String) -> Result<String, String> {
    let pool = get_pool().await;
    let (encrypted, nonce) = encrypt_api_key(&plaintext_key).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO api_keys (id, provider_id, label, encrypted_key, nonce) VALUES (?, ?, ?, ?, ?)")
        .bind(&id).bind(&provider_id).bind(&label).bind(&encrypted).bind(&nonce.as_slice())
        .execute(pool).await.map_err(|e| e.to_string())?;

    Ok(id)
}

#[command]
pub async fn delete_api_key(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM api_keys WHERE id = ?")
        .bind(&id)
        .execute(pool).await.map_err(|e| e.to_string())?;
    Ok(())
}
