use crate::db::pool::get_pool;
use crate::error::ProxyError;

pub enum Strategy {
    RoundRobin,
    Random,
    LeastUsed,
}

pub struct KeyRotation;

impl KeyRotation {
    pub async fn get_next_key(
        provider_id: &str,
        strategy: Strategy,
    ) -> Result<(String, Vec<u8>, Vec<u8>), ProxyError> {
        let pool = get_pool().await;
        let order = match strategy {
            Strategy::LeastUsed => "usage_count ASC",
            Strategy::Random => "RANDOM()",
            Strategy::RoundRobin => "usage_count ASC",
        };

        let query = format!(
            "SELECT id, encrypted_key, nonce FROM api_keys WHERE provider_id = ? AND is_active = 1 ORDER BY {} LIMIT 1",
            order
        );

        let row = sqlx::query_as::<_, DbKeyRow>(&query)
            .bind(provider_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                ProxyError::KeyManagement(format!(
                    "No active key for provider {}",
                    provider_id
                ))
            })?;

        sqlx::query(
            "UPDATE api_keys SET usage_count = usage_count + 1, last_used_at = datetime('now') WHERE id = ?",
        )
        .bind(&row.id)
        .execute(pool)
        .await?;

        Ok((row.id, row.encrypted_key, row.nonce))
    }
}

#[derive(sqlx::FromRow)]
struct DbKeyRow {
    id: String,
    encrypted_key: Vec<u8>,
    nonce: Vec<u8>,
}
