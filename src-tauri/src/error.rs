use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Key management error: {0}")]
    KeyManagement(String),

    #[error("Routing error: {0}")]
    Routing(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProxyError::Parse(m) => (StatusCode::BAD_REQUEST, m.clone()),
            ProxyError::ModelNotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            ProxyError::Provider(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            ProxyError::Network(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            ProxyError::Config(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            ProxyError::KeyManagement(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            ProxyError::Routing(m) => (StatusCode::NOT_FOUND, m.clone()),
            ProxyError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = json!({ "error": { "message": message, "type": "proxy_error" } });
        (status, axum::Json(body)).into_response()
    }
}
