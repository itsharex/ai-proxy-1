use axum::extract::{Path, Json};
use std::collections::HashMap;

use crate::apps::config;
use crate::apps::launcher;
use crate::apps::types::{AppConfig, AppType, DbAppConfig, LaunchRequest, SetPathRequest};
use crate::db::get_pool;
use crate::key::rotation::{KeyRotation, RotationStrategy};
use crate::key::store::decrypt_api_key;
use crate::server::api::{ok, err_json, ApiError, ApiResponse};

fn build_model_config(body: &LaunchRequest) -> Option<String> {
    if body.model_haiku.is_none() && body.model_sonnet.is_none() && body.model_opus.is_none() {
        return None;
    }
    let mut map = serde_json::Map::new();
    if let Some(ref v) = body.model_haiku { map.insert("haiku".into(), serde_json::Value::String(v.clone())); }
    if let Some(ref v) = body.model_sonnet { map.insert("sonnet".into(), serde_json::Value::String(v.clone())); }
    if let Some(ref v) = body.model_opus { map.insert("opus".into(), serde_json::Value::String(v.clone())); }
    Some(serde_json::Value::Object(map).to_string())
}

fn parse_model_config(json: Option<&str>) -> (Option<String>, Option<String>, Option<String>) {
    json
        .and_then(|s| serde_json::from_str::<HashMap<String, String>>(s).ok())
        .map(|m| (m.get("haiku").cloned(), m.get("sonnet").cloned(), m.get("opus").cloned()))
        .unwrap_or((None, None, None))
}

pub async fn list_apps() -> Json<ApiResponse<Vec<AppConfig>>> {
    let pool = get_pool().await;

    let rows: Vec<DbAppConfig> = sqlx::query_as(
        "SELECT app_type, model, proxy_url, launched_at, config_path, install_path, status, work_dir, model_config FROM app_configs",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let db_map: HashMap<String, DbAppConfig> = rows
        .into_iter()
        .map(|r| (r.app_type.clone(), r))
        .collect();

    let mut result: Vec<AppConfig> = Vec::new();

    for app_type in AppType::all() {
        let key = app_type.to_string();
        let db_rec = db_map.get(&key);

        let custom_path = db_rec.and_then(|r| r.install_path.as_deref());
        let detected_path = launcher::detect_path(&app_type).await;
        let install_path = custom_path
            .filter(|p| !p.trim().is_empty())
            .map(|p| p.to_string())
            .or(detected_path);

        let installed = install_path.is_some();
        let config_path_str = config::config_path_for(&app_type).to_string_lossy().to_string();
        let (model_haiku, model_sonnet, model_opus) = parse_model_config(
            db_rec.as_ref().and_then(|r| r.model_config.as_deref())
        );

        let app_config = AppConfig {
            app_type,
            installed,
            install_path,
            config_path: Some(config_path_str),
            model: db_rec.map(|r| {
                if r.model.is_empty() {
                    None
                } else {
                    Some(r.model.clone())
                }
            }).unwrap_or(None),
            proxy_url: db_rec.map(|r| {
                if r.proxy_url.is_empty() {
                    None
                } else {
                    Some(r.proxy_url.clone())
                }
            }).unwrap_or(None),
            launched_at: db_rec.map(|r| {
                if r.launched_at.is_empty() {
                    None
                } else {
                    Some(r.launched_at.clone())
                }
            }).unwrap_or(None),
            status: db_rec.map(|r| {
                if r.status.is_empty() {
                    None
                } else {
                    Some(r.status.clone())
                }
            }).unwrap_or(None),
            model_haiku,
            model_sonnet,
            model_opus,
            work_dir: db_rec.as_ref().and_then(|r| {
                if r.work_dir.as_ref().map_or(true, |s| s.is_empty()) {
                    None
                } else {
                    r.work_dir.clone()
                }
            }),
        };

        result.push(app_config);
    }

    ok(result)
}

pub async fn launch_app(
    Json(body): Json<LaunchRequest>,
) -> Result<Json<ApiResponse<AppConfig>>, Json<ApiError>> {
    let app_type = AppType::from_str(&body.app_type)
        .ok_or_else(|| err_json(format!("Unknown app type: {}", body.app_type)))?;

    let pool = get_pool().await;

    // Resolve install path: custom from DB -> auto-detect -> error
    let db_rec: Option<DbAppConfig> = sqlx::query_as(
        "SELECT app_type, model, proxy_url, launched_at, config_path, install_path, status, work_dir, model_config FROM app_configs WHERE app_type = ?",
    )
    .bind(&body.app_type)
    .fetch_optional(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    let custom_path = db_rec.as_ref().and_then(|r| r.install_path.as_deref());
    let detected_path = launcher::detect_path(&app_type).await;
    let install_path = custom_path
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.to_string())
        .or(detected_path)
        .ok_or_else(|| err_json(format!("{} is not installed or path not detected", app_type.display_name())))?;

    // Get proxy base URL from settings
    let proxy_settings: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE key = 'http_port'",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    let settings_map: HashMap<String, String> = proxy_settings.into_iter().collect();
    let port = settings_map.get("http_port").cloned().unwrap_or_else(|| "7860".into());
    let proxy_base = format!("http://127.0.0.1:{}", port);

    let proxy_url = format!("{}{}", proxy_base, app_type.proxy_url_suffix());
    let now = chrono::Utc::now().to_rfc3339();

    // Write config file
    let model_haiku = body.model_haiku.as_deref();
    let model_sonnet = body.model_sonnet.as_deref();
    let model_opus = body.model_opus.as_deref();
    let model_config_json = build_model_config(&body);

    // Resolve an API key: Claude uses upstream anthropic key; Codex uses proxy auth key
    let api_key = if app_type.is_claude() {
        resolve_api_key_for_claude(&app_type).await.unwrap_or_default()
    } else {
        resolve_proxy_auth_key().await.unwrap_or_default()
    };

    if let Err(e) = config::write_config(
        &app_type,
        &body.model,
        model_haiku,
        model_sonnet,
        model_opus,
        &proxy_url,
        &api_key,
    )
    .await
    {
        // Save error status to DB
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status, work_dir, model_config) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&body.app_type)
        .bind(&body.model)
        .bind(&proxy_url)
        .bind(&now)
        .bind(Option::<String>::None)
        .bind(&install_path)
        .bind("config_error")
        .bind(&body.work_dir)
        .bind(&model_config_json)
        .execute(pool)
        .await;

        return Err(err_json(format!("Failed to write config: {}", e)));
    }

    let config_path = config::config_path_for(&app_type).to_string_lossy().to_string();

    // Launch the app
    let work_dir = body.work_dir.as_deref();
    if let Err(e) = launcher::launch(&app_type, &install_path, work_dir).await {
        // Save error status to DB
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status, work_dir, model_config) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&body.app_type)
        .bind(&body.model)
        .bind(&proxy_url)
        .bind(&now)
        .bind(&config_path)
        .bind(&install_path)
        .bind("launch_error")
        .bind(&body.work_dir)
        .bind(&model_config_json)
        .execute(pool)
        .await;

        return Err(err_json(format!("Failed to launch: {}", e)));
    }

    // Success — save to DB
    sqlx::query(
        "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status, work_dir, model_config) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&body.app_type)
    .bind(&body.model)
    .bind(&proxy_url)
    .bind(&now)
    .bind(&config_path)
    .bind(&install_path)
    .bind("success")
    .bind(&body.work_dir)
    .bind(&model_config_json)
    .execute(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    if app_type.is_codex() {
        sync_codex_route_rule(&body.model).await;
    }

    let app_config = AppConfig {
        app_type,
        installed: true,
        install_path: Some(install_path),
        config_path: Some(config_path),
        model: Some(body.model),
        model_haiku: body.model_haiku,
        model_sonnet: body.model_sonnet,
        model_opus: body.model_opus,
        work_dir: body.work_dir,
        proxy_url: Some(proxy_url),
        launched_at: Some(now),
        status: Some("success".to_string()),
    };

    Ok(ok(app_config))
}

pub async fn set_app_path(
    Path(app_type_str): Path<String>,
    Json(body): Json<SetPathRequest>,
) -> Result<Json<ApiResponse<()>>, Json<ApiError>> {
    let _app_type = AppType::from_str(&app_type_str)
        .ok_or_else(|| err_json(format!("Unknown app type: {}", app_type_str)))?;

    let pool = get_pool().await;

    let exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM app_configs WHERE app_type = ?",
    )
    .bind(&app_type_str)
    .fetch_one(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    if exists {
        sqlx::query("UPDATE app_configs SET install_path = ? WHERE app_type = ?")
            .bind(&body.install_path)
            .bind(&app_type_str)
            .execute(pool)
            .await
            .map_err(|e| err_json(e.to_string()))?;
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&app_type_str)
        .bind("")
        .bind("")
        .bind(&now)
        .bind(Option::<String>::None)
        .bind(&body.install_path)
        .bind("pending")
        .execute(pool)
        .await
        .map_err(|e| err_json(e.to_string()))?;
    }

    Ok(ok(()))
}

