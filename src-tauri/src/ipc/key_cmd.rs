use crate::db::pool::get_pool;
use crate::key::store::KeyStore;
use serde::Serialize;
use tauri::command;

#[command]
pub async fn get_api_keys(provider_id: String) -> Result<Vec<KeyInfo>, String> {
    let pool = get_pool().await;
    let rows = sqlx::query_as::<_, DbKeyInfo>(
        "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ? ORDER BY created_at DESC"
    )
    .bind(&provider_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| KeyInfo {
            id: r.id,
            label: r.label,
            is_active: r.is_active != 0,
            usage_count: r.usage_count,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
        })
        .collect())
}

#[command]
pub async fn create_api_key(
    provider_id: String,
    label: String,
    plaintext_key: String,
) -> Result<String, String> {
    let pool = get_pool().await;
    let key_store = KeyStore::new(&KeyStore::derive_key());
    let (encrypted, nonce) = key_store.encrypt(&plaintext_key).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO api_keys (id, provider_id, label, encrypted_key, nonce) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&provider_id)
    .bind(&label)
    .bind(&encrypted)
    .bind(&nonce)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(id)
}

#[command]
pub async fn delete_api_key(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM api_keys WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct KeyInfo {
    pub id: String,
    pub label: String,
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct DbKeyInfo {
    id: String,
    label: String,
    is_active: i64,
    usage_count: i64,
    last_used_at: Option<String>,
    created_at: String,
}
