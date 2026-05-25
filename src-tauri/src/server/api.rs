use axum::Json;
use axum::extract::{Path, Query};
use axum::routing;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::get_pool;
use crate::key::store::encrypt_api_key;
use crate::provider::endpoint::Provider;
use crate::provider::manager::ProviderManager;

// --- Unified response types ---

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: T,
}

#[derive(Serialize)]
pub struct ApiError {
    pub success: bool,
    pub error: String,
}

pub fn ok<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse { success: true, data })
}

fn err_json(msg: impl Into<String>) -> Json<ApiError> {
    Json(ApiError { success: false, error: msg.into() })
}

// --- Provider handlers ---

async fn list_providers() -> Json<ApiResponse<Vec<Provider>>> {
    match ProviderManager::list().await {
        Ok(providers) => ok(providers),
        Err(e) => {
            tracing::error!("list_providers error: {}", e);
            ok(vec![])
        }
    }
}

#[derive(Deserialize)]
struct CreateProviderBody {
    name: String,
    base_url: String,
    format: String,
    api_key: String,
    models: Vec<ModelInput>,
}

#[derive(Deserialize)]
struct ModelInput {
    model_name: String,
    target_model: Option<String>,
}

async fn create_provider(
    axum::Json(body): axum::Json<CreateProviderBody>,
) -> Result<Json<ApiResponse<String>>, Json<ApiError>> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO providers (id, name, base_url, format) VALUES (?, ?, ?, ?)")
        .bind(&id).bind(&body.name).bind(&body.base_url).bind(&body.format)
        .execute(pool).await.map_err(|e| err_json(e.to_string()))?;

    for m in &body.models {
        let model_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO provider_models (id, provider_id, model_name, target_model) VALUES (?, ?, ?, ?)")
            .bind(&model_id).bind(&id).bind(&m.model_name).bind(&m.target_model)
            .execute(pool).await.map_err(|e| err_json(e.to_string()))?;
    }

    let (encrypted, nonce) = encrypt_api_key(&body.api_key).map_err(|e| err_json(e.to_string()))?;
    let key_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO api_keys (id, provider_id, label, encrypted_key, nonce) VALUES (?, ?, ?, ?, ?)")
        .bind(&key_id).bind(&id).bind(&body.name).bind(&encrypted).bind(&nonce.as_slice())
        .execute(pool).await.map_err(|e| err_json(e.to_string()))?;

    Ok(ok(id))
}

#[derive(Deserialize)]
struct UpdateProviderBody {
    name: Option<String>,
    base_url: Option<String>,
    format: Option<String>,
    api_key: Option<String>,
    models: Option<Vec<ModelInput>>,
}

