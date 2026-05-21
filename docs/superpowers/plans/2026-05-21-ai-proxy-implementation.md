# AI LLM Network Proxy 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一款 Tauri 桌面端 AI LLM 网络代理，支持 OpenAI Completions/Responses、Anthropic Messages、Google Gemini 四种 API 格式互转，含流式 SSE 支持。

**Architecture:** IR 归一化模式 — 所有格式先解析为统一 IrRequest/IrResponse，再生成目标格式。Rust axum HTTP Server 作为代理入口，Vue 3 前端通过 Tauri IPC 管理配置。

**Tech Stack:** Tauri 2.x + Rust (axum/reqwest/sqlx/ring) + Vue 3 (Naive UI/ECharts/Pinia) + SQLite

---

## Phase 0: 项目脚手架

### Task 0.1: 创建 Tauri 项目

**Files:**
- Create: 整个项目骨架

- [ ] **Step 1: 使用 Tauri CLI 创建项目**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
pnpm create tauri-app@latest ai-proxy --template vue-ts
```

选择 Vue + TypeScript 模板。

- [ ] **Step 2: 安装前端依赖**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
pnpm add naive-ui echarts pinia @tauri-apps/api @tauri-apps/plugin-shell
pnpm add -D @types/node
```

- [ ] **Step 3: 配置 Cargo.toml 依赖**

编辑 `src-tauri/Cargo.toml`，添加核心依赖：

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["macros"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
ring = "0.17"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
futures = "0.3"
async-stream = "0.3"
tower-http = { version = "0.5", features = ["cors"] }
hostname = "0.3"
```

- [ ] **Step 4: 配置 tauri.conf.json**

编辑 `src-tauri/tauri.conf.json`：

```json
{
  "$schema": "https://raw.githubusercontent.com/nickelrg/tauri-devtools/main/schemas/tauri.conf.json",
  "productName": "AI Proxy",
  "version": "0.1.0",
  "identifier": "com.aiproxy.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "title": "AI Proxy",
        "width": 1200,
        "height": 800,
        "resizable": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "shell": {
      "open": true
    }
  }
}
```

- [ ] **Step 5: 验证脚手架**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: 编译成功，无错误。

- [ ] **Step 6: Commit**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
git add -A
git commit -m "chore: scaffold Tauri + Vue 3 + Rust project"
```

---

## Phase 1: 基础设施层

### Task 1.1: 数据库层 (SQLite pool + schema)

**Files:**
- Create: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/pool.rs`, `src-tauri/src/db/init.rs`
- Create: `src-tauri/migrations/001_init.sql`

- [ ] **Step 1: 编写数据库迁移 SQL**

创建 `src-tauri/migrations/001_init.sql`：

```sql
-- 供应商配置
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    base_url TEXT NOT NULL,
    auth_type TEXT NOT NULL DEFAULT 'bearer',
    auth_header TEXT NOT NULL DEFAULT 'Authorization',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 供应商端点
