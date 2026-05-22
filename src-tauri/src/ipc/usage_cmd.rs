use crate::db::get_pool;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UsageStat {
    pub model: String,
    pub provider_name: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost_estimate: f64,
    pub request_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub stats: Vec<UsageStat>,
    pub total_cost: f64,
    pub total_requests: i64,
}

#[tauri::command]
pub async fn get_usage_stats(days: i64) -> Result<UsageSummary, String> {
    let pool = get_pool().await;

    let rows: Vec<(String, String, i64, i64, i64, f64, i64)> = sqlx::query_as(
        "SELECT model, provider_name, SUM(prompt_tokens), SUM(completion_tokens), SUM(total_tokens), SUM(cost_estimate), SUM(request_count) FROM usage_stats WHERE bucket_minute >= datetime('now', ? || ' days') GROUP BY model, provider_name ORDER BY SUM(total_tokens) DESC",
    )
    .bind(format!("-{}", days))
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut total_cost = 0.0;
    let mut total_requests = 0i64;

    let stats: Vec<UsageStat> = rows
        .into_iter()
        .map(|(model, provider_name, prompt_tokens, completion_tokens, total_tokens, cost_estimate, request_count)| {
            total_cost += cost_estimate;
            total_requests += request_count;
            UsageStat {
                model,
                provider_name,
                prompt_tokens,
                completion_tokens,
                total_tokens,
                cost_estimate,
                request_count,
            }
        })
        .collect();

    Ok(UsageSummary {
        stats,
        total_cost,
        total_requests,
    })
}
