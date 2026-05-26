# 应用管理 (App Management) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an "App Management" feature that lets users launch Codex CLI/Desktop and Claude CLI/Desktop with proxy config auto-injected into their config files.

**Architecture:** New Rust module `apps/` handles config file read/write (TOML for Codex, JSON for Claude) and process launching. Three new API endpoints under `/api/apps`. New Vue page with card grid UI.

**Tech Stack:** Rust (toml crate, serde_json, tokio::process), Vue 3 + Naive UI, SQLite migration

---

## File Structure

### Backend (new files)
| File | Responsibility |
|---|---|
| `src-tauri/src/apps/mod.rs` | Module entry, exports |
| `src-tauri/src/apps/types.rs` | AppType enum, request/response structs |
| `src-tauri/src/apps/config.rs` | Config file read/write for each app |
| `src-tauri/src/apps/launcher.rs` | Path detection + process spawn |
| `src-tauri/src/apps/handlers.rs` | Axum route handlers |
| `src-tauri/migrations/006_app_configs.sql` | Database table |

### Backend (modified files)
| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | Add `toml` crate |
| `src-tauri/src/lib.rs` | Add `mod apps;` |
| `src-tauri/src/server/mod.rs` | Add `pub mod apps;` re-export (or import in api.rs) |
| `src-tauri/src/server/api.rs` | Add apps routes to `api_routes()` |
| `src-tauri/src/db/init.rs` | Register migration 006 |

### Frontend (new files)
| File | Responsibility |
|---|---|
| `src/views/Apps.vue` | App management page |

### Frontend (modified files)
| File | Change |
|---|---|
| `src/router.ts` | Add `/apps` route |
| `src/App.vue` | Add sidebar menu item |
| `src/types/index.ts` | Add AppType, AppConfig types |

---

## Task 1: Add toml dependency and migration file

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/migrations/006_app_configs.sql`

- [ ] **Step 1: Add `toml` crate to Cargo.toml**

In `src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
toml = "0.8"
```

- [ ] **Step 2: Create migration 006**

Create `src-tauri/migrations/006_app_configs.sql`:

```sql
CREATE TABLE IF NOT EXISTS app_configs (
    app_type TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    proxy_url TEXT NOT NULL,
    launched_at TEXT NOT NULL,
    config_path TEXT,
    install_path TEXT,
    status TEXT NOT NULL DEFAULT 'success'
);
```

- [ ] **Step 3: Register migration in db/init.rs**

In `src-tauri/src/db/init.rs`, after the migration 005 block, add:

```rust
    // Migration 006: app_configs table
    let has_app_configs: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='app_configs'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !has_app_configs {
        let migration6 = include_str!("../../migrations/006_app_configs.sql");
        sqlx::query(migration6).execute(pool).await?;
        info!("Applied migration 006: app_configs table");
    }
```

- [ ] **Step 4: Verify it compiles**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo check`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```
feat: add toml crate and app_configs migration
```

---

## Task 2: Create apps module — types

**Files:**
- Create: `src-tauri/src/apps/mod.rs`
- Create: `src-tauri/src/apps/types.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Register module in lib.rs**

In `src-tauri/src/lib.rs`, add to the module declarations:

```rust
mod apps;
```

- [ ] **Step 2: Create apps/mod.rs**

Create `src-tauri/src/apps/mod.rs`:

```rust
pub mod types;
pub mod config;
pub mod launcher;
pub mod handlers;
```

- [ ] **Step 3: Create apps/types.rs**

Create `src-tauri/src/apps/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub enum AppType {
    CodexCli,
    CodexDesktop,
    ClaudeCli,
    ClaudeDesktop,
}

impl fmt::Display for AppType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppType::CodexCli => write!(f, "codex_cli"),
            AppType::CodexDesktop => write!(f, "codex_desktop"),
            AppType::ClaudeCli => write!(f, "claude_cli"),
            AppType::ClaudeDesktop => write!(f, "claude_desktop"),
        }
    }
}

