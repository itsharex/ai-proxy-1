use crate::server::handlers;
use axum::Router;
use axum::routing::post;
use axum::routing::get;

pub fn create_router() -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handlers::handle_completions))
        .route("/v1/responses", post(handlers::handle_responses))
        .route("/v1/messages", post(handlers::handle_anthropic))
        .route("/v1beta/models/{model}", post(handlers::handle_gemini))
        .route("/health", get(health_check))
}

async fn health_check() -> &'static str {
    "OK"
}
