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

struct ProxyControl {
    running: bool,
    port: u16,
    host: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

static PROXY_CONTROL: Lazy<Mutex<ProxyControl>> = Lazy::new(|| {
    Mutex::new(ProxyControl {
        running: false,
        port: 7860,
        host: "127.0.0.1".to_string(),
        shutdown_tx: None,
    })
});

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

fn start_proxy() {
    {
        let ctrl = PROXY_CONTROL.lock().unwrap();
        if ctrl.running {
            return;
        }
    }

    let handle = {
        let guard = APP_RUNTIME.lock().unwrap();
        guard.as_ref().expect("runtime not initialized").handle().clone()
    };

    let (host, port) = handle.block_on(async {
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
        (host, port_str.parse().unwrap_or(7860u16))
    });

    let (tx, rx) = tokio::sync::watch::channel(false);

    {
        let mut ctrl = PROXY_CONTROL.lock().unwrap();
        ctrl.running = true;
        ctrl.host = host.clone();
        ctrl.port = port;
        ctrl.shutdown_tx = Some(tx);
    }

    handle.spawn(async move {
        server::start_server(&host, port, rx).await;
        let mut ctrl = PROXY_CONTROL.lock().unwrap();
        ctrl.running = false;
    });
}

fn stop_proxy() {
    let mut ctrl = PROXY_CONTROL.lock().unwrap();
    if !ctrl.running {
        return;
    }
    if let Some(tx) = ctrl.shutdown_tx.take() {
        let _ = tx.send(true);
    }
    ctrl.running = false;
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

            {
                let mut guard = APP_RUNTIME.lock().unwrap();
                *guard = Some(rt);
            }

            start_proxy();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_api_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