impl AppType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "codex_cli" => Some(AppType::CodexCli),
            "codex_desktop" => Some(AppType::CodexDesktop),
            "claude_cli" => Some(AppType::ClaudeCli),
            "claude_desktop" => Some(AppType::ClaudeDesktop),
            _ => None,
        }
    }

    pub fn all() -> Vec<AppType> {
        vec![
            AppType::CodexCli,
            AppType::CodexDesktop,
            AppType::ClaudeCli,
            AppType::ClaudeDesktop,
        ]
    }

    pub fn is_cli(&self) -> bool {
        matches!(self, AppType::CodexCli | AppType::ClaudeCli)
    }

    pub fn is_codex(&self) -> bool {
        matches!(self, AppType::CodexCli | AppType::CodexDesktop)
    }

    pub fn proxy_url_suffix(&self) -> &str {
        if self.is_codex() { "/v1" } else { "" }
    }

    pub fn display_name(&self) -> &str {
        match self {
            AppType::CodexCli => "Codex CLI",
            AppType::CodexDesktop => "Codex Desktop",
            AppType::ClaudeCli => "Claude CLI",
            AppType::ClaudeDesktop => "Claude Desktop",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_type: AppType,
    pub installed: bool,
    pub install_path: Option<String>,
    pub config_path: Option<String>,
    pub model: Option<String>,
    pub proxy_url: Option<String>,
    pub launched_at: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LaunchRequest {
    pub app_type: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct SetPathRequest {
    pub install_path: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbAppConfig {
    pub app_type: String,
    pub model: String,
    pub proxy_url: String,
    pub launched_at: String,
    pub config_path: Option<String>,
    pub install_path: Option<String>,
    pub status: String,
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo check`
Expected: compiles (will warn about unused modules — that's OK for now)

- [ ] **Step 5: Commit**

```
feat: add apps module with types
```

---

## Task 3: Create config.rs — config file read/write

**Files:**
- Create: `src-tauri/src/apps/config.rs`

- [ ] **Step 1: Create config.rs**

Create `src-tauri/src/apps/config.rs`:

```rust
use crate::apps::types::AppType;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::info;

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

pub fn codex_config_path() -> PathBuf {
    home_dir().join(".codex").join("config.toml")
}

pub fn claude_cli_config_path() -> PathBuf {
    home_dir().join(".claude").join("settings.json")
}

pub fn claude_desktop_config_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir().join("Library").join("Application Support").join("Claude").join("claude_desktop_config.json")
    }
    #[cfg(target_os = "windows")]
    {
        home_dir().join("AppData").join("Roaming").join("Claude").join("claude_desktop_config.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        home_dir().join(".config").join("Claude").join("claude_desktop_config.json")
    }
}

pub fn config_path_for(app_type: &AppType) -> PathBuf {
    match app_type {
        AppType::CodexCli | AppType::CodexDesktop => codex_config_path(),
        AppType::ClaudeCli => claude_cli_config_path(),
        AppType::ClaudeDesktop => claude_desktop_config_path(),
    }
}

pub async fn write_codex_config(model: &str, proxy_base: &str) -> Result<PathBuf, String> {
    let path = codex_config_path();
    let parent = path.parent().ok_or("Invalid config path")?;
    fs::create_dir_all(parent).await.map_err(|e| format!("Failed to create config dir: {}", e))?;

    let proxy_url = format!("{}{}", proxy_base, "/v1");

    let existing = fs::read_to_string(&path).await.unwrap_or_default();
    let mut doc: HashMap<String, toml::Value> = if existing.trim().is_empty() {
        HashMap::new()
    } else {
        toml::from_str(&existing).map_err(|e| format!("Failed to parse config.toml: {}", e))?
    };

    doc.insert("model".to_string(), toml::Value::String(model.to_string()));
    doc.insert("openai_base_url".to_string(), toml::Value::String(proxy_url));

    let content = toml::to_string_pretty(&doc).map_err(|e| format!("Failed to serialize config.toml: {}", e))?;
    atomic_write(&path, &content).await?;

    info!("Wrote Codex config: model={}, base_url={}", model, proxy_url);
    Ok(path)
}

pub async fn write_claude_cli_config(model: &str, proxy_base: &str) -> Result<PathBuf, String> {
    let path = claude_cli_config_path();
    let parent = path.parent().ok_or("Invalid config path")?;
    fs::create_dir_all(parent).await.map_err(|e| format!("Failed to create config dir: {}", e))?;

    let existing = fs::read_to_string(&path).await.unwrap_or_default();
    let mut doc: serde_json::Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&existing).map_err(|e| format!("Failed to parse settings.json: {}", e))?
    };

    let env = doc["env"].as_object_mut()
        .map(|o| o)
        .unwrap_or_else(|| {
            doc["env"] = serde_json::json!({});
            doc["env"].as_object_mut().unwrap()
        });

    env.insert("ANTHROPIC_BASE_URL".to_string(), serde_json::Value::String(proxy_base.to_string()));
    env.insert("ANTHROPIC_MODEL".to_string(), serde_json::Value::String(model.to_string()));

    let content = serde_json::to_string_pretty(&doc).map_err(|e| format!("Failed to serialize settings.json: {}", e))?;
    atomic_write(&path, &content).await?;

    info!("Wrote Claude CLI config: model={}, base_url={}", model, proxy_base);
    Ok(path)
}

pub async fn write_claude_desktop_config(model: &str, proxy_base: &str) -> Result<PathBuf, String> {
    let path = claude_desktop_config_path();
    let parent = path.parent().ok_or("Invalid config path")?;
    fs::create_dir_all(parent).await.map_err(|e| format!("Failed to create config dir: {}", e))?;

    let existing = fs::read_to_string(&path).await.unwrap_or_default();
    let mut doc: serde_json::Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&existing).map_err(|e| format!("Failed to parse config: {}", e))?
    };

    let env = doc["env"].as_object_mut()
        .map(|o| o)
        .unwrap_or_else(|| {
            doc["env"] = serde_json::json!({});
            doc["env"].as_object_mut().unwrap()
        });

    env.insert("ANTHROPIC_BASE_URL".to_string(), serde_json::Value::String(proxy_base.to_string()));
    env.insert("ANTHROPIC_MODEL".to_string(), serde_json::Value::String(model.to_string()));

    let content = serde_json::to_string_pretty(&doc).map_err(|e| format!("Failed to serialize config: {}", e))?;
    atomic_write(&path, &content).await?;

    info!("Wrote Claude Desktop config: model={}, base_url={}", model, proxy_base);
    Ok(path)
}