CREATE TABLE IF NOT EXISTS endpoints (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    format TEXT NOT NULL CHECK(format IN ('completions', 'responses', 'anthropic', 'gemini')),
    path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 加密的 API Key
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    encrypted_key BLOB NOT NULL,
    nonce BLOB NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    usage_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 模型路由
CREATE TABLE IF NOT EXISTS model_routes (
    id TEXT PRIMARY KEY,
    model_pattern TEXT NOT NULL UNIQUE,
    alias TEXT,
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    target_model TEXT NOT NULL,
    target_format TEXT NOT NULL CHECK(target_format IN ('completions', 'responses', 'anthropic', 'gemini')),
    fallback_provider_id TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 拦截规则
CREATE TABLE IF NOT EXISTS interceptor_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    phase TEXT NOT NULL CHECK(phase IN ('pre', 'post')),
    rule_type TEXT NOT NULL,
    condition_json TEXT NOT NULL DEFAULT '{}',
    action_json TEXT NOT NULL DEFAULT '{}',
    priority INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 请求日志
CREATE TABLE IF NOT EXISTS request_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL,
    client_format TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    provider_format TEXT NOT NULL,
    model TEXT NOT NULL,
    stream INTEGER NOT NULL DEFAULT 0,
    request_body_hash TEXT,
    status_code INTEGER,
    duration_ms INTEGER,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 用量统计汇总 (每分钟聚合)
CREATE TABLE IF NOT EXISTS usage_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    cost_estimate REAL NOT NULL DEFAULT 0.0,
    request_count INTEGER NOT NULL DEFAULT 0,
    bucket_minute TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_stats_bucket ON usage_stats(bucket_minute);
CREATE INDEX IF NOT EXISTS idx_request_logs_request_id ON request_logs(request_id);

-- 应用设置
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('http_host', '127.0.0.1'),
    ('http_port', '7860'),
    ('log_retention_days', '30'),
    ('record_request_body', 'false'),
    ('proxy_auth_enabled', 'false');
```

- [ ] **Step 2: 编写 db/mod.rs**

创建 `src-tauri/src/db/mod.rs`：

```rust
pub mod init;
pub mod pool;

pub use init::init_db;
pub use pool::get_pool;
```

- [ ] **Step 3: 编写 db/pool.rs**

创建 `src-tauri/src/db/pool.rs`：

```rust
use sqlx::sqlite::SqlitePool;
use std::sync::OnceLock;
use tracing::info;

static POOL: OnceLock<SqlitePool> = OnceLock::new();

pub async fn get_pool() -> &'static SqlitePool {
    POOL.get().expect("Database pool not initialized")
}

pub async fn init_pool(path: &str) -> Result<(), sqlx::Error> {
    let pool = SqlitePool::connect(format!("sqlite:{}?mode=rwc", path).as_str()).await?;
    sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
    POOL.set(pool).expect("Database pool already initialized");
    info!("Database pool initialized");
    Ok(())
}
```

- [ ] **Step 4: 编写 db/init.rs**

创建 `src-tauri/src/db/init.rs`：

```rust
use super::pool::{get_pool, init_pool};
use tracing::info;

pub async fn init_db(db_path: &str) -> Result<(), sqlx::Error> {
    init_pool(db_path).await?;
    let pool = get_pool().await;

    let migration = include_str!("../../migrations/001_init.sql");
    sqlx::query(migration).execute(pool).await?;

    info!("Database schema initialized");
    Ok(())
}
```

- [ ] **Step 5: Commit**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
git add -A
git commit -m "feat: add SQLite database layer with schema"
```

### Task 1.2: IR 类型定义

**Files:**
- Create: `src-tauri/src/converter/mod.rs`, `src-tauri/src/converter/ir.rs`

- [ ] **Step 1: 编写 converter/mod.rs**

创建 `src-tauri/src/converter/mod.rs`：

```rust
pub mod ir;
pub mod parsers;
pub mod generators;

use crate::error::ProxyError;
use ir::{IrRequest, IrResponse, IrStreamChunk};

pub trait FormatParser {
    fn parse_request(&self, body: &serde_json::Value) -> Result<IrRequest, ProxyError>;
    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError>;
    fn parse_response(&self, body: &serde_json::Value) -> Result<IrResponse, ProxyError>;
}

pub trait FormatGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<serde_json::Value, ProxyError>;
    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String;
    fn generate_response(&self, ir: &IrResponse) -> Result<serde_json::Value, ProxyError>;
}
```

- [ ] **Step 2: 编写 converter/ir.rs**

创建 `src-tauri/src/converter/ir.rs`：

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrRequest {
    pub model: String,
    pub messages: Vec<IrMessage>,
    pub tools: Option<Vec<IrTool>>,
    pub tool_choice: Option<Value>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub stop_sequences: Option<Vec<String>>,
    pub response_format: Option<Value>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrMessage {
    pub role: IrRole,
    pub content: Vec<IrContentPart>,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Vec<IrToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IrRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IrContentPart {
    Text {
        text: String,
    },
    Image {
        url: Option<String>,
        data: Option<String>,
        media_type: Option<String>,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrStreamChunk {
    pub id: Option<String>,
    pub model: Option<String>,
    pub delta_content: Option<String>,
    pub delta_tool_calls: Option<Vec<IrToolCallDelta>>,
    pub finish_reason: Option<String>,
    pub usage: Option<IrUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrToolCallDelta {
    pub index: u32,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub message: IrMessage,
    pub finish_reason: Option<String>,
    pub usage: IrUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClientFormat {
    Completions,
    Responses,
    Anthropic,
    Gemini,
}

impl ClientFormat {
    pub fn from_path(path: &str) -> Option<Self> {
        if path.contains("/v1/chat/completions") {
            Some(Self::Completions)
        } else if path.contains("/v1/responses") {
            Some(Self::Responses)
        } else if path.contains("/v1/messages") {
            Some(Self::Anthropic)
        } else if path.contains("/v1beta/models") {
            Some(Self::Gemini)
        } else {
            None
        }
    }
}
```

- [ ] **Step 3: 编写 error.rs**

创建 `src-tauri/src/error.rs`：

```rust
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Key management error: {0}")]
    KeyManagement(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProxyError::Parse(m) => (StatusCode::BAD_REQUEST, m.clone()),
            ProxyError::ModelNotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            ProxyError::Provider(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            ProxyError::Network(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            ProxyError::Config(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            ProxyError::KeyManagement(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            ProxyError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = json!({ "error": { "message": message, "type": "proxy_error" } });
        (status, axum::Json(body)).into_response()
    }
}
```

- [ ] **Step 4: 编写 main.rs 基础结构**

编辑 `src-tauri/src/main.rs`，替换内容：

```rust
mod converter;
mod db;
mod error;
mod provider;
mod key;
mod routing;
mod interceptor;
mod usage;
mod logging;
mod server;
mod ipc;

use tracing_subscriber;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("ai-proxy.db");
            let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
            rt.block_on(async {
                db::init::init_db(db_path.to_str().unwrap()).await
                    .expect("failed to initialize database");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::provider_cmd::get_providers,
            ipc::provider_cmd::create_provider,
            ipc::provider_cmd::update_provider,
            ipc::provider_cmd::delete_provider,
            ipc::key_cmd::get_api_keys,
            ipc::key_cmd::create_api_key,
            ipc::key_cmd::delete_api_key,
            ipc::routing_cmd::get_routes,
            ipc::routing_cmd::create_route,
            ipc::routing_cmd::delete_route,
            ipc::log_cmd::get_logs,
            ipc::log_cmd::get_log_detail,
            ipc::usage_cmd::get_usage_stats,
            ipc::interceptor_cmd::get_rules,
            ipc::interceptor_cmd::create_rule,
            ipc::interceptor_cmd::update_rule,
            ipc::interceptor_cmd::delete_rule,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
```

- [ ] **Step 5: Commit**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
git add -A
git commit -m "feat: add core IR types and error handling"
```

---

## Phase 2: 格式转换引擎

### Task 2.1: OpenAI Completions Parser

**Files:**
- Create: `src-tauri/src/converter/parsers/mod.rs`, `src-tauri/src/converter/parsers/completions.rs`

- [ ] **Step 1: 编写 parsers/mod.rs**

```rust
pub mod completions;
pub mod responses;
pub mod anthropic;
pub mod gemini;
```

- [ ] **Step 2: 编写 parsers/completions.rs**

创建 `src-tauri/src/converter/parsers/completions.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct CompletionsParser;

impl FormatParser for CompletionsParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"].as_str()
            .ok_or_else(|| ProxyError::Parse("missing model".into()))?
            .to_string();

        let messages: Vec<Value> = body["messages"].as_array()
            .ok_or_else(|| ProxyError::Parse("missing messages".into()))?
            .clone();

        let ir_messages: Result<Vec<IrMessage>, ProxyError> = messages.iter().map(|m| {
            let role_str = m["role"].as_str().unwrap_or("user");
            let role = match role_str {
                "system" => IrRole::System,
                "user" => IrRole::User,
                "assistant" => IrRole::Assistant,
                "tool" => IrRole::Tool,
                _ => return Err(ProxyError::Parse(format!("unknown role: {}", role_str))),
            };

            let content = match m["content"].is_string() {
                true => vec![IrContentPart::Text { text: m["content"].as_str().unwrap().to_string() }],
                false if m["content"].is_array() => {
                    let parts: Result<Vec<IrContentPart>, ProxyError> = m["content"]
                        .as_array().unwrap().iter().map(|part| {
                            match part["type"].as_str() {
                                Some("text") => Ok(IrContentPart::Text {
                                    text: part.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                }),
                                Some("image_url") => Ok(IrContentPart::Image {
                                    url: part["image_url"].get("url").and_then(|v| v.as_str()).map(String::from),
                                    data: None,
                                    media_type: None,
                                }),
                                other => Err(ProxyError::Parse(format!("unknown content type: {:?}", other))),
                            }
                        }).collect();
                    parts?
                },
                _ => vec![IrContentPart::Text { text: String::new() }],
            };

            let tool_calls = m.get("tool_calls").and_then(|tc| {
                tc.as_array().map(|arr| arr.iter().filter_map(|tc| {
                    Some(IrToolCall {
                        id: tc.get("id")?.as_str()?.to_string(),
                        name: tc.get("function")?.get("name")?.as_str()?.to_string(),
                        arguments: tc.get("function")?.get("arguments")?.as_str()?.to_string(),
                    })
                }).collect::<Vec<_>>())
            });

            let tool_calls = if tool_calls.as_ref().map_or(false, |v| v.is_empty()) {
                None
            } else {
                tool_calls
            };

            Ok(IrMessage {
                role,
                content,
                name: None,
                tool_call_id: m.get("tool_call_id").and_then(|v| v.as_str()).map(String::from),
                tool_calls,
            })
        }).collect();

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| arr.iter().map(|tool| IrTool {
                name: tool.get("function")?.get("name")?.as_str()?.to_string(),
                description: tool.get("function")?.get("description").and_then(|v| v.as_str()).map(String::from),
                input_schema: tool.get("function")?.get("parameters").cloned().unwrap_or(Value::Null),
            }).collect())
        });

        let mut metadata = std::collections::HashMap::new();
        if let Some(user) = body.get("user") {
            metadata.insert("user".into(), user.clone());
        }
        if let Some(seed) = body.get("seed") {
            metadata.insert("seed".into(), seed.clone());
        }

        Ok(IrRequest {
            model,
            messages: ir_messages?,
            tools,
            tool_choice: body.get("tool_choice").cloned(),
            temperature: body.get("temperature").and_then(|v| v.as_f64()).map(|v| v as f32),
            top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32),
            max_tokens: body.get("max_tokens").and_then(|v| v.as_u64()).map(|v| v as u32),
            stream: body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false),
            stop_sequences: body.get("stop").and_then(|v| {
                if v.is_string() { Some(vec![v.as_str()?.to_string()]) }
                else { v.as_array().map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect()) }
            }),
            response_format: body.get("response_format").cloned(),
            metadata,
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data == "[DONE]" {
            return Ok(Some(IrStreamChunk {
                id: None,
                model: None,
                delta_content: None,
                delta_tool_calls: None,
                finish_reason: Some("stop".into()),
                usage: None,
            }));
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let choice = chunk["choices"].get(0);
        let delta = choice.and_then(|c| c.get("delta"));

        let delta_content = delta.and_then(|d| d["content"].as_str()).map(String::from);

        let delta_tool_calls = delta.and_then(|d| d["tool_calls"].as_array()).map(|arr| {
            arr.iter().filter_map(|tc| Some(IrToolCallDelta {
                index: tc.get("index")?.as_u64()? as u32,
                id: tc.get("id").and_then(|v| v.as_str()).map(String::from),
                name: tc.get("function")?.get("name").and_then(|v| v.as_str()).map(String::from),
                arguments: tc.get("function")?.get("arguments").and_then(|v| v.as_str()).map(String::from),
            })).collect()
        });

        let finish_reason = choice.and_then(|c| c["finish_reason"].as_str()).map(String::from);

        Ok(Some(IrStreamChunk {
            id: chunk["id"].as_str().map(String::from),
            model: chunk["model"].as_str().map(String::from),
            delta_content,
            delta_tool_calls,
            finish_reason,
            usage: chunk.get("usage").map(|u| IrUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }),
        }))
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let choice = body["choices"].get(0).ok_or(ProxyError::Parse("no choices".into()))?;
        let msg = &choice["message"];

        let message = IrMessage {
            role: IrRole::Assistant,
            content: vec![IrContentPart::Text {
                text: msg["content"].as_str().unwrap_or("").to_string(),
            }],
            name: None,
            tool_call_id: None,
            tool_calls: msg.get("tool_calls").and_then(|tc| {
                tc.as_array().map(|arr| arr.iter().filter_map(|tc| {
                    Some(IrToolCall {
                        id: tc.get("id")?.as_str()?.to_string(),
                        name: tc.get("function")?.get("name")?.as_str()?.to_string(),
                        arguments: tc.get("function")?.get("arguments")?.as_str()?.to_string(),
                    })
                }).collect::<Vec<_>>())
            }),
        };

        Ok(IrResponse {
            id: body["id"].as_str().map(String::from),
            model: body["model"].as_str().map(String::from),
            message,
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            usage: body.get("usage").map(|u| IrUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }).unwrap_or(IrUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add OpenAI Completions format parser"
```

### Task 2.2: OpenAI Responses Parser

**Files:**
- Create: `src-tauri/src/converter/parsers/responses.rs`

- [ ] **Step 1: 编写 parsers/responses.rs**

创建 `src-tauri/src/converter/parsers/responses.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct ResponsesParser;

impl FormatParser for ResponsesParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"].as_str()
            .ok_or_else(|| ProxyError::Parse("missing model".into()))?
            .to_string();

        let input = body["input"].as_str()
            .ok_or_else(|| ProxyError::Parse("missing input".into()))?;

        let instructions = body["instructions"].as_str();

        // Responses 格式: input 是用户消息, instructions 是 system prompt
        let mut messages = Vec::new();

        if let Some(inst) = instructions {
            messages.push(IrMessage {
                role: IrRole::System,
                content: vec![IrContentPart::Text { text: inst.to_string() }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            });
        }

        messages.push(IrMessage {
            role: IrRole::User,
            content: vec![IrContentPart::Text { text: input.to_string() }],
            name: None,
            tool_call_id: None,
            tool_calls: None,
        });

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| arr.iter().map(|tool| IrTool {
                name: tool.get("name")?.as_str()?.to_string(),
                description: tool.get("description").and_then(|v| v.as_str()).map(String::from),
                input_schema: tool.get("parameters").cloned().unwrap_or(Value::Null),
            }).collect())
        });

        Ok(IrRequest {
            model,
            messages,
            tools,
            tool_choice: body.get("tool_choice").cloned(),
            temperature: body.get("temperature").and_then(|v| v.as_f64()).map(|v| v as f32),
            top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32),
            max_tokens: body.get("max_output_tokens").and_then(|v| v.as_u64()).map(|v| v as u32),
            stream: body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false),
            stop_sequences: None,
            response_format: body.get("text").cloned(),
            metadata: std::collections::HashMap::new(),
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data.is_empty() || data == "[DONE]" {
            return Ok(None);
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let event_type = chunk["type"].as_str().unwrap_or("");

        match event_type {
            "response.output_text.delta" => {
                Ok(Some(IrStreamChunk {
                    id: chunk["response_id"].as_str().map(String::from),
                    model: None,
                    delta_content: chunk["delta"].as_str().map(String::from),
                    delta_tool_calls: None,
                    finish_reason: None,
                    usage: None,
                }))
            }
            "response.completed" => {
                let response = &chunk["response"];
                Ok(Some(IrStreamChunk {
                    id: response["id"].as_str().map(String::from),
                    model: response["model"].as_str().map(String::from),
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason: Some("stop".into()),
                    usage: response.get("usage").map(|u| IrUsage {
                        prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                        completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                        total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
                    }),
                }))
            }
            _ => Ok(None),
        }
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let output = &body["output"];
        let text = output.as_array()
            .and_then(|arr| arr.iter()
                .find(|item| item["type"] == "message")
                .and_then(|msg| msg["content"].as_array())
                .and_then(|content| content.iter()
                    .find(|c| c["type"] == "output_text")
                    .and_then(|ot| ot["text"].as_str())
                )
            ).unwrap_or("");

        Ok(IrResponse {
            id: body["id"].as_str().map(String::from),
            model: body["model"].as_str().map(String::from),
            message: IrMessage {
                role: IrRole::Assistant,
                content: vec![IrContentPart::Text { text: text.to_string() }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason: body["status"].as_str().map(|s| {
                if s == "completed" { "stop".into() } else { s.to_string() }
            }),
            usage: body.get("usage").map(|u| IrUsage {
                prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }).unwrap_or(IrUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: add OpenAI Responses format parser"
```

### Task 2.3: Anthropic Messages Parser

**Files:**
- Create: `src-tauri/src/converter/parsers/anthropic.rs`

- [ ] **Step 1: 编写 parsers/anthropic.rs**

创建 `src-tauri/src/converter/parsers/anthropic.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct AnthropicParser;

impl FormatParser for AnthropicParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        let model = body["model"].as_str()
            .ok_or_else(|| ProxyError::Parse("missing model".into()))?
            .to_string();

        let mut messages: Vec<IrMessage> = Vec::new();

        // Anthropic 的 system 字段在顶层
        if let Some(system) = body.get("system") {
            let system_text = if system.is_string() {
                system.as_str().unwrap().to_string()
            } else if system.is_array() {
                system.as_array().unwrap().iter()
                    .filter_map(|s| s["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                String::new()
            };
            if !system_text.is_empty() {
                messages.push(IrMessage {
                    role: IrRole::System,
                    content: vec![IrContentPart::Text { text: system_text }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        if let Some(msg_array) = body["messages"].as_array() {
            for m in msg_array {
                let role = match m["role"].as_str().unwrap_or("user") {
                    "user" => IrRole::User,
                    "assistant" => IrRole::Assistant,
                    _ => IrRole::User,
                };

                let content = if m["content"].is_string() {
                    vec![IrContentPart::Text { text: m["content"].as_str().unwrap().to_string() }]
                } else if m["content"].is_array() {
                    let parts: Vec<IrContentPart> = m["content"].as_array().unwrap().iter().map(|part| {
                        match part["type"].as_str() {
                            Some("text") => IrContentPart::Text {
                                text: part["text"].as_str().unwrap_or("").to_string(),
                            },
                            Some("image") => IrContentPart::Image {
                                url: None,
                                data: part["source"].get("data").and_then(|v| v.as_str()).map(String::from),
                                media_type: part["source"].get("media_type").and_then(|v| v.as_str()).map(String::from),
                            },
                            Some("tool_use") => IrContentPart::ToolUse {
                                id: part["id"].as_str().unwrap_or("").to_string(),
                                name: part["name"].as_str().unwrap_or("").to_string(),
                                input: part["input"].clone(),
                            },
                            Some("tool_result") => IrContentPart::ToolResult {
                                tool_use_id: part["tool_use_id"].as_str().unwrap_or("").to_string(),
                                content: part["content"].as_str().unwrap_or("").to_string(),
                            },
                            _ => IrContentPart::Text { text: String::new() },
                        }
                    }).collect();
                    parts
                } else {
                    vec![]
                };

                let tool_calls = m["content"].as_array().map(|parts| {
                    parts.iter().filter_map(|p| {
                        if p["type"] == "tool_use" {
                            Some(IrToolCall {
                                id: p["id"].as_str()?.to_string(),
                                name: p["name"].as_str()?.to_string(),
                                arguments: serde_json::to_string(&p["input"]).unwrap_or_default(),
                            })
                        } else { None }
                    }).collect::<Vec<_>>()
                });

                let tool_calls = if tool_calls.as_ref().map_or(false, |v| v.is_empty()) {
                    None
                } else {
                    tool_calls
                };

                messages.push(IrMessage {
                    role,
                    content,
                    name: None,
                    tool_call_id: None,
                    tool_calls,
                });
            }
        }

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| arr.iter().map(|tool| IrTool {
                name: tool["name"].as_str().unwrap_or("").to_string(),
                description: tool.get("description").and_then(|v| v.as_str()).map(String::from),
                input_schema: tool["input_schema"].clone(),
            }).collect())
        });

        Ok(IrRequest {
            model,
            messages,
            tools,
            tool_choice: body.get("tool_choice").cloned(),
            temperature: body.get("temperature").and_then(|v| v.as_f64()).map(|v| v as f32),
            top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32),
            max_tokens: body.get("max_tokens").and_then(|v| v.as_u64()).map(|v| v as u32),
            stream: body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false),
            stop_sequences: body.get("stop_sequences").and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect()),
            response_format: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data.is_empty() {
            return Ok(None);
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let event_type = chunk["type"].as_str().unwrap_or("");

        match event_type {
            "content_block_delta" => {
                let delta = &chunk["delta"];
                let delta_content = delta["text"].as_str().map(String::from);
                let delta_tool_calls = if delta["type"] == "input_json_delta" {
                    Some(vec![IrToolCallDelta {
                        index: chunk["index"].as_u64().unwrap_or(0) as u32,
                        id: None,
                        name: None,
                        arguments: delta["partial_json"].as_str().map(String::from),
                    }])
                } else { None };

                Ok(Some(IrStreamChunk {
                    id: None,
                    model: None,
                    delta_content,
                    delta_tool_calls,
                    finish_reason: None,
                    usage: None,
                }))
            }
            "message_delta" => {
                let delta = &chunk["delta"];
                Ok(Some(IrStreamChunk {
                    id: None,
                    model: None,
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason: delta["stop_reason"].as_str().map(String::from),
                    usage: chunk.get("usage").map(|u| IrUsage {
                        prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                        completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                        total_tokens: (u["input_tokens"].as_u64().unwrap_or(0)
                            + u["output_tokens"].as_u64().unwrap_or(0)) as u32,
                    }),
                }))
            }
            "message_stop" => {
                Ok(Some(IrStreamChunk {
                    id: None,
                    model: None,
                    delta_content: None,
                    delta_tool_calls: None,
                    finish_reason: Some("end_turn".into()),
                    usage: None,
                }))
            }
            _ => Ok(None),
        }
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let msg = &body;
        let content_text = msg["content"].as_array().map(|arr| {
            arr.iter().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("")
        }).unwrap_or_default();

        Ok(IrResponse {
            id: body["id"].as_str().map(String::from),
            model: body["model"].as_str().map(String::from),
            message: IrMessage {
                role: IrRole::Assistant,
                content: vec![IrContentPart::Text { text: content_text }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason: body["stop_reason"].as_str().map(String::from),
            usage: body.get("usage").map(|u| IrUsage {
                prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (u["input_tokens"].as_u64().unwrap_or(0)
                    + u["output_tokens"].as_u64().unwrap_or(0)) as u32,
            }).unwrap_or(IrUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: add Anthropic Messages format parser"
```

### Task 2.4: Google Gemini Parser

**Files:**
- Create: `src-tauri/src/converter/parsers/gemini.rs`

- [ ] **Step 1: 编写 parsers/gemini.rs**

创建 `src-tauri/src/converter/parsers/gemini.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatParser;
use crate::error::ProxyError;
use serde_json::Value;

pub struct GeminiParser;

impl FormatParser for GeminiParser {
    fn parse_request(&self, body: &Value) -> Result<IrRequest, ProxyError> {
        // Gemini 的 model 在 URL path 中，不在 body 里
        // 这里从 body 可选的 model 取，或由 handler 后期设置
        let model = body["model"].as_str().unwrap_or("unknown").to_string();

        let mut messages = Vec::new();

        // systemInstruction
        if let Some(si) = body.get("systemInstruction") {
            let text = si["parts"].as_array().map(|parts| {
                parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("\n")
            }).unwrap_or_default();
            if !text.is_empty() {
                messages.push(IrMessage {
                    role: IrRole::System,
                    content: vec![IrContentPart::Text { text }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        // contents[]
        if let Some(contents) = body["contents"].as_array() {
            for c in contents {
                let role = match c["role"].as_str().unwrap_or("user") {
                    "user" => IrRole::User,
                    "model" => IrRole::Assistant,
                    _ => IrRole::User,
                };

                let parts = c["parts"].as_array().map(|p_arr| {
                    p_arr.iter().filter_map(|p| {
                        if let Some(text) = p["text"].as_str() {
                            Some(IrContentPart::Text { text: text.to_string() })
                        } else if let Some(fc) = p.get("functionCall") {
                            Some(IrContentPart::ToolUse {
                                id: fc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                name: fc["name"].as_str().unwrap_or("").to_string(),
                                input: fc.get("args").cloned().unwrap_or(Value::Null),
                            })
                        } else if let Some(fr) = p.get("functionResponse") {
                            Some(IrContentPart::ToolResult {
                                tool_use_id: fr.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                content: fr.get("response").map(|r| r.to_string()).unwrap_or_default(),
                            })
                        } else {
                            None
                        }
                    }).collect::<Vec<_>>()
                }).unwrap_or_default();

                messages.push(IrMessage {
                    role,
                    content: parts,
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        let tools = body.get("tools").and_then(|t| {
            t.as_array().map(|arr| arr.iter().filter_map(|tool| {
                let fd = tool.get("functionDeclarations")?;
                // 这里返回多个 functionDeclarations
                None // 简化处理，实际 Gemini tool 格式不同
            }).collect())
        });

        Ok(IrRequest {
            model,
            messages,
            tools,
            tool_choice: body.get("toolConfig").cloned(),
            temperature: body.get("generationConfig").and_then(|gc| gc["temperature"].as_f64()).map(|v| v as f32),
            top_p: body.get("generationConfig").and_then(|gc| gc["topP"].as_f64()).map(|v| v as f32),
            max_tokens: body.get("generationConfig").and_then(|gc| gc["maxOutputTokens"].as_u64()).map(|v| v as u32),
            stream: false, // Gemini 流式通过 generateContentStream
            stop_sequences: body.get("generationConfig").and_then(|gc| gc["stopSequences"].as_array())
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect()),
            response_format: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn parse_stream_chunk(&self, line: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data.is_empty() {
            return Ok(None);
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|e| ProxyError::Parse(format!("SSE parse error: {}", e)))?;

        let delta_content = chunk["candidates"].get(0)
            .and_then(|c| c["content"]["parts"].get(0))
            .and_then(|p| p["text"].as_str())
            .map(String::from);

        let finish_reason = chunk["candidates"].get(0)
            .and_then(|c| c["finishReason"].as_str())
            .map(String::from);

        Ok(Some(IrStreamChunk {
            id: None,
            model: None,
            delta_content,
            delta_tool_calls: None,
            finish_reason,
            usage: chunk.get("usageMetadata").map(|u| IrUsage {
                prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            }),
        }))
    }

    fn parse_response(&self, body: &Value) -> Result<IrResponse, ProxyError> {
        let candidate = body["candidates"].get(0).ok_or(ProxyError::Parse("no candidates".into()))?;
        let text = candidate["content"]["parts"].as_array()
            .map(|parts| parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(""))
            .unwrap_or_default();

        Ok(IrResponse {
            id: None,
            model: None,
            message: IrMessage {
                role: IrRole::Assistant,
                content: vec![IrContentPart::Text { text }],
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason: candidate["finishReason"].as_str().map(String::from),
            usage: body.get("usageMetadata").map(|u| IrUsage {
                prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            }).unwrap_or(IrUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: add Google Gemini format parser"
```

### Task 2.5: Format Generators (Completions + Responses + Anthropic + Gemini)

**Files:**
- Create: `src-tauri/src/converter/generators/mod.rs`, `src-tauri/src/converter/generators/completions.rs`, `src-tauri/src/converter/generators/responses.rs`, `src-tauri/src/converter/generators/anthropic.rs`, `src-tauri/src/converter/generators/gemini.rs`

- [ ] **Step 1: 编写 generators/mod.rs**

```rust
pub mod completions;
pub mod responses;
pub mod anthropic;
pub mod gemini;
```

- [ ] **Step 2: 编写 generators/completions.rs**

创建 `src-tauri/src/converter/generators/completions.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct CompletionsGenerator;

impl FormatGenerator for CompletionsGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let messages: Vec<Value> = ir.messages.iter().map(|m| {
            let content = if m.content.len() == 1 {
                match &m.content[0] {
                    IrContentPart::Text { text } => json!(text),
                    _ => json!(""),
                }
            } else {
                let parts: Vec<Value> = m.content.iter().map(|p| match p {
                    IrContentPart::Text { text } => json!({"type": "text", "text": text}),
                    IrContentPart::Image { url, data, media_type } => {
                        let mut img = serde_json::Map::new();
                        if let Some(url) = url {
                            img.insert("url".into(), json!(url));
                        }
                        if let Some(data) = data {
                            img.insert("data".into(), json!(data));
                        }
                        json!({"type": "image_url", "image_url": img})
                    }
                    _ => json!({"type": "text", "text": ""}),
                }).collect();
                json!(parts)
            };

            let mut msg = json!({
                "role": match m.role {
                    IrRole::System => "system",
                    IrRole::User => "user",
                    IrRole::Assistant => "assistant",
                    IrRole::Tool => "tool",
                },
                "content": content,
            });

            if let Some(name) = &m.name {
                msg["name"] = json!(name);
            }
            if let Some(tool_call_id) = &m.tool_call_id {
                msg["tool_call_id"] = json!(tool_call_id);
            }
            if let Some(tool_calls) = &m.tool_calls {
                msg["tool_calls"] = json!(tool_calls.iter().map(|tc| json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments,
                    }
                })).collect::<Vec<_>>());
            }
            msg
        }).collect();

        let mut body = json!({
            "model": ir.model,
            "messages": messages,
            "stream": ir.stream,
        });

        if let Some(tools) = &ir.tools {
            body["tools"] = json!(tools.iter().map(|t| json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })).collect::<Vec<_>>());
        }
        if let Some(tc) = &ir.tool_choice {
            body["tool_choice"] = tc.clone();
        }
        if let Some(t) = ir.temperature { body["temperature"] = json!(t); }
        if let Some(p) = ir.top_p { body["top_p"] = json!(p); }
        if let Some(mt) = ir.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(stop) = &ir.stop_sequences {
            body["stop"] = if stop.len() == 1 { json!(&stop[0]) } else { json!(stop) };
        }
        if let Some(rf) = &ir.response_format { body["response_format"] = rf.clone(); }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        let mut c = json!({
            "id": chunk.id.clone().unwrap_or_default(),
            "object": "chat.completion.chunk",
            "model": chunk.model.clone().unwrap_or_default(),
            "choices": [{}],
        });

        let mut delta = json!({});
        if let Some(content) = &chunk.delta_content {
            delta["content"] = json!(content);
        }
        if let Some(tool_calls) = &chunk.delta_tool_calls {
            delta["tool_calls"] = json!(tool_calls.iter().map(|tc| json!({
                "index": tc.index,
                "id": tc.id,
                "type": "function",
                "function": {
                    "name": tc.name,
                    "arguments": tc.arguments,
                }
            })).collect::<Vec<_>>());
        }
        c["choices"][0]["delta"] = delta;

        if let Some(finish) = &chunk.finish_reason {
            c["choices"][0]["finish_reason"] = finish;
        }
        if let Some(usage) = &chunk.usage {
            c["usage"] = json!({
                "prompt_tokens": usage.prompt_tokens,
                "completion_tokens": usage.completion_tokens,
                "total_tokens": usage.total_tokens,
            });
        }

        format!("data: {}\n\n", serde_json::to_string(&c).unwrap_or_default())
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let content = ir.message.content.iter().find_map(|p| match p {
            IrContentPart::Text { text } => Some(text.clone()),
            _ => None,
        }).unwrap_or_default();

        let mut msg = json!({
            "id": ir.id.clone().unwrap_or_default(),
            "object": "chat.completion",
            "model": ir.model.clone().unwrap_or_default(),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content,
                },
                "finish_reason": ir.finish_reason.clone().unwrap_or_else(|| "stop".into()),
            }],
            "usage": {
                "prompt_tokens": ir.usage.prompt_tokens,
                "completion_tokens": ir.usage.completion_tokens,
                "total_tokens": ir.usage.total_tokens,
            },
        });

        if let Some(tool_calls) = &ir.message.tool_calls {
            msg["choices"][0]["message"]["tool_calls"] = json!(tool_calls.iter().map(|tc| json!({
                "id": tc.id,
                "type": "function",
                "function": { "name": tc.name, "arguments": tc.arguments }
            })).collect::<Vec<_>>());
        }

        Ok(msg)
    }
}
```

- [ ] **Step 3: 编写 generators/responses.rs**

创建 `src-tauri/src/converter/generators/responses.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct ResponsesGenerator;

impl FormatGenerator for ResponsesGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let instructions = ir.messages.iter()
            .find(|m| m.role == IrRole::System)
            .and_then(|m| m.content.iter().find_map(|p| match p {
                IrContentPart::Text { text } => Some(text.clone()),
                _ => None,
            }));

        let input = ir.messages.iter()
            .find(|m| m.role == IrRole::User)
            .and_then(|m| m.content.iter().find_map(|p| match p {
                IrContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })).unwrap_or_default();

        let mut body = json!({
            "model": ir.model,
            "input": input,
            "stream": ir.stream,
        });

        if let Some(inst) = instructions {
            body["instructions"] = json!(inst);
        }
        if let Some(tools) = &ir.tools {
            body["tools"] = json!(tools.iter().map(|t| json!({
                "type": "function",
                "name": t.name,
                "description": t.description,
                "parameters": t.input_schema,
            })).collect::<Vec<_>>());
        }
        if let Some(t) = ir.temperature { body["temperature"] = json!(t); }
        if let Some(p) = ir.top_p { body["top_p"] = json!(p); }
        if let Some(mt) = ir.max_tokens { body["max_output_tokens"] = json!(mt); }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        if let Some(finish) = &chunk.finish_reason {
            let completed = json!({
                "type": "response.completed",
                "response": {
                    "id": chunk.id.clone().unwrap_or_default(),
                    "object": "response",
                    "model": chunk.model.clone().unwrap_or_default(),
                    "status": "completed",
                    "usage": chunk.usage.as_ref().map(|u| json!({
                        "input_tokens": u.prompt_tokens,
                        "output_tokens": u.completion_tokens,
                        "total_tokens": u.total_tokens,
                    })),
                }
            });
            return format!("data: {}\n\n", serde_json::to_string(&completed).unwrap_or_default());
        }

        let delta = json!({
            "type": "response.output_text.delta",
            "response_id": chunk.id.clone().unwrap_or_default(),
            "delta": chunk.delta_content.clone().unwrap_or_default(),
        });
        format!("data: {}\n\n", serde_json::to_string(&delta).unwrap_or_default())
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let text = ir.message.content.iter().find_map(|p| match p {
            IrContentPart::Text { text } => Some(text.clone()),
            _ => None,
        }).unwrap_or_default();

        Ok(json!({
            "id": ir.id.clone().unwrap_or_default(),
            "object": "response",
            "model": ir.model.clone().unwrap_or_default(),
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": text,
                }],
            }],
            "usage": {
                "input_tokens": ir.usage.prompt_tokens,
                "output_tokens": ir.usage.completion_tokens,
                "total_tokens": ir.usage.total_tokens,
            },
        }))
    }
}
```

- [ ] **Step 4: 编写 generators/anthropic.rs**

创建 `src-tauri/src/converter/generators/anthropic.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct AnthropicGenerator;

impl FormatGenerator for AnthropicGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let system: Vec<String> = ir.messages.iter()
            .filter(|m| m.role == IrRole::System)
            .filter_map(|m| m.content.iter().find_map(|p| match p {
                IrContentPart::Text { text } => Some(text.clone()),
                _ => None,
            }))
            .collect();

        let messages: Vec<Value> = ir.messages.iter()
            .filter(|m| m.role != IrRole::System)
            .map(|m| {
                let role = match m.role {
                    IrRole::User => "user",
                    _ => "assistant",
                };

                let content: Vec<Value> = m.content.iter().map(|p| match p {
                    IrContentPart::Text { text } => json!({"type": "text", "text": text}),
                    IrContentPart::Image { url, data, media_type } => {
                        let mut source = json!({"type": "base64"});
                        if let Some(mt) = media_type { source["media_type"] = json!(mt); }
                        if let Some(d) = data { source["data"] = json!(d); }
                        json!({"type": "image", "source": source})
                    }
                    IrContentPart::ToolUse { id, name, input } => json!({
                        "type": "tool_use",
                        "id": id,
                        "name": name,
                        "input": input,
                    }),
                    IrContentPart::ToolResult { tool_use_id, content: c } => json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": c,
                    }),
                }).collect();

                let mut msg = json!({"role": role, "content": content});
                if m.content.len() == 1 {
                    if let IrContentPart::Text { text } = &m.content[0] {
                        msg["content"] = json!(text);
                    }
                }
                msg
            }).collect();

        let mut body = json!({
            "model": ir.model,
            "messages": messages,
            "max_tokens": ir.max_tokens.unwrap_or(4096),
        });

        if !system.is_empty() {
            body["system"] = if system.len() == 1 {
                json!(system[0])
            } else {
                json!(system.iter().map(|s| json!({"type": "text", "text": s})).collect::<Vec<_>>())
            };
        }
        if let Some(t) = ir.temperature { body["temperature"] = json!(t); }
        if let Some(p) = ir.top_p { body["top_p"] = json!(p); }
        if ir.stream { body["stream"] = json!(true); }
        if let Some(stop) = &ir.stop_sequences { body["stop_sequences"] = json!(stop); }
        if let Some(tools) = &ir.tools {
            body["tools"] = json!(tools.iter().map(|t| json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })).collect::<Vec<_>>());
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        if let Some(finish) = &chunk.finish_reason {
            let msg_stop = json!({
                "type": "message_stop",
            });
            return format!("data: {}\n\n", serde_json::to_string(&msg_stop).unwrap_or_default());
        }

        let delta = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": chunk.delta_content.clone().unwrap_or_default(),
            }
        });
        format!("data: {}\n\n", serde_json::to_string(&delta).unwrap_or_default())
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let text = ir.message.content.iter().find_map(|p| match p {
            IrContentPart::Text { text } => Some(text.clone()),
            _ => None,
        }).unwrap_or_default();

        Ok(json!({
            "id": ir.id.clone().unwrap_or_default(),
            "type": "message",
            "role": "assistant",
            "model": ir.model.clone().unwrap_or_default(),
            "content": [{"type": "text", "text": text}],
            "stop_reason": ir.finish_reason.clone().unwrap_or_else(|| "end_turn".into()),
            "stop_sequence": null,
            "usage": {
                "input_tokens": ir.usage.prompt_tokens,
                "output_tokens": ir.usage.completion_tokens,
            },
        }))
    }
}
```

- [ ] **Step 5: 编写 generators/gemini.rs**

创建 `src-tauri/src/converter/generators/gemini.rs`：

```rust
use crate::converter::ir::*;
use crate::converter::FormatGenerator;
use crate::error::ProxyError;
use serde_json::{json, Value};

pub struct GeminiGenerator;

impl FormatGenerator for GeminiGenerator {
    fn generate_request(&self, ir: &IrRequest) -> Result<Value, ProxyError> {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for m in &ir.messages {
            if m.role == IrRole::System {
                let parts: Vec<Value> = m.content.iter().filter_map(|p| match p {
                    IrContentPart::Text { text } => Some(json!({"text": text})),
                    _ => None,
                }).collect();
                system_instruction = Some(json!({"parts": parts}));
            } else {
                let role = match m.role {
                    IrRole::User => "user",
                    IrRole::Assistant => "model",
                    _ => "user",
                };
                let parts: Vec<Value> = m.content.iter().map(|p| match p {
                    IrContentPart::Text { text } => json!({"text": text}),
                    _ => json!({"text": ""}),
                }).collect();
                contents.push(json!({"role": role, "parts": parts}));
            }
        }

        let mut body = json!({"contents": contents});

        if let Some(si) = system_instruction {
            body["systemInstruction"] = si;
        }

        let mut generation_config = json!({});
        if let Some(t) = ir.temperature { generation_config["temperature"] = json!(t); }
        if let Some(p) = ir.top_p { generation_config["topP"] = json!(p); }
        if let Some(mt) = ir.max_tokens { generation_config["maxOutputTokens"] = json!(mt); }
        if generation_config.as_object().map_or(false, |m| !m.is_empty()) {
            body["generationConfig"] = generation_config;
        }

        Ok(body)
    }

    fn generate_stream_chunk(&self, chunk: &IrStreamChunk) -> String {
        let c = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": chunk.delta_content.clone().unwrap_or_default()}]
                },
                "finishReason": chunk.finish_reason.clone(),
            }],
            "usageMetadata": chunk.usage.as_ref().map(|u| json!({
                "promptTokenCount": u.prompt_tokens,
                "candidatesTokenCount": u.completion_tokens,
                "totalTokenCount": u.total_tokens,
            })),
        });
        format!("data: {}\n\n", serde_json::to_string(&c).unwrap_or_default())
    }

    fn generate_response(&self, ir: &IrResponse) -> Result<Value, ProxyError> {
        let text = ir.message.content.iter().find_map(|p| match p {
            IrContentPart::Text { text } => Some(text.clone()),
            _ => None,
        }).unwrap_or_default();

        Ok(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": text}],
                },
                "finishReason": ir.finish_reason.clone().unwrap_or_else(|| "STOP".into()),
            }],
            "usageMetadata": {
                "promptTokenCount": ir.usage.prompt_tokens,
                "candidatesTokenCount": ir.usage.completion_tokens,
                "totalTokenCount": ir.usage.total_tokens,
            },
        }))
    }
}
```

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: add all format generators (completions, responses, anthropic, gemini)"
```

---

## Phase 3: 核心代理引擎

### Task 3.1: 供应商管理

**Files:**
- Create: `src-tauri/src/provider/mod.rs`, `src-tauri/src/provider/manager.rs`, `src-tauri/src/provider/endpoint.rs`

- [ ] **Step 1: 编写 provider/mod.rs**

```rust
pub mod manager;
pub mod endpoint;

pub use manager::ProviderManager;
pub use endpoint::ProviderEndpoint;
```

- [ ] **Step 2: 编写 provider/endpoint.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpoint {
    pub id: String,
    pub provider_id: String,
    pub format: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: String,
    pub endpoints: Vec<ProviderEndpoint>,
    pub api_keys: Vec<ApiKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub label: String,
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
}
```

- [ ] **Step 3: 编写 provider/manager.rs**

```rust
use super::endpoint::{ApiKeyInfo, Provider, ProviderEndpoint};
use crate::db::pool::get_pool;
use crate::error::ProxyError;
use sqlx::SqlitePool;

pub struct ProviderManager;

impl ProviderManager {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Provider>, ProxyError> {
        let providers = sqlx::query_as!(DbProvider, "SELECT id, name, base_url, auth_type, auth_header FROM providers")
            .fetch_all(pool)
            .await?;

        let mut result = Vec::new();
        for p in providers {
            let endpoints = sqlx::query_as!(
                DbEndpoint,
                "SELECT id, provider_id, format, path FROM endpoints WHERE provider_id = ?",
                p.id
            )
            .fetch_all(pool)
            .await?;

            let keys = sqlx::query_as!(
                DbApiKeyInfo,
                "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ?",
                p.id
            )
            .fetch_all(pool)
            .await?;

            result.push(Provider {
                id: p.id,
                name: p.name,
                base_url: p.base_url,
                auth_type: p.auth_type,
                auth_header: p.auth_header,
                endpoints: endpoints.into_iter().map(|e| ProviderEndpoint {
                    id: e.id,
                    provider_id: e.provider_id,
                    format: e.format,
                    path: e.path,
                }).collect(),
                api_keys: keys.into_iter().map(|k| ApiKeyInfo {
                    id: k.id,
                    label: k.label,
                    is_active: k.is_active != 0,
                    usage_count: k.usage_count,
                    last_used_at: k.last_used_at,
                    created_at: k.created_at,
                }).collect(),
            });
        }

        Ok(result)
    }

    pub async fn find_for_model(
        pool: &SqlitePool,
        model: &str,
    ) -> Result<Option<ResolvedRoute>, ProxyError> {
        let route = sqlx::query_as!(
            DbModelRoute,
            "SELECT id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority FROM model_routes WHERE ? LIKE REPLACE(model_pattern, '*', '%') ORDER BY priority DESC LIMIT 1",
            model
        )
        .fetch_optional(pool)
        .await?;

        match route {
            Some(r) => {
                let provider = sqlx::query_as!(DbProvider, "SELECT id, name, base_url, auth_type, auth_header FROM providers WHERE id = ?", r.provider_id)
                    .fetch_one(pool)
                    .await?;

                let keys = sqlx::query_as!(
                    DbApiKeyInfo,
                    "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ? AND is_active = 1 ORDER BY usage_count ASC",
                    r.provider_id
                )
                .fetch_all(pool)
                .await?;

                if keys.is_empty() {
                    return Err(ProxyError::Config(format!("No active API key for provider {}", provider.name)));
                }

                let selected_key = &keys[0];

                Ok(Some(ResolvedRoute {
                    provider_name: provider.name,
                    base_url: provider.base_url,
                    auth_type: provider.auth_type,
                    auth_header: provider.auth_header,
                    api_key_id: selected_key.id.clone(),
                    encrypted_key: None, // 由 key store 解密
                    target_model: r.target_model,
                    target_format: r.target_format,
                    fallback_provider_id: r.fallback_provider_id,
                }))
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug)]
pub struct ResolvedRoute {
    pub provider_name: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: String,
    pub api_key_id: String,
    pub encrypted_key: Option<Vec<u8>>,
    pub target_model: String,
    pub target_format: String,
    pub fallback_provider_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DbProvider {
    id: String,
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: String,
}

#[derive(sqlx::FromRow)]
struct DbEndpoint {
    id: String,
    provider_id: String,
    format: String,
    path: String,
}

#[derive(sqlx::FromRow)]
struct DbApiKeyInfo {
    id: String,
    label: String,
    is_active: i64,
    usage_count: i64,
    last_used_at: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct DbModelRoute {
    id: String,
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: i64,
}
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add provider management module"
```

### Task 3.2: API Key 加密存储与轮询

**Files:**
- Create: `src-tauri/src/key/mod.rs`, `src-tauri/src/key/store.rs`, `src-tauri/src/key/rotation.rs`

- [ ] **Step 1: 编写 key/mod.rs**

```rust
pub mod store;
pub mod rotation;

pub use store::KeyStore;
pub use rotation::KeyRotation;
```

- [ ] **Step 2: 编写 key/store.rs**

```rust
use crate::db::pool::get_pool;
use crate::error::ProxyError;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredKey {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub plaintext: String,
}

pub struct KeyStore {
    key: LessSafeKey,
}

impl KeyStore {
    pub fn new(master_key: &[u8; 32]) -> Self {
        let unbound_key = UnboundKey::new(&AES_256_GCM, master_key)
            .expect("Failed to create encryption key");
        Self { key: LessSafeKey::new(unbound_key) }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<(Vec<u8>, Vec<u8>), ProxyError> {
        let rng = SystemRandom::new();
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)
            .map_err(|_| ProxyError::KeyManagement("Failed to generate nonce".into()))?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let mut in_out = plaintext.as_bytes().to_vec();
        self.key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| ProxyError::KeyManagement("Encryption failed".into()))?;

        Ok((in_out, nonce_bytes.to_vec()))
    }

    pub fn decrypt(&self, encrypted: &[u8], nonce_bytes: &[u8]) -> Result<String, ProxyError> {
        let nonce_arr: [u8; 12] = nonce_bytes.try_into()
            .map_err(|_| ProxyError::KeyManagement("Invalid nonce".into()))?;
        let nonce = Nonce::assume_unique_for_key(nonce_arr);

        let mut in_out = encrypted.to_vec();
        let plaintext = self.key.open_in_place(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| ProxyError::KeyManagement("Decryption failed".into()))?;

        String::from_utf8(plaintext.to_vec())
            .map_err(|e| ProxyError::KeyManagement(format!("Invalid UTF-8: {}", e)))
    }

    pub fn derive_key() -> [u8; 32] {
        // 生产环境应从系统 Keychain (macOS Keychain / Windows Credential Manager) 派生
        // 开发阶段使用固定种子（后续替换为 keyring crate 集成）
        use std::hash::{Hash, Hasher};
        let machine_id = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "default-machine-id".to_string());

        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        machine_id.hash(&mut hasher);
        let seed = hasher.finish();

        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = ((seed >> (i % 8 * 8)) & 0xFF) as u8;
            key[i] ^= b"AI_PROXY_SALT_2026"[i.min(17)];
        }
        key
    }
}
```

- [ ] **Step 3: 编写 key/rotation.rs**

```rust
use crate::db::pool::get_pool;
use crate::error::ProxyError;

pub struct KeyRotation;

impl KeyRotation {
    pub enum Strategy {
        RoundRobin,
        Random,
        LeastUsed,
    }

    pub async fn get_next_key(
        provider_id: &str,
        strategy: Strategy,
    ) -> Result<(String, Vec<u8>, Vec<u8>), ProxyError> {
        let pool = get_pool().await;
        let order = match strategy {
            Strategy::LeastUsed => "usage_count ASC",
            Strategy::Random => "RANDOM()",
            _ => "usage_count ASC",
        };

        let row = sqlx::query_as!(
            DbKeyRow,
            "SELECT id, encrypted_key, nonce FROM api_keys WHERE provider_id = ? AND is_active = 1 ORDER BY {} LIMIT 1",
            provider_id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ProxyError::KeyManagement(format!("No active key for provider {}", provider_id)))?;

        sqlx::query!(
            "UPDATE api_keys SET usage_count = usage_count + 1, last_used_at = datetime('now') WHERE id = ?",
            row.id
        )
        .execute(pool)
        .await?;

        Ok((row.id, row.encrypted_key, row.nonce))
    }
}

#[derive(sqlx::FromRow)]
struct DbKeyRow {
    id: String,
    encrypted_key: Vec<u8>,
    nonce: Vec<u8>,
}
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add API key encryption store and rotation"
```

### Task 3.3: 拦截器规则引擎

**Files:**
- Create: `src-tauri/src/interceptor/mod.rs`, `src-tauri/src/interceptor/engine.rs`, `src-tauri/src/interceptor/rules.rs`

- [ ] **Step 1: 编写 interceptor/mod.rs**

```rust
pub mod engine;
pub mod rules;

pub use engine::InterceptorEngine;
pub use rules::*;
```

- [ ] **Step 2: 编写 interceptor/rules.rs**

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RulePhase {
    Pre,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RuleCondition {
    ModelMatches { pattern: String },
    PathContains { substring: String },
    HeaderExists { name: String },
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RuleAction {
    ReplaceModel { new_model: String },
    SetHeader { name: String, value: String },
    RemoveHeader { name: String },
    InjectSystemPrompt { text: String },
    OverrideParameter { key: String, value: Value },
    FilterResponse { pattern: String, replacement: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptorRule {
    pub id: String,
    pub name: String,
    pub phase: RulePhase,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub priority: i64,
    pub enabled: bool,
}
```

- [ ] **Step 3: 编写 interceptor/engine.rs**

```rust
use super::rules::*;
use crate::converter::ir::IrRequest;
use crate::db::pool::get_pool;
use crate::error::ProxyError;
use serde_json::Value;
use std::collections::HashMap;

pub struct InterceptorEngine;

impl InterceptorEngine {
    pub async fn load_rules(phase: RulePhase) -> Result<Vec<InterceptorRule>, ProxyError> {
        let pool = get_pool().await;
        let phase_str = match phase {
            RulePhase::Pre => "pre",
            RulePhase::Post => "post",
        };

        let rows = sqlx::query_as!(
            DbRule,
            "SELECT id, name, phase, rule_type, condition_json, action_json, priority, enabled FROM interceptor_rules WHERE phase = ? AND enabled = 1 ORDER BY priority ASC",
            phase_str
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(|r| {
            Ok(InterceptorRule {
                id: r.id,
                name: r.name,
                phase: if r.phase == "pre" { RulePhase::Pre } else { RulePhase::Post },
                condition: serde_json::from_str(&r.condition_json)?,
                action: serde_json::from_str(&r.action_json)?,
                priority: r.priority,
                enabled: r.enabled != 0,
            })
        }).collect()
    }

    pub fn check_condition(condition: &RuleCondition, request: &IrRequest, path: &str, headers: &HashMap<String, String>) -> bool {
        match condition {
            RuleCondition::ModelMatches { pattern } => {
                if pattern.contains('*') {
                    let p = pattern.replace('*', "");
                    request.model.contains(&p)
                } else {
                    request.model == *pattern
                }
            }
            RuleCondition::PathContains { substring } => path.contains(substring.as_str()),
            RuleCondition::HeaderExists { name } => headers.contains_key(name.as_str()),
            RuleCondition::Always => true,
        }
    }

    pub fn apply_action(action: &RuleAction, request: &mut IrRequest, _headers: &mut HashMap<String, String>) {
        match action {
            RuleAction::ReplaceModel { new_model } => {
                request.model = new_model.clone();
            }
            RuleAction::SetHeader { name, value } => {
                _headers.insert(name.clone(), value.clone());
            }
            RuleAction::RemoveHeader { name } => {
                _headers.remove(name.as_str());
            }
            RuleAction::InjectSystemPrompt { text } => {
                request.messages.insert(0, crate::converter::ir::IrMessage {
                    role: crate::converter::ir::IrRole::System,
                    content: vec![crate::converter::ir::IrContentPart::Text { text: text.clone() }],
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
            RuleAction::OverrideParameter { key, value } => {
                match key.as_str() {
                    "temperature" => {
                        if let Some(v) = value.as_f64() {
                            request.temperature = Some(v as f32);
                        }
                    }
                    "top_p" => {
                        if let Some(v) = value.as_f64() {
                            request.top_p = Some(v as f32);
                        }
                    }
                    "max_tokens" => {
                        if let Some(v) = value.as_u64() {
                            request.max_tokens = Some(v as u32);
                        }
                    }
                    _ => {
                        request.metadata.insert(key.clone(), value.clone());
                    }
                }
            }
            RuleAction::FilterResponse { .. } => {
                // 后置拦截在响应阶段处理
            }
        }
    }

    pub async fn execute_pre_rules(
        request: &mut IrRequest,
        path: &str,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), ProxyError> {
        let rules = Self::load_rules(RulePhase::Pre).await?;
        for rule in rules {
            if Self::check_condition(&rule.condition, request, path, headers) {
                Self::apply_action(&rule.action, request, headers);
            }
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct DbRule {
    id: String,
    name: String,
    phase: String,
    rule_type: String,
    condition_json: String,
    action_json: String,
    priority: i64,
    enabled: i64,
}
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add interceptor rules engine"
```

### Task 3.4: HTTP 代理服务器 & 请求处理

**Files:**
- Create: `src-tauri/src/server/mod.rs`, `src-tauri/src/server/router.rs`, `src-tauri/src/server/middleware.rs`
- Create: `src-tauri/src/server/handlers/mod.rs`, `src-tauri/src/server/handlers/completions.rs`, `src-tauri/src/server/handlers/responses.rs`, `src-tauri/src/server/handlers/anthropic.rs`, `src-tauri/src/server/handlers/gemini.rs`

- [ ] **Step 1: 编写 server/mod.rs 和 router.rs**

```rust
// server/mod.rs
pub mod router;
pub mod handlers;
pub mod middleware;

use axum::Router;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

pub fn create_server(host: &str, port: u16) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            if host == "127.0.0.1" || host == "localhost" {
                "http://localhost:1420".parse::<HeaderValue>().unwrap()
            } else {
                "*".parse::<HeaderValue>().unwrap()
            }
        )
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION]);

    router::build_router().layer(cors)
}

pub async fn start_server(host: String, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_server(&host, port);
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Proxy server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
```

```rust
// server/router.rs
use axum::Router;
use super::handlers;

pub fn build_router() -> Router {
    Router::new()
        .route("/v1/chat/completions", axum::routing::post(handlers::completions::handle_completions))
        .route("/v1/responses", axum::routing::post(handlers::responses::handle_responses))
        .route("/v1/messages", axum::routing::post(handlers::anthropic::handle_anthropic))
        .route("/v1beta/models/:model", axum::routing::post(handlers::gemini::handle_gemini))
        .route("/v1beta/models/:model/:action", axum::routing::post(handlers::gemini::handle_gemini_action))
        .route("/health", axum::routing::get(health_check))
}

async fn health_check() -> &'static str {
    "ok"
}
```

- [ ] **Step 2: 编写 handlers/mod.rs**

```rust
pub mod completions;
pub mod responses;
pub mod anthropic;
pub mod gemini;
```

- [ ] **Step 3: 编写 handlers/completions.rs (代理核心流程)**

```rust
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use crate::converter::ir::*;
use crate::converter::parsers::completions::CompletionsParser;
use crate::converter::generators::completions::CompletionsGenerator;
use crate::converter::{FormatGenerator, FormatParser};
use crate::provider::manager::ProviderManager;
use crate::key::rotation::KeyRotation;
use crate::key::store::KeyStore;
use crate::interceptor::engine::InterceptorEngine;
use crate::db::pool::get_pool;
use crate::error::ProxyError;

pub async fn handle_completions(
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Result<Response, ProxyError> {
    let pool = get_pool().await;

    // 1. 解析客户端请求为 IR
    let parser = CompletionsParser;
    let mut ir = parser.parse_request(&body)?;

    // 2. 前置拦截
    let mut headers = std::collections::HashMap::new();
    InterceptorEngine::execute_pre_rules(&mut ir, "/v1/chat/completions", &mut headers).await?;

    // 3. 模型路由
    let route = ProviderManager::find_for_model(pool, &ir.model)
        .await?
        .ok_or_else(|| ProxyError::ModelNotFound(format!("No route for model: {}", ir.model)))?;

    // 4. 获取 API Key
    let key_store = KeyStore::new(&KeyStore::derive_key());
    let (_, encrypted_key, nonce) = KeyRotation::get_next_key(
        &route.provider_name,
        KeyRotation::Strategy::LeastUsed,
    ).await?;
    let api_key = key_store.decrypt(&encrypted_key, &nonce)?;

    // 5. 生成目标格式请求
    let target_request = generate_target_request(&ir, &route.target_format)?;

    // 6. 发送请求到供应商
    if ir.stream {
        handle_streaming_request(&route, &api_key, &target_request, &ir).await
    } else {
        handle_non_streaming_request(&route, &api_key, &target_request, &ir).await
    }
}

fn generate_target_request(ir: &IrRequest, target_format: &str) -> Result<serde_json::Value, ProxyError> {
    match target_format {
        "completions" => CompletionsGenerator.generate_request(ir),
        "responses" => crate::converter::generators::responses::ResponsesGenerator.generate_request(ir),
        "anthropic" => crate::converter::generators::anthropic::AnthropicGenerator.generate_request(ir),
        "gemini" => crate::converter::generators::gemini::GeminiGenerator.generate_request(ir),
        _ => Err(ProxyError::Config(format!("Unknown target format: {}", target_format))),
    }
}

async fn handle_non_streaming_request(
    route: &crate::provider::manager::ResolvedRoute,
    api_key: &str,
    target_request: &serde_json::Value,
    ir: &IrRequest,
) -> Result<Response, ProxyError> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", route.base_url.trim_end_matches('/'), get_format_path(&route.target_format));

    let start = std::time::Instant::now();
    let resp = client.post(&url)
        .header(&route.auth_header, format!("Bearer {}", api_key))
        .json(target_request)
        .send()
        .await
        .map_err(|e| ProxyError::Network(format!("Request failed: {}", e)))?;

    let status = resp.status();
    let duration_ms = start.elapsed().as_millis() as i64;
    let body: serde_json::Value = resp.json().await
        .map_err(|e| ProxyError::Provider(format!("Failed to parse response: {}", e)))?;

    // 记录日志
    log_request(ir, &route.target_format, &route.provider_name, status.as_u16(), duration_ms).await;

    if status.is_success() {
        let ir_response = parse_target_response(&body, &route.target_format)?;
        let client_response = generate_client_response(&ir_response)?;
        Ok((StatusCode::OK, axum::Json(client_response)).into_response())
    } else {
        Ok((status, axum::Json(body)).into_response())
    }
}

async fn handle_streaming_request(
    route: &crate::provider::manager::ResolvedRoute,
    api_key: &str,
    target_request: &serde_json::Value,
    ir: &IrRequest,
) -> Result<Response, ProxyError> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", route.base_url.trim_end_matches('/'), get_format_path(&route.target_format));

    let resp = client.post(&url)
        .header(&route.auth_header, format!("Bearer {}", api_key))
        .json(target_request)
        .send()
        .await
        .map_err(|e| ProxyError::Network(format!("Stream request failed: {}", e)))?;

    let stream = resp.bytes_stream();
    let target_format = route.target_format.clone();
    let provider_name = route.provider_name.clone();
    let model = ir.model.clone();
    let client_format = "completions".to_string();

    let sse_stream = async_stream::stream! {
        use futures::StreamExt;
        let mut stream = stream;
        while let Some(chunk) = stream.next().await {
            if let Ok(data) = chunk {
                let text = String::from_utf8_lossy(&data);
                for line in text.lines() {
                    if line.is_empty() { continue; }
                    let ir_chunk = parse_target_stream_chunk(line, &target_format);
                    if let Ok(Some(chunk)) = ir_chunk {
                        let output = generate_client_stream_chunk(&chunk);
                        yield Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default().data(output));
                    }
                }
            }
        }
    };

    let response = axum::response::Sse::new(sse_stream)
        .keep_alive(axum::response::sse::KeepAlive::default());

    // 记录流式请求日志（简化）
    log_request(ir, &target_format, &provider_name, 200, 0).await;

    Ok(response.into_response())
}

fn get_format_path(format: &str) -> &str {
    match format {
        "completions" => "/v1/chat/completions",
        "responses" => "/v1/responses",
        "anthropic" => "/v1/messages",
        "gemini" => "/v1beta/models/gemini-pro:generateContent",
        _ => "/v1/chat/completions",
    }
}

fn parse_target_response(body: &serde_json::Value, format: &str) -> Result<IrResponse, ProxyError> {
    match format {
        "completions" => CompletionsParser.parse_response(body),
        "responses" => crate::converter::parsers::responses::ResponsesParser.parse_response(body),
        "anthropic" => crate::converter::parsers::anthropic::AnthropicParser.parse_response(body),
        "gemini" => crate::converter::parsers::gemini::GeminiParser.parse_response(body),
        _ => Err(ProxyError::Config(format!("Unknown target format: {}", format))),
    }
}

fn generate_client_response(ir: &IrResponse) -> Result<serde_json::Value, ProxyError> {
    CompletionsGenerator.generate_response(ir)
}

fn parse_target_stream_chunk(line: &str, format: &str) -> Result<Option<IrStreamChunk>, ProxyError> {
    match format {
        "completions" => CompletionsParser.parse_stream_chunk(line),
        "responses" => crate::converter::parsers::responses::ResponsesParser.parse_stream_chunk(line),
        "anthropic" => crate::converter::parsers::anthropic::AnthropicParser.parse_stream_chunk(line),
        "gemini" => crate::converter::parsers::gemini::GeminiParser.parse_stream_chunk(line),
        _ => Ok(None),
    }
}

fn generate_client_stream_chunk(chunk: &IrStreamChunk) -> String {
    CompletionsGenerator.generate_stream_chunk(chunk)
}

async fn log_request(ir: &IrRequest, target_format: &str, provider_name: &str, status: u16, duration_ms: i64) {
    let pool = get_pool().await;
    let _ = sqlx::query!(
        "INSERT INTO request_logs (request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        uuid::Uuid::new_v4().to_string(),
        "completions",
        provider_name,
        target_format,
        ir.model,
        ir.stream as i64,
        status as i64,
        duration_ms,
    )
    .execute(pool)
    .await;
}
```

- [ ] **Step 4: 编写其他 handlers (responses, anthropic, gemini)**

各 handler 与 completions.rs 结构一致，差异仅在步骤 1/5/7 使用的 Parser/Generator 类型：

- `handlers/responses.rs`: 导入 `ResponsesParser` + `ResponsesGenerator`，路径为 `/v1/responses`
- `handlers/anthropic.rs`: 导入 `AnthropicParser` + `AnthropicGenerator`，路径为 `/v1/messages`
- `handlers/gemini.rs`: 导入 `GeminiParser` + `GeminiGenerator`，路径从 URL 参数提取

每个 handler 的文件结构与 completions.rs 完全相同，包括 `generate_target_request`、`parse_target_response` 等辅助函数，只替换对应的 Parser/Generator 类型。

- [ ] **Step 5: 编写 usage/tracker.rs（用量聚合）**

创建 `src-tauri/src/usage/tracker.rs`：

```rust
use crate::db::pool::get_pool;
use crate::converter::ir::IrUsage;
use chrono::Utc;

pub struct UsageTracker;

impl UsageTracker {
    pub async fn record(
        model: &str,
        provider_name: &str,
        usage: &IrUsage,
    ) -> Result<(), sqlx::Error> {
        let pool = get_pool().await;
        let now = Utc::now();
        let bucket = now.format("%Y-%m-%d %H:%M:00").to_string();

        // 估算成本（统一按 $0.001/1K tokens 粗略估算，精确价格由 pricing.rs 配置）
        let cost = (usage.prompt_tokens as f64 * 0.003 + usage.completion_tokens as f64 * 0.006) / 1000.0;

        sqlx::query!(
            "INSERT INTO usage_stats (model, provider_name, prompt_tokens, completion_tokens, total_tokens, cost_estimate, request_count, bucket_minute) VALUES (?, ?, ?, ?, ?, ?, 1, ?)",
            model, provider_name, usage.prompt_tokens, usage.completion_tokens, usage.total_tokens, cost, bucket
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
```

创建 `src-tauri/src/usage/mod.rs`：

```rust
pub mod tracker;
pub mod pricing;

pub use tracker::UsageTracker;
```

创建 `src-tauri/src/usage/pricing.rs`：

```rust
use std::collections::HashMap;

pub struct PricingTable {
    models: HashMap<String, ModelPricing>,
}

struct ModelPricing {
    prompt_price_per_1k: f64,
    completion_price_per_1k: f64,
}

impl Default for PricingTable {
    fn default() -> Self {
        let mut models = HashMap::new();
        // OpenAI
        models.insert("gpt-4o".into(), ModelPricing { prompt_price_per_1k: 0.0025, completion_price_per_1k: 0.01 });
        models.insert("gpt-4o-mini".into(), ModelPricing { prompt_price_per_1k: 0.00015, completion_price_per_1k: 0.0006 });
        // Anthropic
        models.insert("claude-sonnet-4-5".into(), ModelPricing { prompt_price_per_1k: 0.003, completion_price_per_1k: 0.015 });
        models.insert("claude-haiku-4-5".into(), ModelPricing { prompt_price_per_1k: 0.0008, completion_price_per_1k: 0.004 });
        // DeepSeek
        models.insert("deepseek-chat".into(), ModelPricing { prompt_price_per_1k: 0.00014, completion_price_per_1k: 0.00028 });
        models.insert("deepseek-reasoner".into(), ModelPricing { prompt_price_per_1k: 0.00055, completion_price_per_1k: 0.00219 });
        // Moonshot
        models.insert("moonshot-v1-8k".into(), ModelPricing { prompt_price_per_1k: 0.0005, completion_price_per_1k: 0.0005 });

        Self { models }
    }
}

impl PricingTable {
    pub fn get_cost(&self, model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        if let Some(pricing) = self.models.get(model) {
            (prompt_tokens as f64 * pricing.prompt_price_per_1k
                + completion_tokens as f64 * pricing.completion_price_per_1k) / 1000.0
        } else {
            // 未知模型默认价格
            ((prompt_tokens + completion_tokens) as f64 * 0.001) / 1000.0
        }
    }
}
```

在 proxy handler 中集成 UsageTracker 调用（在 `handle_non_streaming_request` 和 `handle_streaming_request` 返回前）：

```rust
use crate::usage::UsageTracker;

// 在解析完 ir_response 后添加：
UsageTracker::record(&ir.model, &route.provider_name, &ir_response.usage).await?;
```

- [ ] **Step 5: 在 main.rs 中集成 HTTP 服务器启动**

在 `main.rs` 的 `setup` 中添加：

```rust
use crate::server;

// 在 setup 闭包中：
let host = "127.0.0.1".to_string();
let port = 7860u16;
let host_clone = host.clone();
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = server::start_server(host_clone, port).await {
            tracing::error!("Server error: {}", e);
        }
    });
});
```

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: add HTTP proxy server with request handlers"
```

---

## Phase 4: Tauri IPC 命令

### Task 4.1: Provider & Key IPC 命令

**Files:**
- Create: `src-tauri/src/ipc/mod.rs`, `src-tauri/src/ipc/provider_cmd.rs`, `src-tauri/src/ipc/key_cmd.rs`

- [ ] **Step 1: 编写 ipc/mod.rs**

```rust
pub mod provider_cmd;
pub mod key_cmd;
pub mod routing_cmd;
pub mod log_cmd;
pub mod usage_cmd;
pub mod interceptor_cmd;
```

- [ ] **Step 2: 编写 ipc/provider_cmd.rs**

```rust
use crate::db::pool::get_pool;
use crate::provider::manager::ProviderManager;
use tauri::command;

#[command]
pub async fn get_providers() -> Result<Vec<crate::provider::endpoint::Provider>, String> {
    let pool = get_pool().await;
    ProviderManager::list(pool).await.map_err(|e| e.to_string())
}

#[command]
pub async fn create_provider(
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: String,
    endpoints: Vec<EndpointInput>,
) -> Result<String, String> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query!(
        "INSERT INTO providers (id, name, base_url, auth_type, auth_header) VALUES (?, ?, ?, ?, ?)",
        id, name, base_url, auth_type, auth_header
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    for ep in endpoints {
        sqlx::query!(
            "INSERT INTO endpoints (id, provider_id, format, path) VALUES (?, ?, ?, ?)",
            uuid::Uuid::new_v4().to_string(), id, ep.format, ep.path
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(id)
}

#[command]
pub async fn update_provider(
    id: String,
    name: String,
    base_url: String,
) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query!(
        "UPDATE providers SET name = ?, base_url = ?, updated_at = datetime('now') WHERE id = ?",
        name, base_url, id
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn delete_provider(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query!("DELETE FROM providers WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct EndpointInput {
    pub format: String,
    pub path: String,
}
```

- [ ] **Step 3: 编写 ipc/key_cmd.rs**

```rust
use crate::db::pool::get_pool;
use crate::key::store::KeyStore;
use tauri::command;

#[command]
pub async fn get_api_keys(provider_id: String) -> Result<Vec<KeyInfo>, String> {
    let pool = get_pool().await;
    let rows = sqlx::query_as!(
        DbKeyInfo,
        "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ? ORDER BY created_at DESC",
        provider_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|r| KeyInfo {
        id: r.id,
        label: r.label,
        is_active: r.is_active != 0,
        usage_count: r.usage_count,
        last_used_at: r.last_used_at,
        created_at: r.created_at,
    }).collect())
}

#[command]
pub async fn create_api_key(provider_id: String, label: String, plaintext_key: String) -> Result<String, String> {
    let pool = get_pool().await;
    let key_store = KeyStore::new(&KeyStore::derive_key());
    let (encrypted, nonce) = key_store.encrypt(&plaintext_key).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query!(
        "INSERT INTO api_keys (id, provider_id, label, encrypted_key, nonce) VALUES (?, ?, ?, ?, ?)",
        id, provider_id, label, encrypted, nonce
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(id)
}

#[command]
pub async fn delete_api_key(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query!("DELETE FROM api_keys WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct KeyInfo {
    pub id: String,
    pub label: String,
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct DbKeyInfo {
    id: String,
    label: String,
    is_active: i64,
    usage_count: i64,
    last_used_at: Option<String>,
    created_at: String,
}
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add provider and key IPC commands"
```

### Task 4.2: Routing, Logs, Usage, Interceptor IPC 命令

**Files:**
- Create: `src-tauri/src/ipc/routing_cmd.rs`, `src-tauri/src/ipc/log_cmd.rs`, `src-tauri/src/ipc/usage_cmd.rs`, `src-tauri/src/ipc/interceptor_cmd.rs`

- [ ] **Step 1-4: 编写各 IPC 命令模块**

创建 `routing_cmd.rs`:

```rust
use crate::db::pool::get_pool;
use tauri::command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelRoute {
    pub id: String,
    pub model_pattern: String,
    pub alias: Option<String>,
    pub provider_id: String,
    pub target_model: String,
    pub target_format: String,
    pub fallback_provider_id: Option<String>,
    pub priority: i64,
}

#[command]
pub async fn get_routes() -> Result<Vec<ModelRoute>, String> {
    let pool = get_pool().await;
    let rows = sqlx::query_as!(
        DbRoute,
        "SELECT id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority FROM model_routes ORDER BY priority DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|r| ModelRoute {
        id: r.id,
        model_pattern: r.model_pattern,
        alias: r.alias,
        provider_id: r.provider_id,
        target_model: r.target_model,
        target_format: r.target_format,
        fallback_provider_id: r.fallback_provider_id,
        priority: r.priority,
    }).collect())
}

#[command]
pub async fn create_route(
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: i64,
) -> Result<String, String> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO model_routes (id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority,
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(id)
}

#[command]
pub async fn delete_route(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query!("DELETE FROM model_routes WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct DbRoute {
    id: String,
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: i64,
}
```

创建 `log_cmd.rs`:

```rust
use crate::db::pool::get_pool;
use tauri::command;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RequestLog {
    pub id: i64,
    pub request_id: String,
    pub client_format: String,
    pub provider_name: String,
    pub provider_format: String,
    pub model: String,
    pub stream: bool,
    pub status_code: i64,
    pub duration_ms: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[command]
pub async fn get_logs(page: i64, limit: i64) -> Result<Vec<RequestLog>, String> {
    let pool = get_pool().await;
    let offset = (page - 1) * limit;
    let rows = sqlx::query_as!(
        DbRequestLog,
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs ORDER BY id DESC LIMIT ? OFFSET ?",
        limit, offset
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|r| RequestLog {
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
    }).collect())
}

#[command]
pub async fn get_log_detail(id: i64) -> Result<Option<RequestLog>, String> {
    let pool = get_pool().await;
    let row = sqlx::query_as!(
        DbRequestLog,
        "SELECT id, request_id, client_format, provider_name, provider_format, model, stream, status_code, duration_ms, prompt_tokens, completion_tokens, total_tokens, error_message, created_at FROM request_logs WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|r| RequestLog {
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
    status_code: i64,
    duration_ms: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    error_message: Option<String>,
    created_at: String,
}
```

创建 `usage_cmd.rs`:

```rust
use crate::db::pool::get_pool;
use tauri::command;
use serde::Serialize;

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
    let rows = sqlx::query_as!(
        DbUsageSummary,
        "SELECT model, provider_name, SUM(prompt_tokens) as prompt_tokens, SUM(completion_tokens) as completion_tokens, SUM(total_tokens) as total_tokens, SUM(cost_estimate) as cost, SUM(request_count) as req_count FROM usage_stats WHERE bucket_minute >= datetime('now', ?) GROUP BY model, provider_name ORDER BY total_tokens DESC",
        format!("-{} days", days)
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|r| UsageSummary {
        model: r.model,
        provider_name: r.provider_name,
        total_prompt_tokens: r.prompt_tokens,
        total_completion_tokens: r.completion_tokens,
        total_tokens: r.total_tokens,
        total_cost: r.cost,
        request_count: r.req_count,
    }).collect())
}

#[derive(sqlx::FromRow)]
struct DbUsageSummary {
    model: String,
    provider_name: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cost: f64,
    req_count: i64,
}
```

创建 `interceptor_cmd.rs`:

```rust
use crate::db::pool::get_pool;
use crate::interceptor::rules::InterceptorRule;
use tauri::command;

#[command]
pub async fn get_rules() -> Result<Vec<InterceptorRule>, String> {
    let pool = get_pool().await;
    let rows = sqlx::query_as!(
        DbRule,
        "SELECT id, name, phase, rule_type, condition_json, action_json, priority, enabled FROM interceptor_rules ORDER BY priority ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter().map(|r| {
        Ok(InterceptorRule {
            id: r.id,
            name: r.name,
            phase: if r.phase == "pre" { crate::interceptor::rules::RulePhase::Pre } else { crate::interceptor::rules::RulePhase::Post },
            condition: serde_json::from_str(&r.condition_json).map_err(|e| e.to_string())?,
            action: serde_json::from_str(&r.action_json).map_err(|e| e.to_string())?,
            priority: r.priority,
            enabled: r.enabled != 0,
        })
    }).collect()
}

#[command]
pub async fn create_rule(
    name: String,
    phase: String,
    condition_json: String,
    action_json: String,
    priority: i64,
) -> Result<String, String> {
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO interceptor_rules (id, name, phase, rule_type, condition_json, action_json, priority) VALUES (?, ?, ?, 'custom', ?, ?, ?)",
        id, name, phase, condition_json, action_json, priority,
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(id)
}

#[command]
pub async fn update_rule(id: String, enabled: bool) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query!(
        "UPDATE interceptor_rules SET enabled = ? WHERE id = ?",
        enabled as i64, id
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn delete_rule(id: String) -> Result<(), String> {
    let pool = get_pool().await;
    sqlx::query!("DELETE FROM interceptor_rules WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct DbRule {
    id: String,
    name: String,
    phase: String,
    rule_type: String,
    condition_json: String,
    action_json: String,
    priority: i64,
    enabled: i64,
}
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: add routing, logs, usage, interceptor IPC commands"
```

---

## Phase 5: Vue 3 前端界面

> **UI 设计**: 详细界面设计、配色方案、组件布局由 `frontend-design` 技能在实施时生成。
> 本 Phase 提供基础框架和路由结构，各 view 页面的具体 UI 代码由 frontend-design 产出。

### Task 5.1: 基础布局 & 路由

**Files:**
- Modify: `src/App.vue`, `src/main.ts`
- Create: `src/router.ts`, `src/stores/settings.ts`, `src/types/index.ts`

- [ ] **Step 1: 编写 src/types/index.ts**

```typescript
export interface Provider {
  id: string
  name: string
  base_url: string
  auth_type: string
  auth_header: string
  endpoints: ProviderEndpoint[]
  api_keys: ApiKeyInfo[]
}

export interface ProviderEndpoint {
  id: string
  provider_id: string
  format: 'completions' | 'responses' | 'anthropic' | 'gemini'
  path: string
}

export interface ApiKeyInfo {
  id: string
  label: string
  is_active: boolean
  usage_count: number
  last_used_at: string | null
  created_at: string
}

export interface ModelRoute {
  id: string
  model_pattern: string
  alias: string | null
  provider_id: string
  target_model: string
  target_format: string
  fallback_provider_id: string | null
  priority: number
}

export interface RequestLog {
  id: number
  request_id: string
  client_format: string
  provider_name: string
  provider_format: string
  model: string
  stream: boolean
  status_code: number
  duration_ms: number
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  error_message: string | null
  created_at: string
}

export interface UsageSummary {
  model: string
  provider_name: string
  total_prompt_tokens: number
  total_completion_tokens: number
  total_tokens: number
  total_cost: number
  request_count: number
}

export interface InterceptorRule {
  id: string
  name: string
  phase: 'pre' | 'post'
  condition: RuleCondition
  action: RuleAction
  priority: number
  enabled: boolean
}

export type RuleCondition =
  | { type: 'model_matches'; pattern: string }
  | { type: 'path_contains'; substring: string }
  | { type: 'header_exists'; name: string }
  | { type: 'always' }

export type RuleAction =
  | { type: 'replace_model'; new_model: string }
  | { type: 'set_header'; name: string; value: string }
  | { type: 'remove_header'; name: string }
  | { type: 'inject_system_prompt'; text: string }
  | { type: 'override_parameter'; key: string; value: unknown }
  | { type: 'filter_response'; pattern: string; replacement: string }
```

- [ ] **Step 2: 编写 src/router.ts**

```typescript
import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'Dashboard', component: () => import('./views/Dashboard.vue') },
  { path: '/providers', name: 'Providers', component: () => import('./views/Providers.vue') },
  { path: '/models', name: 'Models', component: () => import('./views/Models.vue') },
  { path: '/logs', name: 'Logs', component: () => import('./views/Logs.vue') },
  { path: '/statistics', name: 'Statistics', component: () => import('./views/Statistics.vue') },
  { path: '/rules', name: 'Rules', component: () => import('./views/Rules.vue') },
  { path: '/settings', name: 'Settings', component: () => import('./views/Settings.vue') },
]

export default createRouter({ history: createWebHashHistory(), routes })
```

- [ ] **Step 3: 更新 src/main.ts**

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import naive from 'naive-ui'
import App from './App.vue'
import router from './router'
import 'vfonts/Inter.css'
import 'vfonts/FiraCode.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(naive)
app.mount('#app')
```

- [ ] **Step 4: 编写 App.vue (主布局)**

```vue
<template>
  <n-config-provider :theme-overrides="themeOverrides">
    <n-layout style="height: 100vh">
      <n-layout-sider bordered collapse-mode="width" :collapsed-width="64" :width="200" :collapsed="collapsed">
        <n-menu
          :collapsed="collapsed"
          :collapsed-width="64"
          :collapsed-icon-size="22"
          :options="menuOptions"
          :value="currentPath"
          @update:value="(v: string) => $router.push(v)"
        />
      </n-layout-sider>
      <n-layout>
        <n-layout-header bordered style="padding: 0 24px; display: flex; align-items: center; justify-content: space-between">
          <n-space>
            <n-button @click="collapsed = !collapsed">
              <template #icon><n-icon><MenuOutline /></n-icon></template>
            </n-button>
            <n-text strong>AI Proxy</n-text>
          </n-space>
          <n-space>
            <n-tag :type="serverStatus === 'running' ? 'success' : 'error'">
              {{ serverStatus === 'running' ? '代理运行中' : '代理已停止' }}
            </n-tag>
          </n-space>
        </n-layout-header>
        <n-layout-content content-style="padding: 24px;">
          <router-view />
        </n-layout-content>
      </n-layout>
    </n-layout>
  </n-config-provider>
</template>

<script setup lang="ts">
import { ref, computed, h } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon } from 'naive-ui'
import {
  HomeOutline, ServerOutline, GitBranchOutline,
  DocumentTextOutline, BarChartOutline, SettingsOutline,
  FilterOutline, MenuOutline,
} from '@vicons/ionicons5'

const router = useRouter()
const collapsed = ref(false)
const serverStatus = ref('running')

const menuOptions = [
  { label: '仪表盘', key: '/', icon: () => h(NIcon, null, () => h(HomeOutline)) },
  { label: '供应商', key: '/providers', icon: () => h(NIcon, null, () => h(ServerOutline)) },
  { label: '模型路由', key: '/models', icon: () => h(NIcon, null, () => h(GitBranchOutline)) },
  { label: '拦截规则', key: '/rules', icon: () => h(NIcon, null, () => h(FilterOutline)) },
  { label: '请求日志', key: '/logs', icon: () => h(NIcon, null, () => h(DocumentTextOutline)) },
  { label: '用量统计', key: '/statistics', icon: () => h(NIcon, null, () => h(BarChartOutline)) },
  { label: '设置', key: '/settings', icon: () => h(NIcon, null, () => h(SettingsOutline)) },
]

const currentPath = computed(() => router.currentRoute.value.path)
</script>
```

- [ ] **Step 5: 安装图标库并验证编译**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
pnpm add @vicons/ionicons5
pnpm dev
```

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: add Vue 3 base layout with navigation"
```

### Task 5.2: 功能视图页面

各 view 使用 `frontend-design` 技能生成详细 UI 代码，功能约束如下：

- **Providers.vue**: 供应商表格 (NDataTable) + 创建/编辑对话框 (NModal + NForm)。API Key 子表格显示后 4 位，支持添加/删除 Key
- **Models.vue**: 模型路由配置，每行包含模型模式、供应商、目标格式、Fallback 选择器。支持通配符模式 (如 `gpt-*`)
- **Logs.vue**: 分页日志表格，支持按模型/状态筛选。点击行弹出详情面板显示请求信息和格式转换对比
- **Statistics.vue**: ECharts 图表 — Token 用量趋势折线图、模型/供应商费用饼图。时间范围选择器 (今日/本周/本月)
- **Rules.vue**: 拦截规则列表，条件/动作配置表单 (条件类型驱动动态表单)，优先级可拖拽排序 (vuedraggable)
- **Settings.vue**: 全局设置表单 — HTTP 主机/端口输入、网络暴露警告 (0.0.0.0 绑定提示)、日志保留天数、代理入口认证开关

- [ ] **Commit**

```bash
git add -A && git commit -m "feat: add all functional view pages"
```

---

## Phase 6: 集成测试 & 验证

### Task 6.1: IR 转换集成测试

**Files:**
- Create: `src-tauri/tests/ir_conversion.rs`

- [ ] **Step 1: 编写 IR 转换测试**

```rust
#[cfg(test)]
mod tests {
    use ai_proxy::converter::ir::*;
    use ai_proxy::converter::parsers::completions::CompletionsParser;
    use ai_proxy::converter::parsers::responses::ResponsesParser;
    use ai_proxy::converter::parsers::anthropic::AnthropicParser;
    use ai_proxy::converter::parsers::gemini::GeminiParser;
    use ai_proxy::converter::generators::completions::CompletionsGenerator;
    use ai_proxy::converter::{FormatParser, FormatGenerator};
    use serde_json::json;

    #[test]
    fn test_completions_to_ir_roundtrip() {
        let input = json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.7,
            "stream": false
        });

        let parser = CompletionsParser;
        let ir = parser.parse_request(&input).unwrap();
        let gen = CompletionsGenerator;
        let output = gen.generate_request(&ir).unwrap();

        assert_eq!(output["model"], "gpt-4o");
        assert_eq!(output["messages"][0]["content"], "You are helpful.");
        assert_eq!(output["temperature"], 0.7);
    }

    #[test]
    fn test_responses_to_completions_conversion() {
        let input = json!({
            "model": "gpt-4o",
            "input": "What is Rust?",
            "instructions": "Be concise"
        });

        let parser = ResponsesParser;
        let ir = parser.parse_request(&input).unwrap();
        let gen = CompletionsGenerator;
        let output = gen.generate_request(&ir).unwrap();

        assert_eq!(output["model"], "gpt-4o");
        assert_eq!(output["messages"][0]["role"], "system");
        assert_eq!(output["messages"][1]["role"], "user");
        assert_eq!(output["messages"][1]["content"], "What is Rust?");
    }

    #[test]
    fn test_anthropic_to_completions_conversion() {
        let input = json!({
            "model": "claude-sonnet-4-5",
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "max_tokens": 4096,
            "stream": false
        });

        let parser = AnthropicParser;
        let ir = parser.parse_request(&input).unwrap();
        let gen = CompletionsGenerator;
        let output = gen.generate_request(&ir).unwrap();

        assert_eq!(output["model"], "claude-sonnet-4-5");
        assert_eq!(output["messages"][0]["role"], "system");
        assert_eq!(output["messages"][0]["content"], "You are helpful.");
        assert_eq!(output["max_tokens"], 4096);
    }

    #[test]
    fn test_stream_chunk_parsing() {
        let line = "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"index\":0}]}";
        let parser = CompletionsParser;
        let chunk = parser.parse_stream_chunk(line).unwrap();
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap().delta_content, Some("Hello".into()));
    }

    #[test]
    fn test_stream_done_detection() {
        let line = "data: [DONE]";
        let parser = CompletionsParser;
        let chunk = parser.parse_stream_chunk(line).unwrap();
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap().finish_reason, Some("stop".into()));
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: 所有测试通过。

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "test: add IR conversion integration tests"
```

---

## 总结

计划共 6 个 Phase、约 15 个 Task，覆盖项目从脚手架到集成测试的完整实施路径：

| Phase | 内容 | Task 数 |
|-------|------|---------|
| 0 | 项目脚手架 | 1 |
| 1 | 基础设施层 (DB, IR, Error) | 2 |
| 2 | 格式转换引擎 (4 Parser + 4 Generator) | 5 |
| 3 | 核心代理引擎 (Provider, Key, Interceptor, Server) | 4 |
| 4 | Tauri IPC 命令 | 2 |
| 5 | Vue 3 前端界面 | 2 |
| 6 | 集成测试 | 1 |
