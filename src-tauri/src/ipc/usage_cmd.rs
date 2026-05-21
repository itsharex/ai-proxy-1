use crate::db::pool::get_pool;
use serde::Serialize;
use tauri::command;

#[derive(Debug, Serialize)]
pub struct UsageSummary {
    pub model: String,
    pub provider_name: String,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

#[command]
pub async fn get_usage_stats(days: i64) -> Result<Vec<UsageSummary>, String> {
    let pool = get_pool().await;
    let days_param = format!("-{} days", days);
    let rows = sqlx::query_as::<_, DbUsageSummary>(
        "SELECT model, provider_name, SUM(prompt_tokens) as total_prompt_tokens, SUM(completion_tokens) as total_completion_tokens, SUM(total_tokens) as total_tokens, SUM(cost_estimate) as total_cost, SUM(request_count) as request_count FROM usage_stats WHERE bucket_minute >= datetime('now', ?) GROUP BY model, provider_name ORDER BY total_tokens DESC"
    )
    .bind(&days_param)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| UsageSummary {
            model: r.model,
            provider_name: r.provider_name,
            total_prompt_tokens: r.total_prompt_tokens,
            total_completion_tokens: r.total_completion_tokens,
            total_tokens: r.total_tokens,
            total_cost: r.total_cost,
            request_count: r.request_count,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct DbUsageSummary {
    model: String,
    provider_name: String,
    total_prompt_tokens: i64,
    total_completion_tokens: i64,
    total_tokens: i64,
    total_cost: f64,
    request_count: i64,
}