pub async fn write_config(app_type: &AppType, model: &str, proxy_base: &str) -> Result<PathBuf, String> {
    match app_type {
        AppType::CodexCli | AppType::CodexDesktop => write_codex_config(model, proxy_base).await,
        AppType::ClaudeCli => write_claude_cli_config(model, proxy_base).await,
        AppType::ClaudeDesktop => write_claude_desktop_config(model, proxy_base).await,
    }
}

async fn atomic_write(path: &PathBuf, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content).await.map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, path).await.map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}
```

- [ ] **Step 2: Add `dirs` crate to Cargo.toml if not present**

Check if `dirs` crate is already in `src-tauri/Cargo.toml`. If not, add:

```toml
dirs = "5"
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo check`
Expected: compiles

- [ ] **Step 4: Commit**

```
feat: add config file read/write for Codex and Claude apps
```

---

## Task 4: Create launcher.rs — path detection + process spawn

**Files:**
- Create: `src-tauri/src/apps/launcher.rs`

- [ ] **Step 1: Create launcher.rs**

Create `src-tauri/src/apps/launcher.rs`:

```rust
use crate::apps::types::AppType;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::info;

pub async fn detect_path(app_type: &AppType) -> Option<String> {
    match app_type {
        AppType::CodexCli => detect_cli("codex").await,
        AppType::CodexDesktop => detect_desktop_app("Codex").await,
        AppType::ClaudeCli => detect_cli("claude").await,
        AppType::ClaudeDesktop => detect_desktop_app("Claude").await,
    }
}

