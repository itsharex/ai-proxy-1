use axum::extract::{Path, Json};
use std::collections::HashMap;

use crate::apps::config;
use crate::apps::launcher;
use crate::apps::types::{AppConfig, AppType, DbAppConfig, LaunchRequest, SetPathRequest};
use crate::db::get_pool;
use crate::key::rotation::{KeyRotation, RotationStrategy};
use crate::key::store::decrypt_api_key;
use crate::server::api::{ok, err_json, ApiError, ApiResponse};

pub async fn list_apps() -> Json<ApiResponse<Vec<AppConfig>>> {
    let pool = get_pool().await;

    let rows: Vec<DbAppConfig> = sqlx::query_as(
        "SELECT app_type, model, proxy_url, launched_at, config_path, install_path, status FROM app_configs",
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
            model_haiku: None,
            model_sonnet: None,
            model_opus: None,
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
        "SELECT app_type, model, proxy_url, launched_at, config_path, install_path, status FROM app_configs WHERE app_type = ?",
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
        "SELECT key, value FROM settings WHERE key IN ('http_host', 'http_port')",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

    let settings_map: HashMap<String, String> = proxy_settings.into_iter().collect();
    let host = settings_map.get("http_host").cloned().unwrap_or_else(|| "127.0.0.1".into());
    let host = if host == "0.0.0.0" { "127.0.0.1".to_string() } else { host };
    let port = settings_map.get("http_port").cloned().unwrap_or_else(|| "7860".into());
    let proxy_base = format!("http://{}:{}", host, port);

    let proxy_url = format!("{}{}", proxy_base, app_type.proxy_url_suffix());
    let now = chrono::Utc::now().to_rfc3339();

    // Write config file
    let model_haiku = body.model_haiku.as_deref();
    let model_sonnet = body.model_sonnet.as_deref();
    let model_opus = body.model_opus.as_deref();

    // Resolve an API key for Claude Desktop gateway config
    let api_key = if app_type.is_claude() {
        resolve_api_key_for_claude(&app_type).await.unwrap_or_default()
    } else {
        String::new()
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
            "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&body.app_type)
        .bind(&body.model)
        .bind(&proxy_url)
        .bind(&now)
        .bind(Option::<String>::None)
        .bind(&install_path)
        .bind("config_error")
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
            "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&body.app_type)
        .bind(&body.model)
        .bind(&proxy_url)
        .bind(&now)
        .bind(&config_path)
        .bind(&install_path)
        .bind("launch_error")
        .execute(pool)
        .await;

        return Err(err_json(format!("Failed to launch: {}", e)));
    }

    // Success — save to DB
    sqlx::query(
        "INSERT OR REPLACE INTO app_configs (app_type, model, proxy_url, launched_at, config_path, install_path, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&body.app_type)
    .bind(&body.model)
    .bind(&proxy_url)
    .bind(&now)
    .bind(&config_path)
    .bind(&install_path)
    .bind("success")
    .execute(pool)
    .await
    .map_err(|e| err_json(e.to_string()))?;

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
