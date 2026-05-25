use crate::apps::types::AppType;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn codex_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    home.join(".codex").join("config.toml")
}

pub fn claude_cli_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    home.join(".claude").join("settings.json")
}

pub fn claude_desktop_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join("Claude")
            .join("claude_desktop_config.json")
    }
    #[cfg(target_os = "windows")]
    {
        home.join("AppData")
            .join("Roaming")
            .join("Claude")
            .join("claude_desktop_config.json")
    }
    #[cfg(target_os = "linux")]
    {
        home.join(".config")
            .join("Claude")
            .join("claude_desktop_config.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        home.join(".config")
            .join("Claude")
            .join("claude_desktop_config.json")
    }
}

pub fn config_path_for(app_type: &AppType) -> PathBuf {
    match app_type {
        AppType::CodexCli | AppType::CodexDesktop => codex_config_path(),
        AppType::ClaudeCli => claude_cli_config_path(),
        AppType::ClaudeDesktop => claude_desktop_config_path(),
    }
}

pub async fn write_codex_config(model: &str, proxy_base: &str) -> Result<PathBuf, String> {
    let path = codex_config_path();
    let mut config: HashMap<String, toml::Value> = if path.exists() {
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read codex config: {}", e))?;
        toml::from_str(&content).unwrap_or_else(|_| HashMap::new())
    } else {
        HashMap::new()
    };

    config.insert(
        "model".to_string(),
        toml::Value::String(model.to_string()),
    );
    config.insert(
        "openai_base_url".to_string(),
        toml::Value::String(proxy_base.to_string()),
    );

    let content =
        toml::to_string_pretty(&config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    atomic_write(&path, &content).await?;
    tracing::info!("Wrote codex config to {:?}", path);
    Ok(path)
}

pub async fn write_claude_cli_config(model: &str, proxy_base: &str) -> Result<PathBuf, String> {
    let path = claude_cli_config_path();
    let mut config: serde_json::Value = if path.exists() {
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read claude cli config: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::Value::Object(Default::default())
    };

    let env = config
        .as_object_mut()
        .unwrap()
        .entry("env")
        .or_insert_with(|| serde_json::Value::Object(Default::default()));

    if let Some(env_obj) = env.as_object_mut() {
        env_obj.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            serde_json::Value::String(proxy_base.to_string()),
        );
        env_obj.insert(
            "ANTHROPIC_MODEL".to_string(),
            serde_json::Value::String(model.to_string()),
        );
    }

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    atomic_write(&path, &content).await?;
    tracing::info!("Wrote claude cli config to {:?}", path);
    Ok(path)
}

pub async fn write_claude_desktop_config(model: &str, proxy_base: &str) -> Result<PathBuf, String> {
    let path = claude_desktop_config_path();
    let mut config: serde_json::Value = if path.exists() {
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read claude desktop config: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::Value::Object(Default::default())
    };

    let env = config
        .as_object_mut()
        .unwrap()
        .entry("env")
        .or_insert_with(|| serde_json::Value::Object(Default::default()));

    if let Some(env_obj) = env.as_object_mut() {
        env_obj.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            serde_json::Value::String(proxy_base.to_string()),
        );
        env_obj.insert(
            "ANTHROPIC_MODEL".to_string(),
            serde_json::Value::String(model.to_string()),
        );
    }

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    atomic_write(&path, &content).await?;
    tracing::info!("Wrote claude desktop config to {:?}", path);
    Ok(path)
}

pub async fn write_config(
    app_type: &AppType,
    model: &str,
    proxy_base: &str,
) -> Result<PathBuf, String> {
    match app_type {
        AppType::CodexCli | AppType::CodexDesktop => {
            write_codex_config(model, proxy_base).await
        }
        AppType::ClaudeCli => write_claude_cli_config(model, proxy_base).await,
        AppType::ClaudeDesktop => {
            write_claude_desktop_config(model, proxy_base).await
        }
    }
}

