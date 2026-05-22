use serde::Deserialize;
use sqlx::FromRow;
use tracing::info;

use crate::converter::ir::ClientFormat;
use crate::db::get_pool;
use crate::error::ProxyError;
use crate::provider::endpoint::{ApiKeyInfo, Provider, ProviderEndpoint};

#[derive(Debug, Clone)]
pub struct ResolvedRoute {
    pub provider_id: String,
    pub provider_name: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: String,
    pub target_format: ClientFormat,
    pub target_model: String,
    pub endpoint_path: String,
    #[allow(dead_code)]
    pub fallback_provider_id: Option<String>,
}

#[derive(Debug, FromRow, Deserialize)]
struct DbProvider {
    id: String,
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: String,
}

#[derive(Debug, FromRow, Deserialize)]
struct DbEndpoint {
    id: String,
    provider_id: String,
    format: String,
    path: String,
}

#[derive(Debug, FromRow, Deserialize)]
#[allow(dead_code)]
struct DbApiKeyInfo {
    id: String,
    provider_id: String,
    label: String,
    is_active: i64,
    usage_count: i64,
    last_used_at: Option<String>,
    created_at: String,
}

#[derive(Debug, FromRow, Deserialize)]
#[allow(dead_code)]
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

pub struct ProviderManager;

impl ProviderManager {
    pub async fn list() -> Result<Vec<Provider>, ProxyError> {
        let pool = get_pool().await;

        let db_providers: Vec<DbProvider> =
            sqlx::query_as("SELECT id, name, base_url, auth_type, auth_header FROM providers ORDER BY name")
                .fetch_all(pool)
                .await?;

        let mut providers = Vec::new();

        for dbp in db_providers {
            let endpoints: Vec<DbEndpoint> = sqlx::query_as(
                "SELECT id, provider_id, format, path FROM endpoints WHERE provider_id = ?",
            )
            .bind(&dbp.id)
            .fetch_all(pool)
            .await?;

            let keys: Vec<DbApiKeyInfo> = sqlx::query_as(
                "SELECT id, provider_id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ?",
            )
            .bind(&dbp.id)
            .fetch_all(pool)
            .await?;

            providers.push(Provider {
                id: dbp.id,
                name: dbp.name,
                base_url: dbp.base_url,
                auth_type: dbp.auth_type,
                auth_header: dbp.auth_header,
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

        Ok(providers)
    }

    pub async fn find_for_model(model: &str) -> Result<ResolvedRoute, ProxyError> {
        let pool = get_pool().await;

        let routes: Vec<DbModelRoute> = sqlx::query_as(
            "SELECT id, model_pattern, alias, provider_id, target_model, target_format, fallback_provider_id, priority FROM model_routes ORDER BY priority DESC",
        )
        .fetch_all(pool)
        .await?;

        let matched_route = routes
            .iter()
            .find(|r| model_matches_pattern(model, &r.model_pattern));

        let route = matched_route
            .ok_or_else(|| ProxyError::ModelNotFound(format!("no route found for model: {}", model)))?;

        let db_provider: DbProvider = sqlx::query_as(
            "SELECT id, name, base_url, auth_type, auth_header FROM providers WHERE id = ?",
        )
        .bind(&route.provider_id)
        .fetch_one(pool)
        .await
        .map_err(|_| {
            ProxyError::Provider(format!("provider not found: {}", route.provider_id))
        })?;

        let target_format = parse_client_format(&route.target_format)?;

        let endpoint: Option<DbEndpoint> = sqlx::query_as(
            "SELECT id, provider_id, format, path FROM endpoints WHERE provider_id = ? AND format = ? LIMIT 1",
        )
        .bind(&db_provider.id)
        .bind(&route.target_format)
        .fetch_optional(pool)
        .await?;

        let endpoint_path = endpoint
            .map(|e| e.path)
            .unwrap_or_else(|| default_path_for_format(&target_format, &route.target_model));

        info!(
            "Resolved model '{}' -> provider '{}' format '{}' endpoint '{}'",
            model, db_provider.name, route.target_format, endpoint_path
        );

        Ok(ResolvedRoute {
            provider_id: db_provider.id,
            provider_name: db_provider.name,
            base_url: db_provider.base_url,
            auth_type: db_provider.auth_type,
            auth_header: db_provider.auth_header,
            target_format,
            target_model: route.target_model.clone(),
            endpoint_path,
            fallback_provider_id: route.fallback_provider_id.clone(),
        })
    }

    #[allow(dead_code)]
    pub async fn get_by_id(provider_id: &str) -> Result<Provider, ProxyError> {
        let pool = get_pool().await;

        let dbp: DbProvider = sqlx::query_as(
            "SELECT id, name, base_url, auth_type, auth_header FROM providers WHERE id = ?",
        )
        .bind(provider_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ProxyError::Provider(format!("provider not found: {}", provider_id)))?;

        let endpoints: Vec<DbEndpoint> = sqlx::query_as(
            "SELECT id, provider_id, format, path FROM endpoints WHERE provider_id = ?",
        )
        .bind(provider_id)
        .fetch_all(pool)
        .await?;

        let keys: Vec<DbApiKeyInfo> = sqlx::query_as(
            "SELECT id, provider_id, label, is_active, usage_count, last_used_at, created_at FROM api_keys WHERE provider_id = ?",
        )
        .bind(provider_id)
        .fetch_all(pool)
        .await?;

        Ok(Provider {
            id: dbp.id,
            name: dbp.name,
            base_url: dbp.base_url,
            auth_type: dbp.auth_type,
            auth_header: dbp.auth_header,
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
        })
    }
}

fn model_matches_pattern(model: &str, pattern: &str) -> bool {
    if pattern == "*" || pattern == model {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return model.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return model.ends_with(suffix);
    }
    false
}

fn parse_client_format(format: &str) -> Result<ClientFormat, ProxyError> {
    match format {
        "completions" => Ok(ClientFormat::Completions),
        "responses" => Ok(ClientFormat::Responses),
        "anthropic" => Ok(ClientFormat::Anthropic),
        "gemini" => Ok(ClientFormat::Gemini),
        other => Err(ProxyError::Config(format!(
            "unknown target format: {}",
            other
        ))),
    }
}

fn default_path_for_format(format: &ClientFormat, target_model: &str) -> String {
    match format {
        ClientFormat::Completions => "/v1/chat/completions".to_string(),
        ClientFormat::Responses => "/v1/responses".to_string(),
        ClientFormat::Anthropic => "/v1/messages".to_string(),
        ClientFormat::Gemini => format!("/v1beta/models/{}:generateContent", target_model),
    }
}