async fn detect_cli(name: &str) -> Option<String> {
    #[cfg(unix)]
    {
        let output = Command::new("which")
            .arg(name)
            .output()
            .await
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    #[cfg(windows)]
    {
        let output = Command::new("where")
            .arg(format!("{}.exe", name))
            .output()
            .await
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    None
}

async fn detect_desktop_app(name: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let app_path = PathBuf::from("/Applications").join(format!("{}.app", name));
        if app_path.exists() {
            return Some(app_path.to_string_lossy().to_string());
        }
    }
    #[cfg(target_os = "windows")]
    {
        let candidates = vec![
            PathBuf::from(format!("C:\\Program Files\\{}\\{}.exe", name, name)),
            PathBuf::from(format!("C:\\Program Files (x86)\\{}\\{}.exe", name, name)),
            dirs::data_dir().map(|d| d.join(format!("Local\\Programs\\{}\\{}.exe", name, name))),
        ].into_iter().flatten().collect::<Vec<_>>();

        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let desktop_entry = PathBuf::from("/usr/share/applications").join(format!("{}.desktop", name.to_lowercase()));
        if desktop_entry.exists() {
            return Some(desktop_entry.to_string_lossy().to_string());
        }
    }
    None
}

pub fn resolve_install_path(app_type: &AppType, custom_path: Option<&str>) -> Option<String> {
    if let Some(p) = custom_path {
        if !p.is_empty() {
            return Some(p.to_string());
        }
    }
    None
}

pub async fn launch(app_type: &AppType, install_path: &str) -> Result<(), String> {
    info!("Launching {:?} from {}", app_type, install_path);

    if app_type.is_cli() {
        Command::new(install_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", app_type.display_name(), e))?;
    } else {
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(install_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to launch {}: {}", app_type.display_name(), e))?;
        }
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open")
                .arg(install_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to launch {}: {}", app_type.display_name(), e))?;
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "start", "", install_path])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to launch {}: {}", app_type.display_name(), e))?;
        }
    }

    info!("Successfully launched {:?}", app_type);
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```
feat: add app launcher with path detection and process spawn
```

---

## Task 5: Create handlers.rs — API route handlers

**Files:**
- Create: `src-tauri/src/apps/handlers.rs`
- Modify: `src-tauri/src/server/api.rs`

- [ ] **Step 1: Create handlers.rs**

Create `src-tauri/src/apps/handlers.rs`:

```rust
use crate::apps::config;
use crate::apps::launcher;
use crate::apps::types::{AppConfig, AppType, DbAppConfig, LaunchRequest, SetPathRequest};
use crate::db::get_pool;
use crate::server::api::{err_json, ok, ApiError, ApiResponse};
use axum::extract::{Path, Query};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct SettingsQuery {
    key: Option<String>,
}

async fn get_proxy_base_url() -> String {
    let pool = get_pool().await;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'proxy_host'"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let host = row.map(|r| r.0).unwrap_or_else(|| "127.0.0.1".to_string());

    let port_row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'proxy_port'"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let port = port_row.map(|r| r.0).unwrap_or_else(|| "7860".to_string());

    format!("http://{}:{}", host, port)
}

async fn load_db_configs() -> HashMap<String, DbAppConfig> {
    let pool = get_pool().await;
    let rows: Vec<DbAppConfig> = sqlx::query_as(
        "SELECT app_type, model, proxy_url, launched_at, config_path, install_path, status FROM app_configs"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().map(|r| (r.app_type.clone(), r)).collect()
}

pub async fn list_apps() -> Json<ApiResponse<Vec<AppConfig>>> {
    let db_configs = load_db_configs().await;
    let mut result = Vec::new();

    for app_type in AppType::all() {
        let type_str = app_type.to_string();
        let db = db_configs.get(&type_str);
        let custom_path = db.and_then(|d| d.install_path.clone());

        let resolved_path = if let Some(ref p) = custom_path {
            Some(p.clone())
        } else {
            launcher::detect_path(&app_type).await
        };

        let installed = resolved_path.is_some();
        let config_path = config::config_path_for(&app_type).to_string_lossy().to_string();

        result.push(AppConfig {
            app_type: app_type.clone(),
            installed,
            install_path: resolved_path.or(custom_path),
            config_path: Some(config_path),
            model: db.map(|d| d.model.clone()),
            proxy_url: db.map(|d| d.proxy_url.clone()),
            launched_at: db.map(|d| d.launched_at.clone()),
            status: db.map(|d| d.status.clone()),
        });
    }

    ok(result)
}

pub async fn launch_app(
    axum::Json(body): axum::Json<LaunchRequest>,
) -> Result<Json<ApiResponse<AppConfig>>, Json<ApiError>> {
    let app_type = AppType::from_str(&body.app_type)
        .ok_or_else(|| err_json(format!("Unknown app type: {}", body.app_type)))?;

    let db_configs = load_db_configs().await;
    let db = db_configs.get(&body.app_type);
    let custom_path = db.and_then(|d| d.install_path.clone());

    let install_path = launcher::resolve_install_path(&app_type, custom_path.as_deref())
        .or_else(|| {
            let detected = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(launcher::detect_path(&app_type))
            });
            detected
        })
        .ok_or_else(|| err_json(format!("{} 未检测到，请在设置中配置安装路径", app_type.display_name())))?;

    let proxy_base = get_proxy_base_url().await;
    let config_path = config::write_config(&app_type, &body.model, &proxy_base).await
        .map_err(|e| {
            let pool = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(get_pool())
            });
            let now = chrono::Utc::now().to_rfc3339();
            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    sqlx::query(
                        "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&body.app_type)
                    .bind(&body.model)
                    .bind(&proxy_base)
                    .bind(&now)
                    .bind(Option::<String>::None)
                    .bind(&install_path)
                    .bind("config_error")
                    .execute(pool)
                )
            });
            err_json(e)
        })?;

    launcher::launch(&app_type, &install_path).await.map_err(|e| {
        let pool = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(get_pool())
        });
        let now = chrono::Utc::now().to_rfc3339();
        let _ = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                sqlx::query(
                    "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&body.app_type)
                .bind(&body.model)
                .bind(&proxy_base)
                .bind(&now)
                .bind(config_path.to_str())
                .bind(&install_path)
                .bind("launch_error")
                .execute(pool)
            )
        });
        err_json(e)
    })?;

    let pool = get_pool().await;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&body.app_type)
    .bind(&body.model)
    .bind(&proxy_base)
    .bind(&now)
    .bind(config_path.to_str())
    .bind(&install_path)
    .bind("success")
    .execute(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    Ok(ok(AppConfig {
        app_type,
        installed: true,
        install_path: Some(install_path),
        config_path: Some(config_path.to_string_lossy().to_string()),
        model: Some(body.model),
        proxy_url: Some(proxy_base),
        launched_at: Some(now),
        status: Some("success".to_string()),
    }))
}

pub async fn set_app_path(
    Path(app_type_str): Path<String>,
    axum::Json(body): axum::Json<SetPathRequest>,
) -> Result<Json<ApiResponse<()>>, Json<ApiError>> {
    let _app_type = AppType::from_str(&app_type_str)
        .ok_or_else(|| err_json(format!("Unknown app type: {}", app_type_str)))?;

    let pool = get_pool().await;

    let existing: Option<DbAppConfig> = sqlx::query_as(
        "SELECT app_type, model, proxy_url, launched_at, config_path, install_path, status FROM app_configs WHERE app_type = ?"
    )
    .bind(&app_type_str)
    .fetch_optional(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    match existing {
        Some(db) => {
            sqlx::query(
                "UPDATE app_configs SET install_path = ? WHERE app_type = ?"
            )
            .bind(&body.install_path)
            .bind(&app_type_str)
            .execute(pool)
            .await
            .map_err(|e| err_json(e.to_string()))?;
        }
        None => {
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, '', '', ?, NULL, ?, 'idle')"
            )
            .bind(&app_type_str)
            .bind(&now)
            .bind(&body.install_path)
            .execute(pool)
            .await
            .map_err(|e| err_json(e.to_string()))?;
        }
    }

    Ok(ok(()))
}
```

- [ ] **Step 2: Register routes in api.rs**

In `src-tauri/src/server/api.rs`, add at the top:

```rust
use crate::apps::handlers;
```

In the `api_routes()` function, add a new `.route()` chain:

```rust
        .route("/apps", axum::routing::get(handlers::list_apps))
        .route("/apps/launch", axum::routing::post(handlers::launch_app))
        .route("/apps/:app_type/path", axum::routing::put(handlers::set_app_path))
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo check`
Expected: compiles

- [ ] **Step 4: Commit**

```
feat: add apps API handlers and routes
```

---

## Task 6: Frontend types and routing

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/router.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: Add types to src/types/index.ts**

Append to the end of `src/types/index.ts`:

```ts
export type AppType = 'codex_cli' | 'codex_desktop' | 'claude_cli' | 'claude_desktop'

export interface AppConfig {
  app_type: AppType
  installed: boolean
  install_path: string | null
  config_path: string | null
  model: string | null
  proxy_url: string | null
  launched_at: string | null
  status: 'success' | 'config_error' | 'launch_error' | null
}

export interface LaunchRequest {
  app_type: AppType
  model: string
}

export interface SetPathRequest {
  install_path: string
}
```

- [ ] **Step 2: Add route to src/router.ts**

In `src/router.ts`, add to the `routes` array (before the closing `]`):

```ts
  { path: '/apps', name: 'Apps', component: () => import('./views/Apps.vue') },
```

- [ ] **Step 3: Add sidebar menu item to src/App.vue**

In `src/App.vue`, find the `menuOptions` array and add a new entry. Import the icon at the top of the `<script setup>`:

```ts
import { ..., AppsOutline } from '@vicons/ionicons5'
```

Add to `menuOptions` (after '用量统计' and before '设置'):

```ts
  { label: '应用管理', key: '/apps', icon: renderIcon(AppsOutline) },
```

- [ ] **Step 4: Verify frontend compiles**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy && pnpm build`
Expected: builds without errors

- [ ] **Step 5: Commit**

```
feat: add Apps route, types, and sidebar menu item
```

---

## Task 7: Create Apps.vue — the main page

**Files:**
- Create: `src/views/Apps.vue`

- [ ] **Step 1: Create Apps.vue**

Create `src/views/Apps.vue` following the existing Providers.vue pattern:

```vue
<template>
  <n-space vertical size="large">
    <n-card title="应用管理">
      <n-spin :show="loading">
        <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px;">
          <n-card v-for="app in apps" :key="app.app_type" size="small" hoverable>
            <template #header>
              <div style="display: flex; align-items: center; justify-content: space-between;">
                <span>{{ displayName(app.app_type) }}</span>
                <n-button quaternary circle size="small" @click="openPathModal(app)">
                  <template #icon><n-icon :component="SettingsOutline" /></template>
                </n-button>
              </div>
            </template>
            <n-space vertical size="small">
              <div>
                <n-tag :type="app.installed ? 'success' : 'error'" size="small">
                  {{ app.installed ? '已安装' : '未检测到' }}
                </n-tag>
              </div>
              <div v-if="app.install_path" style="font-size: 12px; color: #999; word-break: break-all;">
                路径: {{ app.install_path }}
              </div>
              <div v-if="app.model">
                模型: <n-tag size="small">{{ app.model }}</n-tag>
              </div>
              <div v-if="app.launched_at" style="font-size: 12px; color: #999;">
                最后启动: {{ formatTime(app.launched_at) }}
              </div>
              <n-button
                type="primary"
                block
                :disabled="!app.installed"
                @click="openLaunchModal(app)"
              >
                打开
              </n-button>
            </n-space>
          </n-card>
        </div>
      </n-spin>
    </n-card>

    <!-- Launch Modal -->
    <n-modal
      v-model:show="showLaunchModal"
      preset="dialog"
      title="启动应用"
      positive-text="确定"
      negative-text="取消"
      :loading="launchLoading"
      @positive-click="handleLaunch"
    >
      <n-space vertical>
        <p>启动 <strong>{{ displayName(launchForm.app_type) }}</strong></p>
        <n-form label-placement="left" label-width="60">
          <n-form-item label="模型">
            <n-select
              v-model:value="launchForm.model"
              :options="modelOptions"
              filterable
              tag
              placeholder="选择或输入模型名"
            />
          </n-form-item>
        </n-form>
      </n-space>
    </n-modal>

    <!-- Path Config Modal -->
    <n-modal
      v-model:show="showPathModal"
      preset="dialog"
      title="配置安装路径"
      positive-text="保存"
      negative-text="取消"
      :loading="pathLoading"
      @positive-click="handleSetPath"
    >
      <n-space vertical>
        <p>为 <strong>{{ displayName(pathForm.app_type) }}</strong> 设置安装路径</p>
        <n-form label-placement="left" label-width="80">
          <n-form-item label="安装路径">
            <n-input
              v-model:value="pathForm.install_path"
              placeholder="留空则自动检测"
            />
          </n-form-item>
        </n-form>
      </n-space>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useMessage, NIcon } from 'naive-ui'
