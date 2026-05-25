use crate::apps::types::AppType;
use std::path::PathBuf;
use tokio::process::Command;

pub async fn detect_path(app_type: &AppType) -> Option<String> {
    match app_type {
        AppType::CodexCli => detect_cli("codex").await,
        AppType::ClaudeCli => detect_cli("claude").await,
        AppType::CodexDesktop => detect_desktop_app("Codex").await,
        AppType::ClaudeDesktop => detect_desktop_app("Claude").await,
    }
}

async fn detect_cli(name: &str) -> Option<String> {
    let cmd_name = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(cmd_name).arg(name).output().await.ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            tracing::info!("Detected {} at {}", name, path);
            return Some(path);
        }
    }
    None
}

async fn detect_desktop_app(name: &str) -> Option<String> {
    let candidates = desktop_candidates(name);
    for candidate in candidates {
        if candidate.exists() {
            let path = candidate.to_string_lossy().to_string();
            tracing::info!("Detected {} at {}", name, path);
            return Some(path);
        }
    }
    None
}

fn desktop_candidates(name: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            PathBuf::from(format!("/Applications/{}.app", name)),
            dirs::home_dir()
                .map(|h| h.join("Applications").join(format!("{}.app", name)))
                .unwrap_or_default(),
        ]
        .into_iter()
        .filter(|p| !p.as_os_str().is_empty())
        .collect()
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        vec![
            PathBuf::from(format!("C:\\Program Files\\{}\\{}.exe", name, name)),
            PathBuf::from(format!(
                "C:\\Program Files (x86)\\{}\\{}.exe",
                name, name
            )),
            PathBuf::from(format!(
                "{}\\Programs\\{}\\{}.exe",
                local_app_data, name, name
            )),
        ]
    }

    #[cfg(target_os = "linux")]
    {
        let name_lower = name.to_lowercase();
        vec![
            PathBuf::from(format!("/usr/share/applications/{}.desktop", name_lower)),
            dirs::home_dir()
                .map(|h| {
                    h.join(".local")
                        .join("share")
                        .join("applications")
                        .join(format!("{}.desktop", name_lower))
                })
                .unwrap_or_default(),
        ]
        .into_iter()
        .filter(|p| !p.as_os_str().is_empty())
        .collect()
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = name;
        vec![]
    }
}

pub fn resolve_install_path(app_type: &AppType, custom_path: Option<&str>) -> Option<String> {
    let _ = app_type;
    custom_path
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.to_string())
}

pub async fn launch(app_type: &AppType, install_path: &str) -> Result<(), String> {
    if app_type.is_cli() {
        launch_cli(install_path).await
    } else {
        launch_desktop(install_path).await
    }
}

async fn launch_cli(install_path: &str) -> Result<(), String> {
    tracing::info!("Launching CLI app at {}", install_path);
    let child = Command::new(install_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
    tracing::info!("Launched CLI process, pid: {:?}", child.id());
    Ok(())
}

async fn launch_desktop(install_path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        tracing::info!("Launching macOS desktop app at {}", install_path);
        Command::new("open")
            .arg(install_path)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        tracing::info!("Launching Linux desktop app at {}", install_path);
        Command::new("xdg-open")
            .arg(install_path)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        tracing::info!("Launching Windows desktop app at {}", install_path);
        Command::new("cmd")
            .args(["/C", "start", "", install_path])
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = install_path;
        Err("Unsupported platform for desktop app launch".to_string())
    }
}
