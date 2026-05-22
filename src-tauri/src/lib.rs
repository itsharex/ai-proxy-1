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
mod ipc;

use tauri::Manager;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static APP_RUNTIME: Lazy<Mutex<Option<tokio::runtime::Runtime>>> = Lazy::new(|| Mutex::new(None));

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

            let _handle = app.handle().clone();
            rt.spawn(async move {
                let pool = db::get_pool().await;

                let host: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_host'")
                    .fetch_one(pool)
                    .await
                    .unwrap_or_else(|_| "127.0.0.1".to_string());

                let port_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_port'")
                    .fetch_one(pool)
                    .await
                    .unwrap_or_else(|_| "7860".to_string());

                let port: u16 = port_str.parse().unwrap_or(7860);

                server::start_server(&host, port).await;
            });

            {
                let mut guard = APP_RUNTIME.lock().unwrap();
                *guard = Some(rt);
            }

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