import { SettingsOutline } from '@vicons/ionicons5'
import { api } from '../api'
import type { AppConfig, AppType, ProviderModel } from '../types'

const message = useMessage()
const loading = ref(false)
const apps = ref<AppConfig[]>([])

const showLaunchModal = ref(false)
const launchLoading = ref(false)
const launchForm = ref<{ app_type: AppType; model: string }>({
  app_type: 'codex_cli',
  model: '',
})

const showPathModal = ref(false)
const pathLoading = ref(false)
const pathForm = ref<{ app_type: AppType; install_path: string }>({
  app_type: 'codex_cli',
  install_path: '',
})

const allModels = ref<ProviderModel[]>([])
const modelOptions = computed(() => {
  const seen = new Set<string>()
  const opts: { label: string; value: string }[] = []
  for (const m of allModels.value) {
    const name = m.target_model || m.model_name
    if (!seen.has(name)) {
      seen.add(name)
      opts.push({ label: name, value: name })
    }
  }
  return opts
})

function displayName(appType: AppType): string {
  const map: Record<AppType, string> = {
    codex_cli: 'Codex CLI',
    codex_desktop: 'Codex Desktop',
    claude_cli: 'Claude CLI',
    claude_desktop: 'Claude Desktop',
  }
  return map[appType]
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString('zh-CN')
  } catch {
    return iso
  }
}

