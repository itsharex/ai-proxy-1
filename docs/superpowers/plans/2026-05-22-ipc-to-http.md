# IPC → HTTP 管理接口迁移实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 17 个 Tauri IPC 命令替换为 axum HTTP `/api/*` 端点，仅保留 1 个 IPC 用于获取服务端地址。

**Architecture:** 在现有 axum 服务器上新增 `/api/*` 管理路由组，handler 复用 IPC 中的业务逻辑。前端通过 `fetch()` 调用，`api()` 客户端封装统一响应格式。仅保留 `get_api_config` IPC 命令传递服务端 URL。

**Tech Stack:** Rust/axum (JSON extractor, Path/Query params), TypeScript/fetch, Vite proxy

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/server/api.rs` | Create | 管理 API 路由注册 + 统一响应包装 |
| `src-tauri/src/server/router.rs` | Modify | 合并 `/api` 路由组 |
| `src-tauri/src/server/mod.rs` | Modify | 导出 api 模块 |
| `src-tauri/src/lib.rs` | Modify | invoke_handler 仅注册 1 个命令，删除 ipc 模块 |
| `src-tauri/src/ipc/*.rs` | Delete | 不再需要 |
| `src-tauri/src/ipc/mod.rs` | Delete | 不再需要 |
| `src/api/index.ts` | Create | API 客户端（baseUrl + fetch 封装） |
| `src/App.vue` | Modify | 启动时调用 initApi() |
| `src/stores/providers.ts` | Modify | invoke → api() |
| `src/views/Dashboard.vue` | Modify | 通过 api() 获取数据 |
| `src/views/Providers.vue` | Modify | invoke → api() |
| `src/views/Models.vue` | Modify | invoke → api() |
| `src/views/Logs.vue` | Modify | invoke → api() |
| `src/views/Rules.vue` | Modify | invoke → api() |
| `src/views/Statistics.vue` | Modify | invoke → api() |
| `src/views/Settings.vue` | Modify | 通过 api() 读写设置 |
| `vite.config.ts` | Modify | 添加开发代理 |

---

### Task 1: Rust 统一响应类型 + API 路由框架

**Files:**
- Create: `src-tauri/src/server/api.rs`

- [ ] **Step 1: 创建 api.rs — 统一响应 + 空路由**

```rust
use axum::Json;
use serde::Serialize;

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

impl ApiError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { success: false, error: msg.into() }
    }
}

pub fn ok<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse { success: true, data })
}

pub fn err(msg: impl Into<String>) -> Json<ApiError> {
    Json(ApiError::new(msg))
}

pub fn api_routes() -> axum::Router {
    axum::Router::new()
}
```

- [ ] **Step 2: 更新 server/mod.rs 导出 api 模块**

在 `src-tauri/src/server/mod.rs` 添加 `pub mod api;`。

- [ ] **Step 3: 更新 router.rs 合并 API 路由**

修改 `src-tauri/src/server/router.rs`：

```rust
use crate::server::handlers;
use crate::server::api;
use axum::Router;
use axum::routing::{post, get};

pub fn create_router() -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handlers::handle_completions))
        .route("/v1/responses", post(handlers::handle_responses))
        .route("/v1/messages", post(handlers::handle_anthropic))
        .route("/v1beta/models/{model}", post(handlers::handle_gemini))
        .route("/health", get(health_check))
        .nest("/api", api::api_routes())
}

async fn health_check() -> &'static str {
    "OK"
}
```

- [ ] **Step 4: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 5: Commit**

```
feat: add API route framework with unified response type
```

---

### Task 2: Rust Provider + Key HTTP handlers

**Files:**
- Modify: `src-tauri/src/server/api.rs`

- [ ] **Step 1: 添加 Provider 和 Key handler**

在 `api.rs` 中添加以下内容（在 `pub fn api_routes()` 之前）：

```rust
use axum::extract::{Path, Json as AxumJson};
use crate::db::get_pool;
use crate::provider::endpoint::{Provider, ApiKeyInfo};
use crate::provider::manager::ProviderManager;
use crate::key::store::encrypt_api_key;
use serde::Deserialize;

// --- Provider handlers ---

async fn list_providers() -> axum::Json<ApiResponse<Vec<Provider>>> {
    match ProviderManager::list().await {
        Ok(providers) => ok(providers),
        Err(e) => { tracing::error!("list_providers error: {}", e); ok(vec![]) }
    }
}

#[derive(Deserialize)]
struct CreateProviderBody {
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: String,
    endpoints: Vec<EndpointInput>,
}

#[derive(Deserialize)]
struct EndpointInput {
    format: String,
    path: String,
}

async fn create_provider(
    AxumJson(body): AxumJson<CreateProviderBody>,
) -> Result<axum::Json<ApiResponse<String>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO providers (id, name, base_url, auth_type, auth_header) VALUES (?, ?, ?, ?, ?)")
        .bind(&id).bind(&body.name).bind(&body.base_url).bind(&body.auth_type).bind(&body.auth_header)
        .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    for ep in &body.endpoints {
        let ep_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO endpoints (id, provider_id, format, path) VALUES (?, ?, ?, ?)")
            .bind(&ep_id).bind(&id).bind(&ep.format).bind(&ep.path)
            .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
    }

    Ok(ok(id))
}

#[derive(Deserialize)]
struct UpdateProviderBody {
    name: String,
    base_url: String,
}

async fn update_provider(
    Path(id): Path<String>,
    AxumJson(body): AxumJson<UpdateProviderBody>,
) -> Result<axum::Json<ApiResponse<()>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    sqlx::query("UPDATE providers SET name = ?, base_url = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&body.name).bind(&body.base_url).bind(&id)
        .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
    Ok(ok(()))
}

async fn delete_provider(
    Path(id): Path<String>,
) -> Result<axum::Json<ApiResponse<()>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(&id)
        .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
    Ok(ok(()))
}

// --- Key handlers ---

async fn list_keys(
    Path(provider_id): Path<String>,
) -> Result<axum::Json<ApiResponse<Vec<ApiKeyInfo>>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let rows = sqlx::query_as::<_, (String, String, i64, i64, Option<String>, String)>(
        "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ? ORDER BY created_at DESC"
    )
    .bind(&provider_id)
    .fetch_all(pool)
    .await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    Ok(ok(rows.into_iter().map(|(id, label, is_active, usage_count, last_used_at, created_at)| ApiKeyInfo {
        id, label, is_active: is_active != 0, usage_count, last_used_at, created_at,
    }).collect()))
}

#[derive(Deserialize)]
struct CreateKeyBody {
    label: String,
    plaintext_key: String,
}

async fn create_key(
    Path(provider_id): Path<String>,
    AxumJson(body): AxumJson<CreateKeyBody>,
) -> Result<axum::Json<ApiResponse<String>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let (encrypted, nonce) = encrypt_api_key(&body.plaintext_key).map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO api_keys (id, provider_id, label, encrypted_key, nonce) VALUES (?, ?, ?, ?, ?)")
        .bind(&id).bind(&provider_id).bind(&body.label).bind(&encrypted).bind(&nonce.as_slice())
        .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    Ok(ok(id))
}

async fn delete_key(
    Path(id): Path<String>,
) -> Result<axum::Json<ApiResponse<()>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM api_keys WHERE id = ?")
        .bind(&id)
        .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
    Ok(ok(()))
}
```

更新 `api_routes()` 添加路由：

```rust
pub fn api_routes() -> axum::Router {
    axum::Router::new()
        .route("/providers", get(list_providers).post(create_provider))
        .route("/providers/{id}", axum::routing::put(update_provider).delete(delete_provider))
        .route("/providers/{id}/keys", get(list_keys).post(create_key))
        .route("/keys/{id}", axum::routing::delete(delete_key))
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```
feat: add provider and API key HTTP endpoints
```

---

### Task 3: Rust Route + Log + Usage + Rule + Settings handlers

**Files:**
- Modify: `src-tauri/src/server/api.rs`

- [ ] **Step 1: 添加 Route handler**

在 `api.rs` 添加路由相关的 handler。在文件末尾 `api_routes()` 之前追加：

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::interceptor::rules::InterceptorRule;

// --- Route handlers ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelRoute {
    id: String,
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: i64,
}

async fn list_routes() -> Result<axum::Json<ApiResponse<Vec<ModelRoute>>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let rows: Vec<(String, String, Option<String>, String, String, String, Option<String>, i64)> = sqlx::query_as(
        "SELECT id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority FROM model_routes ORDER BY priority DESC",
    )
    .fetch_all(pool)
    .await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    Ok(ok(rows.into_iter().map(|(id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority)| {
        ModelRoute { id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority }
    }).collect()))
}

#[derive(Deserialize)]
struct CreateRouteBody {
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: Option<i64>,
}

async fn create_route(
    AxumJson(body): AxumJson<CreateRouteBody>,
) -> Result<axum::Json<ApiResponse<ModelRoute>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let id = Uuid::new_v4().to_string();
    let priority = body.priority.unwrap_or(0);

    sqlx::query(
        "INSERT INTO model_routes (id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id).bind(&body.model_pattern).bind(&body.alias).bind(&body.provider_id)
    .bind(&body.target_model).bind(&body.target_format).bind(&body.fallback_provider_id).bind(priority)
    .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    Ok(ok(ModelRoute {
        id, model_pattern: body.model_pattern, alias: body.alias, provider_id: body.provider_id,
        target_model: body.target_model, target_format: body.target_format,
        fallback_provider_id: body.fallback_provider_id, priority,
    }))
}

async fn delete_route(
    Path(id): Path<String>,
) -> Result<axum::Json<ApiResponse<()>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM model_routes WHERE id = ?")
        .bind(&id)
        .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
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
    duration_ms: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
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
    axum::extract::Query(query): axum::extract::Query<LogQuery>,
) -> Result<axum::Json<ApiResponse<LogList>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let offset = (query.page - 1).max(0) * query.limit;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM request_logs")
        .fetch_one(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    let rows: Vec<(i64, String, String, String, String, String, bool, i64, i64, i64, i64, Option<String>, String)> = sqlx::query_as(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs ORDER BY id DESC LIMIT ? OFFSET ?",
    )
    .bind(query.limit).bind(offset)
    .fetch_all(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    let logs = rows.into_iter().map(|(id, request_id, client_format, provider_name, provider_format, model, stream, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at)| {
        LogEntry { id, request_id, client_format, provider_name, provider_format, model, stream, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at }
    }).collect();

    Ok(ok(LogList { logs, total: total.0 }))
}

async fn get_log(
    Path(id): Path<i64>,
) -> Result<axum::Json<ApiResponse<LogEntry>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let row: (i64, String, String, String, String, String, bool, i64, i64, i64, i64, Option<String>, String) = sqlx::query_as(
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    Ok(ok(LogEntry { id: row.0, request_id: row.1, client_format: row.2, provider_name: row.3, provider_format: row.4, model: row.5, stream: row.6, duration_ms: row.7, prompt_tokens: row.8, completion_tokens: row.9, total_tokens: row.10, error_message: row.11, created_at: row.12 }))
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
    axum::extract::Query(query): axum::extract::Query<UsageQuery>,
) -> Result<axum::Json<ApiResponse<UsageSummary>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let rows: Vec<(String, String, i64, i64, i64, f64, i64)> = sqlx::query_as(
        "SELECT model, provider_name, SUM(prompt_tokens), SUM(completion_tokens), SUM(total_tokens), SUM(cost_estimate), SUM(request_count) FROM usage_stats WHERE bucket_minute >= datetime('now', ? || ' days') GROUP BY model, provider_name ORDER BY SUM(total_tokens) DESC",
    )
    .bind(format!("-{}", query.days))
    .fetch_all(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

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

async fn list_rules() -> Result<axum::Json<ApiResponse<Vec<InterceptorRule>>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let rows: Vec<(String, String, String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT id, name, phase, rule_type, condition_json, action_json, priority, enabled FROM interceptor_rules ORDER BY priority DESC",
    )
    .fetch_all(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

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
    AxumJson(body): AxumJson<CreateRuleBody>,
) -> Result<axum::Json<ApiResponse<InterceptorRule>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let id = Uuid::new_v4().to_string();
    let priority = body.priority.unwrap_or(0);
    let enabled = body.enabled.unwrap_or(true) as i32;

    let condition_json = serde_json::to_string(&body.condition).map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
    let action_json = serde_json::to_string(&body.action).map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    sqlx::query(
        "INSERT INTO interceptor_rules (id, name, phase, rule_type, condition_json, action_json, priority, enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id).bind(&body.name).bind(&body.phase).bind("custom")
    .bind(&condition_json).bind(&action_json).bind(priority).bind(enabled)
    .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

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
    AxumJson(body): AxumJson<UpdateRuleBody>,
) -> Result<axum::Json<ApiResponse<()>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let current: (String, String, String, String, i64, i64) = sqlx::query_as(
        "SELECT name, phase, condition_json, action_json, priority, enabled FROM interceptor_rules WHERE id = ?",
    ).bind(&id).fetch_one(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    let name = body.name.unwrap_or(current.0);
    let phase = body.phase.unwrap_or(current.1);
    let condition_json = body.condition.map(|c| serde_json::to_string(&c).unwrap_or_default()).unwrap_or(current.2);
    let action_json = body.action.map(|a| serde_json::to_string(&a).unwrap_or_default()).unwrap_or(current.3);
    let priority = body.priority.unwrap_or(current.4);
    let enabled = body.enabled.map(|e| e as i32).unwrap_or(current.5 as i32);

    sqlx::query(
        "UPDATE interceptor_rules SET name = ?, phase = ?, condition_json = ?, action_json = ?, priority = ?, enabled = ? WHERE id = ?",
    ).bind(&name).bind(&phase).bind(&condition_json).bind(&action_json).bind(priority).bind(enabled).bind(&id)
    .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    Ok(ok(()))
}

async fn delete_rule(
    Path(id): Path<String>,
) -> Result<axum::Json<ApiResponse<()>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    sqlx::query("DELETE FROM interceptor_rules WHERE id = ?")
        .bind(&id)
        .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
    Ok(ok(()))
}

// --- Settings handlers ---

use std::collections::HashMap;

#[derive(Serialize)]
struct Settings {
    http_host: String,
    http_port: String,
    log_retention_days: String,
    log_request_body: String,
    require_auth: String,
}

async fn get_settings() -> Result<axum::Json<ApiResponse<Settings>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE key IN ('http_host', 'http_port', 'log_retention_days', 'log_request_body', 'require_auth')"
    ).fetch_all(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;

    let map: HashMap<String, String> = rows.into_iter().collect();
    Ok(ok(Settings {
        http_host: map.get("http_host").cloned().unwrap_or_else(|| "127.0.0.1".into()),
        http_port: map.get("http_port").cloned().unwrap_or_else(|| "7860".into()),
        log_retention_days: map.get("log_retention_days").cloned().unwrap_or_else(|| "30".into()),
        log_request_body: map.get("log_request_body").cloned().unwrap_or_else(|| "false".into()),
        require_auth: map.get("require_auth").cloned().unwrap_or_else(|| "false".into()),
    }))
}

#[derive(Deserialize)]
struct UpdateSettingsBody {
    http_host: Option<String>,
    http_port: Option<String>,
    log_retention_days: Option<String>,
    log_request_body: Option<String>,
    require_auth: Option<String>,
}

async fn update_settings(
    AxumJson(body): AxumJson<UpdateSettingsBody>,
) -> Result<axum::Json<ApiResponse<()>>, axum::Json<ApiError>> {
    let pool = get_pool().await;
    let updates = [
        ("http_host", body.http_host),
        ("http_port", body.http_port),
        ("log_retention_days", body.log_retention_days),
        ("log_request_body", body.log_request_body),
        ("require_auth", body.require_auth),
    ];
    for (key, value) in updates {
        if let Some(v) = value {
            sqlx::query("UPDATE settings SET value = ? WHERE key = ?")
                .bind(&v).bind(key)
                .execute(pool).await.map_err(|e| axum::Json(ApiError::new(e.to_string())))?;
        }
    }
    Ok(ok(()))
}
```

更新 `api_routes()`：

```rust
pub fn api_routes() -> axum::Router {
    axum::Router::new()
        .route("/providers", get(list_providers).post(create_provider))
        .route("/providers/{id}", axum::routing::put(update_provider).delete(delete_provider))
        .route("/providers/{id}/keys", get(list_keys).post(create_key))
        .route("/keys/{id}", axum::routing::delete(delete_key))
        .route("/routes", get(list_routes).post(create_route))
        .route("/routes/{id}", axum::routing::delete(delete_route))
        .route("/logs", get(list_logs))
        .route("/logs/{id}", get(get_log))
        .route("/usage", get(get_usage))
        .route("/rules", get(list_rules).post(create_rule))
        .route("/rules/{id}", axum::routing::put(update_rule).delete(delete_rule))
        .route("/settings", get(get_settings).put(update_settings))
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过（可能有 unused import 警告，清理即可）

- [ ] **Step 3: Commit**

```
feat: add all management HTTP endpoints (routes, logs, usage, rules, settings)
```

---

### Task 4: 清理 IPC + 更新 lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Delete: `src-tauri/src/ipc/` 目录

- [ ] **Step 1: 更新 lib.rs — 删除 ipc 模块，仅保留 get_api_config**

```rust
pub mod converter;
mod db;
mod error;
mod provider;
mod key;
mod routing;
mod interceptor;
mod usage;
mod logging;
mod server;

use tauri::Manager;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static APP_RUNTIME: Lazy<Mutex<Option<tokio::runtime::Runtime>>> = Lazy::new(|| Mutex::new(None));

#[tauri::command]
fn get_api_config() -> String {
    let pool = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async { db::get_pool().await })
    });
    let host: String = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_host'")
                .fetch_one(pool)
                .await
                .unwrap_or_else(|_| "127.0.0.1".to_string())
        })
    });
    let port: String = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_port'")
                .fetch_one(pool)
                .await
                .unwrap_or_else(|_| "7860".to_string())
        })
    });
    format!("http://{}:{}", host, port)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("ai-proxy.db");
            let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
            rt.block_on(async {
                db::init::init_db(db_path.to_str().unwrap()).await
                    .expect("failed to initialize database");
            });

            let rt_clone = rt.clone();
            rt.spawn(async move {
                let pool = db::get_pool().await;
                let host: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_host'")
                    .fetch_one(pool).await.unwrap_or_else(|_| "127.0.0.1".to_string());
                let pool = db::get_pool().await;
                let port_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_port'")
                    .fetch_one(pool).await.unwrap_or_else(|_| "7860".to_string());
                let port: u16 = port_str.parse().unwrap_or(7860);
                server::start_server(&host, port).await;
            });

            {
                let mut guard = APP_RUNTIME.lock().unwrap();
                *guard = Some(rt);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_api_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: 删除 ipc 目录**

```bash
rm -rf src-tauri/src/ipc/
```

- [ ] **Step 3: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 4: Commit**

```
refactor: remove IPC module, keep only get_api_config command
```

---

### Task 5: 前端 API 客户端 + Vite 代理

**Files:**
- Create: `src/api/index.ts`
- Modify: `vite.config.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: 创建 API 客户端**

```typescript
// src/api/index.ts
import { invoke } from '@tauri-apps/api/core'

let baseUrl = ''

export async function initApi(): Promise<void> {
  baseUrl = await invoke<string>('get_api_config')
  const res = await fetch(`${baseUrl}/health`)
  if (!res.ok) throw new Error('Proxy server not reachable')
}

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${baseUrl}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  })
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: `HTTP ${res.status}` }))
    throw new Error(body.error || `API error: ${res.status}`)
  }
  const body = await res.json()
  if (!body.success) throw new Error(body.error || 'Unknown error')
  return body.data as T
}

export function getBaseUrl(): string {
  return baseUrl
}
```

- [ ] **Step 2: 更新 vite.config.ts 添加开发代理**

在 `vite.config.ts` 的 `server` 配置中添加 proxy：

```typescript
server: {
  port: 1420,
  strictPort: true,
  host: host || false,
  proxy: {
    '/api': 'http://127.0.0.1:7860',
    '/health': 'http://127.0.0.1:7860',
  },
  // ...hmr, watch 保持不变
}
```

- [ ] **Step 3: 更新 App.vue 启动时调用 initApi**

在 App.vue 的 `<script setup>` 中添加：

```typescript
import { initApi } from './api'

onMounted(async () => {
  try {
    await initApi()
    serverRunning.value = true
  } catch {
    serverRunning.value = false
  }
})
```

在 import 中添加 `onMounted`。

- [ ] **Step 4: TypeScript 编译验证**

Run: `npx vue-tsc --noEmit`
Expected: 通过

- [ ] **Step 5: Commit**

```
feat: add frontend API client with fetch, Vite proxy, and startup init
```

---

### Task 6: 前端页面迁移 — Providers store + Dashboard + Settings

**Files:**
- Modify: `src/stores/providers.ts`
- Modify: `src/views/Dashboard.vue`
- Modify: `src/views/Settings.vue`

- [ ] **Step 1: 替换 providers store**

```typescript
// src/stores/providers.ts
import { defineStore } from 'pinia'
import { api } from '../api'
import type { Provider } from '../types'

export const useProvidersStore = defineStore('providers', {
  state: () => ({
    providers: [] as Provider[],
    loading: false,
  }),
  actions: {
    async fetchProviders() {
      this.loading = true
      try {
        this.providers = await api<Provider[]>('/api/providers')
      } finally {
        this.loading = false
      }
    },
    async createProvider(data: {
      name: string
      base_url: string
      auth_type: string
      auth_header: string
      endpoints: { format: string; path: string }[]
    }) {
      await api('/api/providers', {
        method: 'POST',
        body: JSON.stringify(data),
      })
      await this.fetchProviders()
    },
    async deleteProvider(id: string) {
      await api(`/api/providers/${id}`, { method: 'DELETE' })
      await this.fetchProviders()
    },
    async createApiKey(providerId: string, label: string, plaintextKey: string) {
      await api(`/api/providers/${providerId}/keys`, {
        method: 'POST',
        body: JSON.stringify({ label, plaintext_key: plaintextKey }),
      })
      await this.fetchProviders()
    },
    async deleteApiKey(id: string) {
      await api(`/api/keys/${id}`, { method: 'DELETE' })
      await this.fetchProviders()
    },
  },
})
```

- [ ] **Step 2: 更新 Dashboard.vue 使用 api()**

Dashboard 当前是硬编码数据。添加 API 调用：

```typescript
import { ref, h, onMounted } from 'vue'
import { api } from '../api'
// ... existing imports ...

// 替换硬编码:
// const serverRunning = ref(true)
// const proxyPort = ref(7860)
// ... 在 onMounted 中:

onMounted(async () => {
  try {
    interface DashboardData {
      provider_count: number
      route_count: number
      today_requests: number
      today_tokens: number
      recent_logs: Record<string, unknown>[]
    }
    // 先简单用独立接口
    const providers = await api<Provider[]>('/api/providers')
    providerCount.value = providers.length
    const routes = await api<unknown[]>('/api/routes')
    routeCount.value = routes.length
  } catch {
    // silently fail
  }
})
```

- [ ] **Step 3: 更新 Settings.vue 通过 API 读写**

```typescript
import { api, initApi } from '../api'

// loadSettings 改为:
async function loadSettings() {
  try {
    const data = await api<{
      http_host: string
      http_port: string
      log_retention_days: string
      log_request_body: string
      require_auth: string
    }>('/api/settings')
    settings.value = {
      host: data.http_host,
      port: parseInt(data.http_port) || 7860,
      logRetentionDays: parseInt(data.log_retention_days) || 30,
      logRequestBody: data.log_request_body === 'true',
      requireAuth: data.require_auth === 'true',
    }
  } catch (error) {
    console.error('Failed to load settings:', error)
  }
}

// handleSave 改为:
async function handleSave() {
  try {
    await api('/api/settings', {
      method: 'PUT',
      body: JSON.stringify({
        http_host: settings.value.host,
        http_port: String(settings.value.port),
        log_retention_days: String(settings.value.logRetentionDays),
        log_request_body: String(settings.value.logRequestBody),
        require_auth: String(settings.value.requireAuth),
      }),
    })
    await initApi()
    message.success('设置已保存')
  } catch (error) {
    message.error(`保存失败: ${error}`)
  }
}
```

- [ ] **Step 4: TypeScript 编译验证**

Run: `npx vue-tsc --noEmit`
Expected: 通过

- [ ] **Step 5: Commit**

```
refactor: migrate providers store, dashboard, and settings to HTTP API
```

---

### Task 7: 前端页面迁移 — Logs, Models, Rules, Statistics

**Files:**
- Modify: `src/views/Logs.vue`
- Modify: `src/views/Models.vue`
- Modify: `src/views/Rules.vue`
- Modify: `src/views/Statistics.vue`

- [ ] **Step 1: 更新 Logs.vue**

替换 `invoke` 为 `api`：

- `import { invoke }` → `import { api } from '../api'`
- `invoke<RequestLog[]>('get_logs', { page, limit })` → `api<{ logs: RequestLog[]; total: number }>('/api/logs?page=' + currentPage.value + '&limit=' + pageSize)`

fetchLogs 函数改为：

```typescript
async function fetchLogs() {
  loading.value = true
  try {
    const result = await api<{ logs: RequestLog[]; total: number }>(
      `/api/logs?page=${currentPage.value}&limit=${pageSize}`
    )
    logs.value = result.logs
  } catch (error) {
    console.error('Failed to load logs:', error)
  } finally {
    loading.value = false
  }
}
```

- [ ] **Step 2: 更新 Models.vue**

- `import { invoke }` → `import { api } from '../api'`
- `invoke<ModelRoute[]>('get_routes')` → `api<ModelRoute[]>('/api/routes')`
- `invoke<Provider[]>('get_providers')` → `api<Provider[]>('/api/providers')`
- `invoke('create_route', { ... })` → `api('/api/routes', { method: 'POST', body: JSON.stringify({ ... }) })`
- `invoke('delete_route', { id })` → `api('/api/routes/' + id, { method: 'DELETE' })`

- [ ] **Step 3: 更新 Rules.vue**

- `import { invoke }` → `import { api } from '../api'`
- `invoke<InterceptorRule[]>('get_rules')` → `api<InterceptorRule[]>('/api/rules')`
- `invoke('create_rule', { ... })` → `api('/api/rules', { method: 'POST', body: JSON.stringify({ name, phase, condition, action }) })`
- `invoke('update_rule', { id, enabled })` → `api('/api/rules/' + id, { method: 'PUT', body: JSON.stringify({ enabled }) })`
- `invoke('delete_rule', { id })` → `api('/api/rules/' + id, { method: 'DELETE' })`

- [ ] **Step 4: 更新 Statistics.vue**

- `import { invoke }` → `import { api } from '../api'`
- `invoke<UsageSummary[]>('get_usage_stats', { days })` → `api('/api/usage?days=' + days)`

返回类型需要适配：后端返回 `{ stats, total_cost, total_requests }`，前端之前期望的是 `UsageSummary[]`。需要更新 Vue 中的数据处理。

- [ ] **Step 5: TypeScript 编译验证**

Run: `npx vue-tsc --noEmit`
Expected: 通过

- [ ] **Step 6: Commit**

```
refactor: migrate logs, models, rules, and statistics views to HTTP API
```

---

### Task 8: 集成验证

**Files:**
- All

- [ ] **Step 1: Rust 编译检查**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 零错误，零警告

- [ ] **Step 2: TypeScript 编译检查**

Run: `npx vue-tsc --noEmit`
Expected: 通过

- [ ] **Step 3: 启动 Tauri 应用**

Run: `cargo tauri dev`

验证：
1. 应用窗口正常显示
2. 打开浏览器 DevTools，Network 面板能看到 `/api/*` 请求
3. 各页面加载正常，无卡死
4. `/health` 返回 OK
5. 供应商页面、模型路由页面、请求日志页面、拦截规则页面都能正常显示

- [ ] **Step 4: Commit**

```
test: verify IPC to HTTP migration works end-to-end
```
