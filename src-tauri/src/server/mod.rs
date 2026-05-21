pub mod router;
pub mod handlers;
pub mod middleware;

use axum::Router;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

pub fn create_server(host: &str, _port: u16) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            if host == "127.0.0.1" || host == "localhost" {
                "http://localhost:1420"
                    .parse::<HeaderValue>()
                    .unwrap()
            } else {
                "*".parse::<HeaderValue>().unwrap()
            },
        )
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    router::build_router().layer(cors)
}

pub async fn start_server(
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_server(&host, port);
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Proxy server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
