use crate::server::handlers;
use crate::server::api;
use crate::server::middleware::auth_middleware;
use crate::mcp;
use crate::skill;
use axum::Router;
use axum::routing::{post, get};
use axum::middleware;

pub fn create_router() -> Router {
    let proxy_routes = Router::new()
        .route("/v1/chat/completions", post(handlers::handle_completions))
        .route("/v1/responses", post(handlers::handle_responses))
        .route("/v1/messages", post(handlers::handle_anthropic))
        .route("/v1/messages/count_tokens", post(handlers::handle_anthropic_count_tokens))
        .route("/v1/models", get(handlers::handle_list_models))
        .route("/v1/models/:model", get(handlers::handle_get_model))
        .route("/v1beta/models", get(handlers::handle_gemini_list_models))
        .route(
            "/v1beta/models/:model",
            get(handlers::handle_gemini_get_model).post(handlers::handle_gemini),
        )
        .layer(middleware::from_fn(auth_middleware));

    let mut router = Router::new()
        .merge(proxy_routes)
        .route("/health", get(health_check))
        .nest("/api", api::api_routes())
        .nest("/api/mcp", mcp::mcp_routes())
        .nest("/api/skills", skill::skill_routes());

    // Server mode: mount auth routes (login/logout/me)
    #[cfg(feature = "server")]
    {
        router = router.nest("/api/auth", crate::auth::handlers::auth_routes());
    }

    router
}

async fn health_check() -> &'static str {
    "OK"
}