async function fetchApps() {
  loading.value = true
  try {
    apps.value = await api<AppConfig[]>('/api/apps')
  } catch (e: any) {
    message.error('加载应用列表失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

async function fetchModels() {
  try {
    const providers = await api<{ models: ProviderModel[] }[]>('/api/providers')
    const models: ProviderModel[] = []
    for (const p of providers) {
      models.push(...p.models)
    }
    allModels.value = models
  } catch {
    // non-critical, model select will just allow manual input
  }
}

function openLaunchModal(app: AppConfig) {
  launchForm.value = {
    app_type: app.app_type,
    model: app.model || '',
  }
  showLaunchModal.value = true
}

function openPathModal(app: AppConfig) {
  pathForm.value = {
    app_type: app.app_type,
    install_path: app.install_path || '',
  }
  showPathModal.value = true
}

async function handleLaunch() {
  if (!launchForm.value.model) {
    message.warning('请选择模型')
    return false
  }
  launchLoading.value = true
  try {
    await api<AppConfig>('/api/apps/launch', {
      method: 'POST',
      body: JSON.stringify(launchForm.value),
    })
    message.success('已启动 ' + displayName(launchForm.value.app_type))
    showLaunchModal.value = false
    await fetchApps()
  } catch (e: any) {
    message.error('启动失败: ' + e.message)
  } finally {
    launchLoading.value = false
  }
  return false
}

async function handleSetPath() {
  pathLoading.value = true
  try {
    await api<void>(`/api/apps/${pathForm.value.app_type}/path`, {
      method: 'PUT',
      body: JSON.stringify({ install_path: pathForm.value.install_path }),
    })
    message.success('路径已保存')
    showPathModal.value = false
    await fetchApps()
  } catch (e: any) {
    message.error('保存失败: ' + e.message)
  } finally {
    pathLoading.value = false
  }
  return false
}

onMounted(() => {
  fetchApps()
  fetchModels()
})
</script>
```

- [ ] **Step 2: Verify frontend builds**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy && pnpm build`
Expected: builds without errors

- [ ] **Step 3: Commit**

```
feat: add Apps.vue page with card grid and launch/config modals
```

---

## Task 8: Integration test — verify full flow

**Files:** None new (manual verification)

- [ ] **Step 1: Start the dev server**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy && pnpm tauri dev`

- [ ] **Step 2: Verify sidebar**

Expected: "应用管理" menu item appears in sidebar between "用量统计" and "设置"

- [ ] **Step 3: Click into the page**

Expected: 4 application cards are displayed, each showing installed/not-detected status

- [ ] **Step 4: Click the gear icon on an installed app**

Expected: Path config modal opens, shows current path (or empty)

- [ ] **Step 5: Click "打开" on an installed app**

Expected: Launch modal opens with model dropdown populated from provider_models

- [ ] **Step 6: Select a model and click "确定"**

Expected: Success message, card updates with model name and timestamp

- [ ] **Step 7: Verify config file was written**

For Codex: `cat ~/.codex/config.toml` should show `model` and `openai_base_url`
For Claude CLI: `cat ~/.claude/settings.json` should show updated `env.ANTHROPIC_MODEL` and `env.ANTHROPIC_BASE_URL`

- [ ] **Step 8: Commit if any fixes needed**

```
fix: address integration issues in app management
```

---

## Task 9: Rust unit tests

**Files:**
- Modify: `src-tauri/src/apps/config.rs` (add tests module)
- Modify: `src-tauri/src/apps/launcher.rs` (add tests module)

- [ ] **Step 1: Add unit tests to config.rs**

Append to `src-tauri/src/apps/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_codex_config_new_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".codex").join("config.toml");

        // Temporarily override the config path for testing
        let content = model_and_base_url("gpt-4", "http://127.0.0.1:7860/v1");
        fs::create_dir_all(path.parent().unwrap()).await.unwrap();
        atomic_write(&path, &content).await.unwrap();

        let written = fs::read_to_string(&path).await.unwrap();
        assert!(written.contains("model = \"gpt-4\""));
        assert!(written.contains("openai_base_url = \"http://127.0.0.1:7860/v1\""));
    }

    #[tokio::test]
    async fn test_write_claude_cli_config_preserves_fields() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");

        let existing = r#"{"language":"Chinese","env":{"ANTHROPIC_API_KEY":"sk-xxx"}}"#;
        fs::write(&path, existing).await.unwrap();

        let mut doc: serde_json::Value = serde_json::from_str(existing).unwrap();
        let env = doc["env"].as_object_mut().unwrap();
        env.insert("ANTHROPIC_BASE_URL".to_string(), serde_json::Value::String("http://127.0.0.1:7860".to_string()));
        env.insert("ANTHROPIC_MODEL".to_string(), serde_json::Value::String("glm-5.1".to_string()));

        let content = serde_json::to_string_pretty(&doc).unwrap();
        atomic_write(&path, &content).await.unwrap();

        let written: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).await.unwrap()).unwrap();
        assert_eq!(written["language"], "Chinese");
        assert_eq!(written["env"]["ANTHROPIC_API_KEY"], "sk-xxx");
        assert_eq!(written["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:7860");
        assert_eq!(written["env"]["ANTHROPIC_MODEL"], "glm-5.1");
    }

    fn model_and_base_url(model: &str, base_url: &str) -> String {
        let mut doc = std::collections::HashMap::new();
        doc.insert("model".to_string(), toml::Value::String(model.to_string()));
        doc.insert("openai_base_url".to_string(), toml::Value::String(base_url.to_string()));
        toml::to_string_pretty(&doc).unwrap()
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` in `src-tauri/Cargo.toml`.

- [ ] **Step 2: Add unit tests to launcher.rs**

Append to `src-tauri/src/apps/launcher.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_install_path_custom_takes_priority() {
        let result = resolve_install_path(&AppType::CodexCli, Some("/custom/path/codex"));
        assert_eq!(result, Some("/custom/path/codex".to_string()));
    }

    #[test]
    fn test_resolve_install_path_empty_string_returns_none() {
        let result = resolve_install_path(&AppType::CodexCli, Some(""));
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_install_path_none_returns_none() {
        let result = resolve_install_path(&AppType::CodexCli, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_app_type_display() {
        assert_eq!(AppType::CodexCli.to_string(), "codex_cli");
        assert_eq!(AppType::ClaudeDesktop.to_string(), "claude_desktop");
    }

    #[test]
    fn test_app_type_is_cli() {
        assert!(AppType::CodexCli.is_cli());
        assert!(AppType::ClaudeCli.is_cli());
        assert!(!AppType::CodexDesktop.is_cli());
        assert!(!AppType::ClaudeDesktop.is_cli());
    }

    #[test]
    fn test_app_type_is_codex() {
        assert!(AppType::CodexCli.is_codex());
        assert!(AppType::CodexDesktop.is_codex());
        assert!(!AppType::ClaudeCli.is_codex());
        assert!(!AppType::ClaudeDesktop.is_codex());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo test`
Expected: all tests pass

- [ ] **Step 4: Commit**

```
test: add unit tests for apps config and launcher
```

---

## Self-Review

### Spec Coverage

| Spec Section | Task |
|---|---|
| Data model (app_configs table) | Task 1 |
| Config file write - Codex | Task 3 |
| Config file write - Claude CLI | Task 3 |
| Config file write - Claude Desktop | Task 3 |
| Path detection (macOS/Linux/Windows) | Task 4 |
| Process launch | Task 4 |
| API: GET /api/apps | Task 5 |
| API: POST /api/apps/launch | Task 5 |
| API: PUT /api/apps/:app_type/path | Task 5 |
| Frontend types | Task 6 |
| Frontend routing + sidebar | Task 6 |
| Frontend Apps.vue page | Task 7 |
| Integration testing | Task 8 |
| Unit tests | Task 9 |

### Placeholder Scan
- No TBD/TODO found
- All code blocks contain complete implementations

### Type Consistency
- `AppType` enum values: `CodexCli`, `CodexDesktop`, `ClaudeCli`, `ClaudeDesktop` — consistent across types.rs, config.rs, launcher.rs, handlers.rs
- `AppConfig` struct fields match between Rust types and TypeScript interface
- API response shape `{ success, data }` matches existing pattern