async fn update_provider(
    Path(id): Path<String>,
    axum::Json(body): axum::Json<UpdateProviderBody>,
) -> Result<Json<ApiResponse<()>>, Json<ApiError>> {
    let pool = get_pool().await;

    let current: (String, String, String) = sqlx::query_as(
        "SELECT name, base_url, format FROM providers WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    let name = body.name.unwrap_or(current.0);
    let base_url = body.base_url.unwrap_or(current.1);
    let format = body.format.unwrap_or(current.2);

    sqlx::query("UPDATE providers SET name = ?, base_url = ?, format = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&name).bind(&base_url).bind(&format).bind(&id)
        .execute(pool).await.map_err(|e| err_json(e.to_string()))?;

    if let Some(models) = body.models {
        sqlx::query("DELETE FROM provider_models WHERE provider_id = ?")
            .bind(&id)
            .execute(pool).await.map_err(|e| err_json(e.to_string()))?;

        for m in &models {
            let model_id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO provider_models (id, provider_id, model_name, target_model) VALUES (?, ?, ?, ?)")
                .bind(&model_id).bind(&id).bind(&m.model_name).bind(&m.target_model)
                .execute(pool).await.map_err(|e| err_json(e.to_string()))?;
        }
    }

    if let Some(ref plaintext_key) = body.api_key {
        if !plaintext_key.is_empty() {
            let (encrypted, nonce) = encrypt_api_key(plaintext_key).map_err(|e| err_json(e.to_string()))?;
            sqlx::query("UPDATE api_keys SET encrypted_key = ?, nonce = ?, label = ? WHERE provider_id = ?")
                .bind(&encrypted).bind(&nonce.as_slice()).bind(&name).bind(&id)
                .execute(pool).await.map_err(|e| err_json(e.to_string()))?;
        }
    }

    Ok(ok(()))
}

async fn delete_provider(
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, Json<ApiError>> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(&id)
        .execute(pool).await.map_err(|e| err_json(e.to_string()))?;
    Ok(ok(()))
}

// --- Log handlers ---

#[derive(Debug, Clone, Serialize)]
struct LogEntry {
    id: i64,
    request_id: String,
    client_format: String,
    provider_name: String,
    provider_format: String,
    model: String,
    stream: bool,
    status_code: Option<i64>,
    duration_ms: Option<i64>,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cached_tokens: i64,
    ttft_ms: Option<i64>,
    error_message: Option<String>,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 20 }

#[derive(Serialize)]
struct LogList {
    logs: Vec<LogEntry>,
    total: i64,
}

async fn list_logs(
    Query(query): Query<LogQuery>,
) -> Result<Json<ApiResponse<LogList>>, Json<ApiError>> {
    let pool = get_pool().await;
    let offset = (query.page - 1).max(0) * query.limit;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM request_logs")
        .fetch_one(pool).await.map_err(|e| err_json(e.to_string()))?;

    let rows: Vec<(i64, String, String, String, String, String, i32, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, Option<i64>, Option<i64>, String)> = sqlx::query_as(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, cached_tokens, ttft_ms, created_at FROM request_logs ORDER BY id DESC LIMIT ? OFFSET ?",
    )
    .bind(query.limit).bind(offset)
    .fetch_all(pool).await.map_err(|e| err_json(e.to_string()))?;

    let logs = rows.into_iter().map(|(id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, cached_tokens, ttft_ms, created_at)| {
        LogEntry {
            id, request_id, client_format, provider_name, provider_format, model,
            stream: stream != 0, status_code, duration_ms,
            prompt_tokens: prompt_tokens.unwrap_or(0),
            completion_tokens: completion_tokens.unwrap_or(0),
            total_tokens: total_tokens.unwrap_or(0),
            cached_tokens: cached_tokens.unwrap_or(0),
            ttft_ms,
            error_message, created_at,
        }
    }).collect();

    Ok(ok(LogList { logs, total: total.0 }))
}

async fn get_log(
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<LogEntry>>, Json<ApiError>> {
    let pool = get_pool().await;
    let row: (i64, String, String, String, String, String, i32, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>, Option<i64>, Option<i64>, String) = sqlx::query_as(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, cached_tokens, ttft_ms, created_at FROM request_logs WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool).await.map_err(|e| err_json(e.to_string()))?;

    Ok(ok(LogEntry {
        id: row.0, request_id: row.1, client_format: row.2, provider_name: row.3,
        provider_format: row.4, model: row.5, stream: row.6 != 0,
        status_code: row.7, duration_ms: row.8,
        prompt_tokens: row.9.unwrap_or(0), completion_tokens: row.10.unwrap_or(0),
        total_tokens: row.11.unwrap_or(0), error_message: row.12,
        cached_tokens: row.13.unwrap_or(0), ttft_ms: row.14, created_at: row.15,
    }))
}

// --- Usage handlers ---

#[derive(Debug, Clone, Serialize)]
struct UsageStat {
    model: String,
    provider_name: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cost_estimate: f64,
    request_count: i64,
}

#[derive(Debug, Clone, Serialize)]
struct UsageSummary {
    stats: Vec<UsageStat>,
    total_cost: f64,
    total_requests: i64,
}

#[derive(Deserialize)]
struct UsageQuery {
    #[serde(default = "default_days")]
    days: i64,
}

fn default_days() -> i64 { 7 }

async fn get_usage(
    Query(query): Query<UsageQuery>,
) -> Result<Json<ApiResponse<UsageSummary>>, Json<ApiError>> {
    let pool = get_pool().await;
    let rows: Vec<(String, String, i64, i64, i64, f64, i64)> = sqlx::query_as(
        "SELECT model, provider_name, SUM(prompt_tokens), SUM(completion_tokens), SUM(total_tokens), SUM(cost_estimate), SUM(request_count) FROM usage_stats WHERE bucket_minute >= datetime('now', ? || ' days') GROUP BY model, provider_name ORDER BY SUM(total_tokens) DESC",
    )
    .bind(format!("-{}", query.days))
    .fetch_all(pool).await.map_err(|e| err_json(e.to_string()))?;

    let mut total_cost = 0.0;
    let mut total_requests = 0i64;
    let stats: Vec<UsageStat> = rows.into_iter().map(|(model, provider_name, prompt_tokens, completion_tokens, total_tokens, cost_estimate, request_count)| {
        total_cost += cost_estimate;
        total_requests += request_count;
        UsageStat { model, provider_name, prompt_tokens, completion_tokens, total_tokens, cost_estimate, request_count }
    }).collect();

    Ok(ok(UsageSummary { stats, total_cost, total_requests }))
}

// --- Rule handlers ---

use crate::interceptor::rules::InterceptorRule;

async fn list_rules() -> Result<Json<ApiResponse<Vec<InterceptorRule>>>, Json<ApiError>> {
    let pool = get_pool().await;
    let rows: Vec<(String, String, String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT id, name, phase, rule_type, condition_json, action_json, priority, enabled FROM interceptor_rules ORDER BY priority DESC",
    )
    .fetch_all(pool).await.map_err(|e| err_json(e.to_string()))?;

    use crate::interceptor::rules::{RuleAction, RuleCondition, RulePhase};
    let rules: Vec<InterceptorRule> = rows.into_iter().map(|(id, name, phase, _rule_type, condition_json, action_json, priority, enabled)| {
        let rule_phase = RulePhase::from_str(&phase).unwrap_or(RulePhase::Pre);
        let condition: RuleCondition = serde_json::from_str(&condition_json).unwrap_or(RuleCondition::Always);
        let action: RuleAction = serde_json::from_str(&action_json).unwrap_or(RuleAction::SetHeader { name: "x-no-op".into(), value: "true".into() });
        InterceptorRule { id, name, phase: rule_phase, condition, action, priority, enabled: enabled != 0 }
    }).collect();

    Ok(ok(rules))
}

#[derive(Deserialize)]
struct CreateRuleBody {
    name: String,
    phase: String,
    condition: serde_json::Value,
    action: serde_json::Value,
    priority: Option<i64>,
    enabled: Option<bool>,
}

async fn create_rule(
    axum::Json(body): axum::Json<CreateRuleBody>,
) -> Result<Json<ApiResponse<InterceptorRule>>, Json<ApiError>> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();
    let priority = body.priority.unwrap_or(0);
    let enabled = body.enabled.unwrap_or(true) as i32;

    let condition_json = serde_json::to_string(&body.condition).map_err(|e| err_json(e.to_string()))?;
    let action_json = serde_json::to_string(&body.action).map_err(|e| err_json(e.to_string()))?;

    sqlx::query(
        "INSERT INTO interceptor_rules (id, name, phase, rule_type, condition_json, action_json, priority, enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id).bind(&body.name).bind(&body.phase).bind("custom")
    .bind(&condition_json).bind(&action_json).bind(priority).bind(enabled)
    .execute(pool).await.map_err(|e| err_json(e.to_string()))?;

    use crate::interceptor::rules::{RuleAction, RuleCondition, RulePhase};
    let rule_phase = RulePhase::from_str(&body.phase).unwrap_or(RulePhase::Pre);
    let condition: RuleCondition = serde_json::from_value(body.condition).unwrap_or(RuleCondition::Always);
    let action: RuleAction = serde_json::from_value(body.action).unwrap_or(RuleAction::SetHeader { name: "x-no-op".into(), value: "true".into() });

    Ok(ok(InterceptorRule { id, name: body.name, phase: rule_phase, condition, action, priority, enabled: enabled != 0 }))
}

#[derive(Deserialize)]
struct UpdateRuleBody {
    name: Option<String>,
    phase: Option<String>,
    condition: Option<serde_json::Value>,
    action: Option<serde_json::Value>,
    priority: Option<i64>,
    enabled: Option<bool>,
}

async fn update_rule(
    Path(id): Path<String>,
    axum::Json(body): axum::Json<UpdateRuleBody>,
) -> Result<Json<ApiResponse<()>>, Json<ApiError>> {
    let pool = get_pool().await;
    let current: (String, String, String, String, i64, i64) = sqlx::query_as(
        "SELECT name, phase, condition_json, action_json, priority, enabled FROM interceptor_rules WHERE id = ?",
    ).bind(&id).fetch_one(pool).await.map_err(|e| err_json(e.to_string()))?;

    let name = body.name.unwrap_or(current.0);
    let phase = body.phase.unwrap_or(current.1);
    let condition_json = body.condition.map(|c| serde_json::to_string(&c).unwrap_or_default()).unwrap_or(current.2);
    let action_json = body.action.map(|a| serde_json::to_string(&a).unwrap_or_default()).unwrap_or(current.3);
    let priority = body.priority.unwrap_or(current.4);
    let enabled = body.enabled.map(|e| e as i32).unwrap_or(current.5 as i32);

    sqlx::query(
        "UPDATE interceptor_rules SET name = ?, phase = ?, condition_json = ?, action_json = ?, priority = ?, enabled = ? WHERE id = ?",
    ).bind(&name).bind(&phase).bind(&condition_json).bind(&action_json).bind(priority).bind(enabled).bind(&id)
    .execute(pool).await.map_err(|e| err_json(e.to_string()))?;

    Ok(ok(()))
}

async fn delete_rule(
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, Json<ApiError>> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM interceptor_rules WHERE id = ?")
        .bind(&id)
        .execute(pool).await.map_err(|e| err_json(e.to_string()))?;
    Ok(ok(()))
}

// --- Settings handlers ---

#[derive(Serialize)]
struct Settings {
    http_host: String,
    http_port: String,
    log_retention_days: String,
    record_request_body: String,
    proxy_auth_enabled: String,
    proxy_auth_key: String,
}

async fn get_settings() -> Result<Json<ApiResponse<Settings>>, Json<ApiError>> {
    let pool = get_pool().await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE key IN ('http_host', 'http_port', 'log_retention_days', 'record_request_body', 'proxy_auth_enabled', 'proxy_auth_key')"
    ).fetch_all(pool).await.map_err(|e| err_json(e.to_string()))?;

    let map: HashMap<String, String> = rows.into_iter().collect();
    Ok(ok(Settings {
        http_host: map.get("http_host").cloned().unwrap_or_else(|| "127.0.0.1".into()),
        http_port: map.get("http_port").cloned().unwrap_or_else(|| "7860".into()),
        log_retention_days: map.get("log_retention_days").cloned().unwrap_or_else(|| "30".into()),
        record_request_body: map.get("record_request_body").cloned().unwrap_or_else(|| "false".into()),
        proxy_auth_enabled: map.get("proxy_auth_enabled").cloned().unwrap_or_else(|| "false".into()),
        proxy_auth_key: map.get("proxy_auth_key").cloned().unwrap_or_default(),
    }))
}

#[derive(Deserialize)]
struct UpdateSettingsBody {
    http_host: Option<String>,
    http_port: Option<String>,
    log_retention_days: Option<String>,
    record_request_body: Option<String>,
    proxy_auth_enabled: Option<String>,
    proxy_auth_key: Option<String>,
}

async fn update_settings(
    axum::Json(body): axum::Json<UpdateSettingsBody>,
) -> Result<Json<ApiResponse<()>>, Json<ApiError>> {
    let pool = get_pool().await;
    let updates = [
        ("http_host", body.http_host),
        ("http_port", body.http_port),
        ("log_retention_days", body.log_retention_days),
        ("record_request_body", body.record_request_body),
        ("proxy_auth_enabled", body.proxy_auth_enabled),
        ("proxy_auth_key", body.proxy_auth_key),
    ];
    for (key, value) in updates {
        if let Some(v) = value {
            sqlx::query("UPDATE settings SET value = ? WHERE key = ?")
                .bind(&v).bind(key)
                .execute(pool).await.map_err(|e| err_json(e.to_string()))?;
        }
    }
    Ok(ok(()))
}

// --- Route registration ---

pub fn api_routes() -> axum::Router {
    axum::Router::new()
        .route("/providers", axum::routing::get(list_providers).post(create_provider))
        .route("/providers/:id", routing::put(update_provider).delete(delete_provider))
        .route("/logs", axum::routing::get(list_logs))
        .route("/logs/:id", axum::routing::get(get_log))
        .route("/usage", axum::routing::get(get_usage))
        .route("/rules", axum::routing::get(list_rules).post(create_rule))
        .route("/rules/:id", routing::put(update_rule).delete(delete_rule))
        .route("/settings", axum::routing::get(get_settings).put(update_settings))
}
