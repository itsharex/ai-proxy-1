use crate::converter::ir::IrUsage;
use crate::db::pool::get_pool;

pub struct UsageTracker;

impl UsageTracker {
    pub async fn record(
        model: &str,
        provider_name: &str,
        usage: &IrUsage,
    ) -> Result<(), sqlx::Error> {
        let pool = get_pool().await;
        let now = chrono::Utc::now();
        let bucket = now.format("%Y-%m-%d %H:%M:00").to_string();
        let cost = (usage.prompt_tokens as f64 * 0.003
            + usage.completion_tokens as f64 * 0.006)
            / 1000.0;

        sqlx::query(
            "INSERT INTO usage_stats (model, provider_name, prompt_tokens, completion_tokens, total_tokens, cost_estimate, request_count, bucket_minute) VALUES (?, ?, ?, ?, ?, ?, 1, ?)",
        )
        .bind(model)
        .bind(provider_name)
        .bind(usage.prompt_tokens)
        .bind(usage.completion_tokens)
        .bind(usage.total_tokens)
        .bind(cost)
        .bind(&bucket)
        .execute(pool)
        .await?;

        Ok(())
    }
}
