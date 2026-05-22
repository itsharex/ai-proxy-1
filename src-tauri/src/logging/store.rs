use crate::db::get_pool;
use crate::error::ProxyError;

pub async fn log_request(
    request_id: &str,
    client_format: &str,
    provider_name: &str,
    provider_format: &str,
    model: &str,
    stream: bool,
    duration_ms: i64,
    error_message: Option<&str>,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> Result<(), ProxyError> {
    let pool = get_pool().await;
    let total = prompt_tokens + completion_tokens;

    sqlx::query(
        "INSERT INTO request_logs (request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(request_id)
    .bind(client_format)
    .bind(provider_name)
    .bind(provider_format)
    .bind(model)
    .bind(stream as i64)
    .bind(200i64)
    .bind(duration_ms)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}
