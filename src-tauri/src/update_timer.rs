use std::sync::Mutex;
use tauri::Emitter;
use once_cell::sync::Lazy;

use crate::update;

const INITIAL_DELAY_SECS: u64 = 30;
const CHECK_INTERVAL_SECS: u64 = 8 * 60 * 60;

static LAST_NOTIFIED_VERSION: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

fn should_notify(version: &str) -> bool {
    let mut last = LAST_NOTIFIED_VERSION.lock().unwrap();
    if last.as_deref() == Some(version) {
        return false;
    }
    *last = Some(version.to_string());
    true
}

pub fn start_update_timer(app: tauri::AppHandle) {
    let handle = {
        let guard = crate::APP_RUNTIME.lock().unwrap();
        guard.as_ref().expect("runtime not initialized").handle().clone()
    };

    handle.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;
        loop {
            match update::check_update(&app).await {
                Ok(Some(info)) => {
                    if should_notify(&info.version) {
                        let _ = app.emit("update-available", &info);
                    }
                }
                Ok(None) => {
                    let _ = app.emit("up-to-date", ());
                }
                Err(e) => {
                    tracing::warn!("Update check failed: {}", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(CHECK_INTERVAL_SECS)).await;
        }
    });
}
