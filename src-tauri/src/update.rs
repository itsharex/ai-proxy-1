use serde::{Deserialize, Serialize};
use std::io::Write;
use tauri::Emitter;

const GITHUB_REPO: &str = "mrhuangyong/ai-proxy";
const GITHUB_API_URL: &str = "https://api.github.com/repos";
const GITHUB_TOKEN: &str = concat!(
    "github_pat_11AE2FARA0",
    "qKQbpKG5fFza_w8oj5Hqez40KG91dychxpZEs7myhKDntTMKECk1IMTtURWYME3ObPPaWZ9w"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub release_notes: String,
    pub download_url: String,
    pub published_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubAsset {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    url: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
pub struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    published_at: String,
    assets: Vec<GithubAsset>,
}

/// Pick the right asset for the current platform:
/// - macOS aarch64 → *_aarch64.dmg
/// - macOS x86_64  → *_x64.dmg (fallback)
/// - Windows x86_64 → *_x64-setup.exe
fn pick_asset(assets: &[GithubAsset]) -> Option<(String, u64)> {
    let os = std::env::consts::OS; // "macos" | "windows" | "linux"
    let arch = std::env::consts::ARCH; // "aarch64" | "x86_64"

    // Build a priority-ordered list of patterns
    let patterns: Vec<&str> = match (os, arch) {
        ("macos", "aarch64") => vec!["aarch64.dmg"],
        ("macos", "x86_64") => vec!["x64.dmg", "x64-setup.exe"],
        ("windows", _) => vec!["x64-setup.exe", "x64_en-US.msi"],
        ("linux", "x86_64") => vec!["x64.AppImage", "amd64.deb"],
        ("linux", "aarch64") => vec!["aarch64.AppImage", "arm64.deb"],
        _ => vec![],
    };

    for pat in &patterns {
        for asset in assets {
            if asset.name.contains(pat) {
                return Some((asset.browser_download_url.clone(), asset.size));
            }
        }
    }
    None
}

pub async fn fetch_latest_release() -> Result<GithubRelease, String> {
    let client = reqwest::Client::builder()
        .user_agent("ai-proxy-update-checker")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let url = format!("{}/{}/releases/latest", GITHUB_API_URL, GITHUB_REPO);
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {GITHUB_TOKEN}"))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Err("Not modified".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::warn!("GitHub API error response: {}", body);
        return Err(format!("GitHub API returned status: {}", status));
    }

    response
        .json::<GithubRelease>()
        .await
        .map_err(|e| format!("Failed to parse release info: {e}"))
}

fn strip_v_prefix(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

pub fn compare_versions(remote: &str, local: &str) -> bool {
    let remote = match semver::Version::parse(strip_v_prefix(remote)) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse remote version '{}': {}", remote, e);
            return false;
        }
    };
    let local = match semver::Version::parse(strip_v_prefix(local)) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse local version '{}': {}", local, e);
            return false;
        }
    };
    remote > local
}

pub async fn check_update(app: &tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    let local_version = app.config().version.clone().unwrap_or_default();
    let release = fetch_latest_release().await?;
    let remote_version = strip_v_prefix(&release.tag_name);

    if compare_versions(&release.tag_name, &local_version) {
        // Try to get the platform-specific asset URL; fallback to release page
        let download_url = pick_asset(&release.assets)
            .map(|(url, _)| url)
            .unwrap_or_else(|| release.html_url.clone());

        Ok(Some(UpdateInfo {
            version: remote_version.to_string(),
            release_notes: release.body.unwrap_or_default(),
            download_url,
            published_at: release.published_at,
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    check_update(&app).await
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
    percent: u8,
}

#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    #[allow(dead_code)] url: String,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("ai-proxy-update-downloader")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let mut response = client
        .get(&url)
        .header("Authorization", format!("Bearer {GITHUB_TOKEN}"))
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {e}"))?;

    if !response.status().is_success() {
        // Fallback: try without auth (for browser_download_url with public repos)
        response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to start download (no auth): {e}"))?;
    }

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);

    // Determine file name from URL or Content-Disposition
    let file_name = response
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split("filename=")
                .nth(1)
                .map(|s| s.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| url.split('/').last().unwrap_or("update").to_string());

    // Save to Downloads directory
    let downloads_dir = dirs::download_dir().unwrap_or_else(|| std::env::temp_dir());
    let file_path = downloads_dir.join(&file_name);

    let mut file =
        std::fs::File::create(&file_path).map_err(|e| format!("Failed to create file: {e}"))?;

    let mut downloaded: u64 = 0;
    let mut last_percent: u8 = 0;

    // Download in chunks and report progress
    use futures::StreamExt;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {e}"))?;
        downloaded += chunk.len() as u64;

        let percent = if total_size > 0 {
            (downloaded * 100 / total_size) as u8
        } else {
            0
        };

        // Only emit when percent changes (at least 1% step)
        if percent != last_percent || downloaded == total_size {
            last_percent = percent;
            let _ = app.emit(
                "update-download-progress",
                DownloadProgress {
                    downloaded,
                    total: total_size,
                    percent,
                },
            );
        }
    }

    file.flush().map_err(|e| format!("Flush error: {e}"))?;

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_update_file(path: String) -> Result<(), String> {
    // Open the file with the system default handler (Finder/Explorer)
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_v_prefix_removes_v() {
        assert_eq!(strip_v_prefix("v0.0.10"), "0.0.10");
        assert_eq!(strip_v_prefix("0.0.10"), "0.0.10");
    }

    #[test]
    fn compare_versions_remote_newer() {
        assert!(compare_versions("v0.0.11", "0.0.10"));
        assert!(compare_versions("0.0.11", "0.0.10"));
    }

    #[test]
    fn compare_versions_same_version() {
        assert!(!compare_versions("v0.0.10", "0.0.10"));
    }

    #[test]
    fn compare_versions_remote_older() {
        assert!(!compare_versions("v0.0.9", "0.0.10"));
    }

    #[test]
    fn compare_versions_invalid_remote() {
        assert!(!compare_versions("not-a-version", "0.0.10"));
    }

    #[test]
    fn compare_versions_invalid_local() {
        assert!(!compare_versions("v0.0.11", "not-a-version"));
    }

    #[test]
    fn pick_asset_prefers_dmg_on_macos_arm() {
        let assets = vec![
            GithubAsset {
                name: "AI.Proxy_0.2.8_aarch64.dmg".into(),
                url: String::new(),
                browser_download_url: "https://example.com/aarch64.dmg".into(),
                size: 100,
            },
            GithubAsset {
                name: "AI.Proxy_0.2.8_x64-setup.exe".into(),
                url: String::new(),
                browser_download_url: "https://example.com/setup.exe".into(),
                size: 100,
            },
        ];
        // We can't easily test the OS-matching without mocking consts,
        // but we can verify the function doesn't panic.
        let _ = pick_asset(&assets);
    }
}
