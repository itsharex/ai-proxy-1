use serde::Deserialize;
use sqlx::FromRow;

use crate::db::get_pool;
use crate::error::ProxyError;

#[derive(Debug, FromRow, Deserialize)]
#[allow(dead_code)]
struct DbApiKey {
    id: String,
    provider_id: String,
    encrypted_key: Vec<u8>,
    nonce: Vec<u8>,
    is_active: i64,
    usage_count: i64,
}

#[derive(Debug, Clone)]
pub struct SelectedKey {
    #[allow(dead_code)]
    pub key_id: String,
    pub encrypted_key: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub enum RotationStrategy {
    LeastUsed,
}

pub struct KeyRotation;

impl KeyRotation {
    pub async fn get_next_key(
        provider_id: &str,
        strategy: &RotationStrategy,
    ) -> Result<SelectedKey, ProxyError> {
        let pool = get_pool().await;

        let query = match strategy {
            RotationStrategy::LeastUsed => {
                "SELECT id, provider_id, encrypted_key, nonce, is_active, usage_count FROM api_keys WHERE provider_id = ? AND is_active = 1 ORDER BY usage_count ASC, created_at ASC LIMIT 1"
            }
        };

        let db_key: DbApiKey = sqlx::query_as(query)
            .bind(provider_id)
            .fetch_one(pool)
            .await
            .map_err(|_| {
                ProxyError::KeyManagement(format!(
                    "no active API key found for provider: {}",
                    provider_id
                ))
            })?;

        let now = chrono::Utc::now().to_rfc3339();
        let new_count = db_key.usage_count + 1;

        sqlx::query("UPDATE api_keys SET usage_count = ?, last_used_at = ? WHERE id = ?")
            .bind(new_count)
            .bind(&now)
            .bind(&db_key.id)
            .execute(pool)
            .await?;

        Ok(SelectedKey {
            key_id: db_key.id,
            encrypted_key: db_key.encrypted_key,
            nonce: db_key.nonce,
        })
    }
}
