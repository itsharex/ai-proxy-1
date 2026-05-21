use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpoint {
    pub id: String,
    pub provider_id: String,
    pub format: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: String,
    pub endpoints: Vec<ProviderEndpoint>,
    pub api_keys: Vec<ApiKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub label: String,
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
}
