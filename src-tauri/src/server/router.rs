use axum::Router;
use super::handlers;

pub fn build_router() -> Router {
    Router::new()
        .route(
            "/v1/chat/completions",
            axum::routing::post(handlers::completions::handle_completions),
        )
        .route(
            "/v1/responses",
            axum::routing::post(handlers::responses::handle_responses),
        )
        .route(
            "/v1/messages",
            axum::routing::post(handlers::anthropic::handle_anthropic),
        )
        .route(
            "/v1beta/models/{model}",
            axum::routing::post(handlers::gemini::handle_gemini),
        )
        .route(
            "/v1beta/models/{model}/{action}",
            axum::routing::post(handlers::gemini::handle_gemini_action),
        )
        .route("/health", axum::routing::get(health_check))
}

async fn health_check() -> &'static str {
    "ok"
}
