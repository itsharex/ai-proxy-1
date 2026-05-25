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
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
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

            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let status_text = {
                let ctrl = PROXY_CONTROL.lock().unwrap();
                if ctrl.running {
                    format!("Proxy :{} running", ctrl.port)
                } else {
                    "Proxy stopped".to_string()
                }
            };
            let status_item = MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
            let toggle_text = {
                let ctrl = PROXY_CONTROL.lock().unwrap();
                if ctrl.running { "Stop Proxy" } else { "Start Proxy" }
            };
            let toggle_item = MenuItem::with_id(app, "toggle", toggle_text, true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &status_item, &toggle_item, &separator, &quit_item])?;

            let status_for_handler = status_item.clone();
            let toggle_for_handler = toggle_item.clone();

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

            TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .tooltip("AI Proxy")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "toggle" => {
                            let is_running = {
                                let ctrl = PROXY_CONTROL.lock().unwrap();
                                ctrl.running
                            };
                            if is_running {
                                stop_proxy();
                                let _ = toggle_for_handler.set_text("Start Proxy");
                                let _ = status_for_handler.set_text("Proxy stopped");
                            } else {
                                start_proxy();
                                let port = {
                                    let ctrl = PROXY_CONTROL.lock().unwrap();
                                    ctrl.port
                                };
                                let _ = toggle_for_handler.set_text("Stop Proxy");
                                let _ = status_for_handler.set_text(&format!("Proxy :{} running", port));
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
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
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
