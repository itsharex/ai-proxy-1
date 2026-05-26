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
mod apps;

use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use std::sync::Mutex;
use once_cell::sync::Lazy;

const DEFAULT_PROXY_PORT: u16 = 7860;
const DEFAULT_PROXY_HOST: &str = "127.0.0.1";

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
        port: DEFAULT_PROXY_PORT,
        host: DEFAULT_PROXY_HOST.to_string(),
        shutdown_tx: None,
    })
});

async fn get_proxy_config() -> (String, u16) {
    let pool = db::get_pool().await;
    let host: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_host'")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| DEFAULT_PROXY_HOST.to_string());
    let pool = db::get_pool().await;
    let port_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_port'")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| DEFAULT_PROXY_PORT.to_string());
    (host, port_str.parse().unwrap_or(DEFAULT_PROXY_PORT))
}

#[tauri::command]
async fn get_api_config() -> String {
    let (host, port) = get_proxy_config().await;
    format!("http://{}:{}", host, port)
}

fn start_proxy() -> (String, u16) {
    {
        let ctrl = PROXY_CONTROL.lock().unwrap();
        if ctrl.running {
            let host = ctrl.host.clone();
            let port = ctrl.port;
            return (host, port);
        }
    }

    let handle = {
        let guard = APP_RUNTIME.lock().unwrap();
        guard.as_ref().expect("runtime not initialized").handle().clone()
    };

    let (tx, rx) = tokio::sync::watch::channel(false);

    handle.spawn(async move {
        let (host, port) = get_proxy_config().await;

        {
            let mut ctrl = PROXY_CONTROL.lock().unwrap();
            ctrl.running = true;
            ctrl.host = host.clone();
            ctrl.port = port;
            ctrl.shutdown_tx = Some(tx);
        }

        server::start_server(&host, port, rx).await;

        let mut ctrl = PROXY_CONTROL.lock().unwrap();
        ctrl.running = false;
    });

    (DEFAULT_PROXY_HOST.to_string(), DEFAULT_PROXY_PORT)
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

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
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

            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

            TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .tooltip("AI Proxy")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    if event.id() == "quit" {
                        stop_proxy();
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        show_main_window(app);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_api_config])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main_window(app_handle);
            }
        });
}
