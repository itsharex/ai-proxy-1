use crate::db::pool::get_pool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RequestLog {
    pub id: i64,
    pub request_id: String,
    pub client_format: String,
    pub provider_name: String,
    pub provider_format: String,
    pub model: String,
    pub stream: i64,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub error_message: Option<String>,
    pub created_at: String,
}

pub struct LogStore;

impl LogStore {
    pub async fn insert(
        request_id: &str,
        client_format: &str,
        provider_name: &str,
        provider_format: &str,
        model: &str,
        stream: bool,
        status_code: Option<u16>,
        duration_ms: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        let pool = get_pool().await;
        sqlx::query(
            "INSERT INTO request_logs (request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(request_id)
        .bind(client_format)
        .bind(provider_name)
        .bind(provider_format)
        .bind(model)
        .bind(stream as i64)
        .bind(status_code.map(|s| s as i64))
        .bind(duration_ms)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list(limit: i64) -> Result<Vec<RequestLog>, sqlx::Error> {
        let pool = get_pool().await;
        let rows = sqlx::query_as::<_, RequestLog>(
            "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_by_id(id: i64) -> Result<Option<RequestLog>, sqlx::Error> {
        let pool = get_pool().await;
        let row = sqlx::query_as::<_, RequestLog>(
            "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }
}
