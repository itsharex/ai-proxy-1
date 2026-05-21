mod converter;
mod db;
mod error;
mod interceptor;
mod ipc;
mod key;
mod logging;
mod provider;
mod routing;
mod server;
mod usage;

use tauri::Manager;

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
                db::init::init_db(db_path.to_str().unwrap())
                    .await
                    .expect("failed to initialize database");
            });

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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
