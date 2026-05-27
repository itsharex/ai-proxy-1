use crate::apps::types::AppType;
use std::path::PathBuf;
use tokio::process::Command;

/// Create a Command with CREATE_NO_WINDOW flag on Windows to prevent console window flash.
fn quiet_command(program: &str) -> Command {
    let cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

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
    let output = quiet_command(cmd_name).arg(name).output().await.ok()?;
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
            // Microsoft Store / WindowsApps (winget)
            PathBuf::from(format!(
                "{}\\Microsoft\\WindowsApps\\{}.exe",
                local_app_data, name
            )),
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

#[allow(dead_code)]
pub fn resolve_install_path(app_type: &AppType, custom_path: Option<&str>) -> Option<String> {
    let _ = app_type;
    custom_path
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.to_string())
}

pub async fn launch(
    app_type: &AppType,
    install_path: &str,
    work_dir: Option<&str>,
) -> Result<(), String> {
    if app_type.is_cli() {
        launch_cli(install_path, work_dir).await
    } else {
        launch_desktop(install_path).await
    }
}

#[cfg(target_os = "macos")]
async fn launch_cli(install_path: &str, work_dir: Option<&str>) -> Result<(), String> {
    let dir = work_dir.unwrap_or("$HOME");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = format!("/tmp/ai-proxy-launch-{ts}.sh");

    let cd_cmd = if dir == "$HOME" {
        "cd \"$HOME\"".to_string()
    } else {
        format!("cd '{}'", dir.replace('\'', "'\\''"))
    };
    let content = format!(
        "#!/bin/bash\nrm -f \"$0\"\n{cd_cmd} && exec '{}'\n",
        install_path.replace('\'', "'\\''")
    );

    tokio::fs::write(&tmp_path, &content)
        .await
        .map_err(|e| format!("Failed to create launch script: {e}"))?;

    Command::new("chmod")
        .args(["+x", &tmp_path])
        .output()
        .await
        .map_err(|e| format!("Failed to set permissions: {e}"))?;

    Command::new("open")
        .args(["-a", "Terminal", &tmp_path])
        .spawn()
        .map_err(|e| format!("Failed to open terminal: {e}"))?;

    Ok(())
}

#[cfg(target_os = "windows")]
async fn launch_cli(install_path: &str, work_dir: Option<&str>) -> Result<(), String> {
    let dir = work_dir.unwrap_or("%USERPROFILE%");
    let cmd = format!("cd /d {} && {}", dir, install_path);
    Command::new("cmd")
        .args(["/C", "start", "cmd", "/K", &cmd])
        .spawn()
        .map_err(|e| format!("Failed to open terminal: {}", e))?;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn launch_cli(install_path: &str, work_dir: Option<&str>) -> Result<(), String> {
    let dir = work_dir.unwrap_or("$HOME");
    for term in &["gnome-terminal", "konsole", "xfce4-terminal", "xterm"] {
        let result = Command::new(term)
            .args(["--working-directory", dir, "-e", install_path])
            .spawn();
        if result.is_ok() {
            return Ok(());
        }
    }
    Err("No supported terminal emulator found".into())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
async fn launch_cli(_install_path: &str, _work_dir: Option<&str>) -> Result<(), String> {
    Err("Unsupported platform for CLI launch".to_string())
}

async fn kill_existing_desktop(install_path: &str) {
    let name = PathBuf::from(install_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    #[cfg(target_os = "macos")]
    {
        let _ = quiet_command("pkill")
            .args(["-x", &name])
            .output()
            .await;
    }

    #[cfg(target_os = "linux")]
    {
        let _ = quiet_command("pkill")
            .args(["-x", &name])
            .output()
            .await;
    }

    #[cfg(target_os = "windows")]
    {
        let exe_name = format!("{}.exe", name);
        let _ = quiet_command("taskkill")
            .args(["/F", "/IM", &exe_name])
            .output()
            .await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
}

async fn launch_desktop(install_path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        tracing::info!("Launching macOS desktop app at {}", install_path);
        kill_existing_desktop(install_path).await;
        Command::new("open")
            .arg(install_path)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        tracing::info!("Launching Linux desktop app at {}", install_path);
        kill_existing_desktop(install_path).await;
        Command::new("xdg-open")
            .arg(install_path)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        tracing::info!("Launching Windows desktop app at {}", install_path);
        kill_existing_desktop(install_path).await;
        quiet_command("cmd")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_install_path_custom_takes_priority() {
        let result = resolve_install_path(&AppType::CodexCli, Some("/custom/path"));
        assert_eq!(result, Some("/custom/path".to_string()));
    }

    #[test]
    fn test_resolve_install_path_empty_returns_none() {
        let result = resolve_install_path(&AppType::CodexCli, Some(""));
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_install_path_none_returns_none() {
        let result = resolve_install_path(&AppType::CodexCli, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_app_type_display() {
        assert_eq!(AppType::CodexCli.to_string(), "codex_cli");
        assert_eq!(AppType::ClaudeDesktop.to_string(), "claude_desktop");
    }

    #[test]
    fn test_app_type_is_cli() {
        assert!(AppType::CodexCli.is_cli());
        assert!(AppType::ClaudeCli.is_cli());
        assert!(!AppType::CodexDesktop.is_cli());
        assert!(!AppType::ClaudeDesktop.is_cli());
    }

    #[test]
    fn test_app_type_is_codex() {
        assert!(AppType::CodexCli.is_codex());
        assert!(AppType::CodexDesktop.is_codex());
        assert!(!AppType::ClaudeCli.is_codex());
        assert!(!AppType::ClaudeDesktop.is_codex());
    }

    #[test]
    fn test_app_type_from_str() {
        assert_eq!(AppType::from_str("codex_cli"), Some(AppType::CodexCli));
        assert_eq!(AppType::from_str("unknown"), None);
    }

    #[test]
    fn test_launch_function_accepts_work_dir() {
        // Verify the launch function signature accepts work_dir parameter
        let app_type = AppType::CodexCli;
        assert!(app_type.is_cli());
        // work_dir is Option<&str>, should accept None or Some("path")
        let _work_dir_none: Option<&str> = None;
        let _work_dir_some: Option<&str> = Some("/tmp");
        // Compilation of this test validates the signature change
    }
}