pub(crate) async fn atomic_write(path: &PathBuf, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    tokio::fs::write(&tmp_path, content)
        .await
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    tokio::fs::rename(&tmp_path, path)
        .await
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_toml_config_serialization() {
        let mut config: HashMap<String, toml::Value> = HashMap::new();
        config.insert(
            "model".to_string(),
            toml::Value::String("gpt-4".to_string()),
        );
        config.insert(
            "openai_base_url".to_string(),
            toml::Value::String("http://127.0.0.1:7860/v1".to_string()),
        );

        let output = toml::to_string_pretty(&config).expect("Failed to serialize");

        assert!(output.contains("model = \"gpt-4\""));
        assert!(output.contains("openai_base_url = \"http://127.0.0.1:7860/v1\""));
    }

    #[test]
    fn test_toml_config_preserves_existing() {
        let initial = r#"approval_policy = "on-request""#;
        let mut config: HashMap<String, toml::Value> =
            toml::from_str(initial).expect("Failed to parse initial TOML");

        config.insert(
            "model".to_string(),
            toml::Value::String("gpt-4".to_string()),
        );
        config.insert(
            "openai_base_url".to_string(),
            toml::Value::String("http://127.0.0.1:7860/v1".to_string()),
        );

        let output = toml::to_string_pretty(&config).expect("Failed to serialize");

        assert!(
            output.contains("approval_policy = \"on-request\""),
            "original field should be preserved"
        );
        assert!(output.contains("model = \"gpt-4\""));
        assert!(output.contains("openai_base_url = \"http://127.0.0.1:7860/v1\""));
    }

    #[test]
    fn test_json_config_serialization() {
        let initial = r#"{"language":"Chinese","env":{"ANTHROPIC_API_KEY":"sk-xxx"}}"#;
        let mut config: serde_json::Value =
            serde_json::from_str(initial).expect("Failed to parse initial JSON");

        let env = config
            .as_object_mut()
            .unwrap()
            .entry("env")
            .or_insert_with(|| serde_json::Value::Object(Default::default()));

        if let Some(env_obj) = env.as_object_mut() {
            env_obj.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                serde_json::Value::String("http://127.0.0.1:7860".to_string()),
            );
            env_obj.insert(
                "ANTHROPIC_MODEL".to_string(),
                serde_json::Value::String("claude-sonnet-4-20250514".to_string()),
            );
        }

        let output = serde_json::to_string_pretty(&config).expect("Failed to serialize");

        assert!(output.contains("\"language\": \"Chinese\""));
        assert!(output.contains("\"ANTHROPIC_API_KEY\": \"sk-xxx\""));
        assert!(output.contains("\"ANTHROPIC_BASE_URL\": \"http://127.0.0.1:7860\""));
        assert!(output.contains("\"ANTHROPIC_MODEL\": \"claude-sonnet-4-20250514\""));
    }

    #[test]
    fn test_json_config_creates_env_if_missing() {
        let initial = r#"{"language":"Chinese"}"#;
        let mut config: serde_json::Value =
            serde_json::from_str(initial).expect("Failed to parse initial JSON");

        let env = config
            .as_object_mut()
            .unwrap()
            .entry("env")
            .or_insert_with(|| serde_json::Value::Object(Default::default()));

        if let Some(env_obj) = env.as_object_mut() {
            env_obj.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                serde_json::Value::String("http://127.0.0.1:7860".to_string()),
            );
            env_obj.insert(
                "ANTHROPIC_MODEL".to_string(),
                serde_json::Value::String("claude-sonnet-4-20250514".to_string()),
            );
        }

        let output = serde_json::to_string_pretty(&config).expect("Failed to serialize");

        assert!(output.contains("\"language\": \"Chinese\""));
        assert!(output.contains("\"ANTHROPIC_BASE_URL\": \"http://127.0.0.1:7860\""));
        assert!(output.contains("\"ANTHROPIC_MODEL\": \"claude-sonnet-4-20250514\""));
    }

    #[tokio::test]
    async fn test_atomic_write() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let file_path = PathBuf::from(dir.path()).join("test_config.toml");
        let content = "model = \"gpt-4\"\nopenai_base_url = \"http://127.0.0.1:7860/v1\"\n";

        atomic_write(&file_path, content)
            .await
            .expect("atomic_write failed");

        let read_back = tokio::fs::read_to_string(&file_path)
            .await
            .expect("Failed to read back file");

        assert_eq!(read_back, content);
    }
}
