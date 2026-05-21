use crate::db::pool::get_pool;
use serde::Serialize;
use tauri::command;

#[derive(Debug, Serialize)]
pub struct RequestLogEntry {
    pub id: i64,
    pub request_id: String,
    pub client_format: String,
    pub provider_name: String,
    pub provider_format: String,
    pub model: String,
    pub stream: bool,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[command]
pub async fn get_logs(page: i64, limit: i64) -> Result<Vec<RequestLogEntry>, String> {
    let pool = get_pool().await;
    let offset = (page - 1) * limit;
    let rows = sqlx::query_as::<_, DbRequestLog>(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs ORDER BY id DESC LIMIT ? OFFSET ?"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| RequestLogEntry {
            id: r.id,
            request_id: r.request_id,
            client_format: r.client_format,
            provider_name: r.provider_name,
            provider_format: r.provider_format,
            model: r.model,
            stream: r.stream != 0,
            status_code: r.status_code,
            duration_ms: r.duration_ms,
            prompt_tokens: r.prompt_tokens,
            completion_tokens: r.completion_tokens,
            total_tokens: r.total_tokens,
            error_message: r.error_message,
            created_at: r.created_at,
        })
        .collect())
}

#[command]
pub async fn get_log_detail(id: i64) -> Result<Option<RequestLogEntry>, String> {
    let pool = get_pool().await;
    let row = sqlx::query_as::<_, DbRequestLog>(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|r| RequestLogEntry {
        id: r.id,
        request_id: r.request_id,
        client_format: r.client_format,
        provider_name: r.provider_name,
        provider_format: r.provider_format,
        model: r.model,
        stream: r.stream != 0,
        status_code: r.status_code,
        duration_ms: r.duration_ms,
        prompt_tokens: r.prompt_tokens,
        completion_tokens: r.completion_tokens,
        total_tokens: r.total_tokens,
        error_message: r.error_message,
        created_at: r.created_at,
    }))
}

#[derive(sqlx::FromRow)]
struct DbRequestLog {
    id: i64,
    request_id: String,
    client_format: String,
    provider_name: String,
    provider_format: String,
    model: String,
    stream: i64,
    status_code: Option<i64>,
    duration_ms: Option<i64>,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    error_message: Option<String>,
    created_at: String,
}
