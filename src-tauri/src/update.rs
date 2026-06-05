use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub release_notes: String,
    pub published_at: String,
}

pub async fn check_update(app: &tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let update = app
        .updater_builder()
        .build()
        .map_err(|e| format!("Failed to build updater: {e}"))?
        .check()
        .await
        .map_err(|e| format!("Update check failed: {e}"))?;

    match update {
        Some(update) => Ok(Some(UpdateInfo {
            version: update.version.clone(),
            release_notes: update.body.clone().unwrap_or_default(),
            published_at: update.date.map(|d| d.to_string()).unwrap_or_default(),
        })),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    check_update(&app).await
}

// Re-export tauri_plugin_updater for use in update_timer and tray handler
pub use tauri_plugin_updater;
