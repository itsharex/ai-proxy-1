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
async fn get_api_config() -> String {
    let pool = db::get_pool().await;
    let host: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_host'")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let pool = db::get_pool().await;
    let port: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_port'")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "7860".to_string());
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

            rt.spawn(async move {
                let pool = db::get_pool().await;

                let host: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_host'")
                    .fetch_one(pool)
                    .await
                    .unwrap_or_else(|_| "127.0.0.1".to_string());

                let pool = db::get_pool().await;
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
        .invoke_handler(tauri::generate_handler![get_api_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
