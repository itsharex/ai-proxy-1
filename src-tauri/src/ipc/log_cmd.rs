use crate::db::get_pool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: i64,
    pub request_id: String,
    pub client_format: String,
    pub provider_name: String,
    pub provider_format: String,
    pub model: String,
    pub stream: bool,
    pub duration_ms: Option<i64>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogList {
    pub logs: Vec<LogEntry>,
    pub total: i64,
}

#[tauri::command]
pub async fn get_logs(page: i64, limit: i64) -> Result<LogList, String> {
    let pool = get_pool().await;
    let offset = (page - 1).max(0) * limit;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM request_logs")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let rows: Vec<(i64, String, String, String, String, String, i32, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, String)> = sqlx::query_as(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs ORDER BY id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let logs = rows
        .into_iter()
        .map(|(id, request_id, client_format, provider_name, provider_format, model, stream, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at)| {
            LogEntry {
                id,
                request_id,
                client_format,
                provider_name,
                provider_format,
                model,
                stream: stream != 0,
                duration_ms,
                prompt_tokens: prompt_tokens.unwrap_or(0),
                completion_tokens: completion_tokens.unwrap_or(0),
                total_tokens: total_tokens.unwrap_or(0),
                error_message,
                created_at,
            }
        })
        .collect();

    Ok(LogList {
        logs,
        total: total.0,
    })
}

#[tauri::command]
pub async fn get_log_detail(id: i64) -> Result<LogEntry, String> {
    let pool = get_pool().await;

    let row: (i64, String, String, String, String, String, i32, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, String) = sqlx::query_as(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(LogEntry {
        id: row.0,
        request_id: row.1,
        client_format: row.2,
        provider_name: row.3,
        provider_format: row.4,
        model: row.5,
        stream: row.6 != 0,
        duration_ms: row.7,
        prompt_tokens: row.8.unwrap_or(0),
        completion_tokens: row.9.unwrap_or(0),
        total_tokens: row.10.unwrap_or(0),
        error_message: row.11,
        created_at: row.12,
    })
}
