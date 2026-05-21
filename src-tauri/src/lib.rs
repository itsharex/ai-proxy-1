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