async fn resolve_api_key_for_claude(app_type: &AppType) -> Result<String, String> {
    let _ = app_type; // Both Claude CLI and Desktop use the anthropic provider
    let provider_id = "anthropic";

    let selected_key = KeyRotation::get_next_key(provider_id, &RotationStrategy::LeastUsed)
        .await
        .map_err(|e| format!("Failed to get API key: {}", e))?;

    let nonce_array: [u8; 12] = selected_key
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid nonce length".to_string())?;

    decrypt_api_key(&selected_key.encrypted_key, &nonce_array)
        .map_err(|e| format!("Failed to decrypt API key: {}", e))
}

async fn sync_codex_route_rule(model: &str) {
    let pool = get_pool().await;

    if model.is_empty() {
        let _ = sqlx::query("DELETE FROM interceptor_rules WHERE id = 'auto_codex_model_route'")
            .execute(pool)
            .await;
        return;
    }

    let condition_json = r#"{"type":"model_matches","pattern":"gpt*"}"#;
    let action_json = format!(r#"{{"type":"replace_model","model":"{}"}}"#, model);

    let _ = sqlx::query(
        "INSERT OR REPLACE INTO interceptor_rules (id, name, phase, rule_type, condition_json, action_json, priority, enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("auto_codex_model_route")
    .bind("Codex 模型自动路由")
    .bind("pre")
    .bind("model_route")
    .bind(condition_json)
    .bind(&action_json)
    .bind(100i64)
    .bind(1i64)
    .execute(pool)
    .await;

    tracing::info!("Synced codex auto-route rule: gpt* -> {}", model);
}

async fn resolve_proxy_auth_key() -> Result<String, String> {
    let pool = get_pool().await;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'proxy_auth_key'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to query proxy_auth_key: {}", e))?;

    row.and_then(|(v,)| if v.is_empty() { None } else { Some(v) })
        .ok_or_else(|| "proxy_auth_key not configured".to_string())
}
