use crate::db::get_pool;
use crate::error::ProxyError;
use super::pricing::PricingTable;

pub struct UsageTracker;

impl UsageTracker {
    pub async fn record(model: &str, provider_name: &str, prompt_tokens: i64, completion_tokens: i64) -> Result<(), ProxyError> {
        let pool = get_pool().await;
        let now = chrono::Utc::now();
        let bucket = now.format("%Y-%m-%d %H:%M:00").to_string();
        let total = prompt_tokens + completion_tokens;
        let pricing = PricingTable::default();
        let cost = pricing.get_cost(model, prompt_tokens as u32, completion_tokens as u32);

        sqlx::query(
            "INSERT INTO usage_stats (model, provider_name, prompt_tokens, completion_tokens, total_tokens, cost_estimate, request_count, bucket_minute) VALUES (?, ?, ?, ?, ?, ?, 1, ?)"
        )
        .bind(model).bind(provider_name).bind(prompt_tokens).bind(completion_tokens)
        .bind(total).bind(cost).bind(&bucket)
        .execute(pool).await?;

        Ok(())
    }
}
