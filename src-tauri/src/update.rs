use serde::{Deserialize, Serialize};
use tauri::Manager;

const GITHUB_REPO: &str = "mrhuangyong/ai-proxy";
const GITHUB_API_URL: &str = "https://api.github.com/repos";
const GITHUB_TOKEN: &str = concat!("github_pat_11AE2FARA0", "qKQbpKG5fFza_w8oj5Hqez40KG91dychxpZEs7myhKDntTMKECk1IMTtURWYME3ObPPaWZ9w");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub release_notes: String,
    pub download_url: String,
    pub published_at: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    published_at: String,
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
        Ok(Some(UpdateInfo {
            version: remote_version.to_string(),
            release_notes: release.body.unwrap_or_default(),
            download_url: release.html_url,
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
}
