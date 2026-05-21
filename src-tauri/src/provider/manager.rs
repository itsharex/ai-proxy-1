use super::endpoint::{ApiKeyInfo, Provider, ProviderEndpoint};
use crate::error::ProxyError;
use sqlx::SqlitePool;

pub struct ProviderManager;

impl ProviderManager {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Provider>, ProxyError> {
        let providers = sqlx::query_as::<_, DbProvider>(
            "SELECT id, name, base_url, auth_type, auth_header FROM providers",
        )
        .fetch_all(pool)
        .await?;

        let mut result = Vec::new();
        for p in providers {
            let endpoints = sqlx::query_as::<_, DbEndpoint>(
                "SELECT id, provider_id, format, path FROM endpoints WHERE provider_id = ?",
            )
            .bind(&p.id)
            .fetch_all(pool)
            .await?;

            let keys = sqlx::query_as::<_, DbApiKeyInfo>(
                "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ?",
            )
            .bind(&p.id)
            .fetch_all(pool)
            .await?;

            result.push(Provider {
                id: p.id,
                name: p.name,
                base_url: p.base_url,
                auth_type: p.auth_type,
                auth_header: p.auth_header,
                endpoints: endpoints
                    .into_iter()
                    .map(|e| ProviderEndpoint {
                        id: e.id,
                        provider_id: e.provider_id,
                        format: e.format,
                        path: e.path,
                    })
                    .collect(),
                api_keys: keys
                    .into_iter()
                    .map(|k| ApiKeyInfo {
                        id: k.id,
                        label: k.label,
                        is_active: k.is_active != 0,
                        usage_count: k.usage_count,
                        last_used_at: k.last_used_at,
                        created_at: k.created_at,
                    })
                    .collect(),
            });
        }

        Ok(result)
    }

    pub async fn create(
        pool: &SqlitePool,
        id: &str,
        name: &str,
        base_url: &str,
        auth_type: &str,
        auth_header: &str,
    ) -> Result<(), ProxyError> {
        sqlx::query(
            "INSERT INTO providers (id, name, base_url, auth_type, auth_header) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(base_url)
        .bind(auth_type)
        .bind(auth_header)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), ProxyError> {
        sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn find_for_model(
        pool: &SqlitePool,
        model: &str,
    ) -> Result<Option<ResolvedRoute>, ProxyError> {
        let route = sqlx::query_as::<_, DbModelRoute>(
            "SELECT id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority FROM model_routes WHERE ? LIKE REPLACE(model_pattern, '*', '%') ORDER BY priority DESC LIMIT 1",
        )
        .bind(model)
        .fetch_optional(pool)
        .await?;

        match route {
            Some(r) => {
                let provider = sqlx::query_as::<_, DbProvider>(
                    "SELECT id, name, base_url, auth_type, auth_header FROM providers WHERE id = ?",
                )
                .bind(&r.provider_id)
                .fetch_one(pool)
                .await?;

                let keys = sqlx::query_as::<_, DbApiKeyInfo>(
                    "SELECT id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ? AND is_active = 1 ORDER BY usage_count ASC",
                )
                .bind(&r.provider_id)
                .fetch_all(pool)
                .await?;

                if keys.is_empty() {
                    return Err(ProxyError::Config(format!(
                        "No active API key for provider {}",
                        provider.name
                    )));
                }

                let selected_key = &keys[0];

                Ok(Some(ResolvedRoute {
                    provider_name: provider.name,
                    base_url: provider.base_url,
                    auth_type: provider.auth_type,
                    auth_header: provider.auth_header,
                    api_key_id: selected_key.id.clone(),
                    encrypted_key: None,
                    target_model: r.target_model,
                    target_format: r.target_format,
                    fallback_provider_id: r.fallback_provider_id,
                }))
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug)]
pub struct ResolvedRoute {
    pub provider_name: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: String,
    pub api_key_id: String,
    pub encrypted_key: Option<Vec<u8>>,
    pub target_model: String,
    pub target_format: String,
    pub fallback_provider_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DbProvider {
    id: String,
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: String,
}

#[derive(sqlx::FromRow)]
struct DbEndpoint {
    id: String,
    provider_id: String,
    format: String,
    path: String,
}

#[derive(sqlx::FromRow)]
struct DbApiKeyInfo {
    id: String,
    label: String,
    is_active: i64,
    usage_count: i64,
    last_used_at: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct DbModelRoute {
    id: String,
    model_pattern: String,
    alias: Option<String>,
    provider_id: String,
    target_model: String,
    target_format: String,
    fallback_provider_id: Option<String>,
    priority: i64,
}
