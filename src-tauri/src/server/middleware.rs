use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

pub fn create_cors_layer(host: &str) -> CorsLayer {
    let origin = if host == "0.0.0.0" || host == "127.0.0.1" || host == "localhost" {
        "*"
    } else {
        host
    };

    let allowed_origin = if origin == "*" {
        tower_http::cors::Any.into()
    } else {
        match HeaderValue::from_str(&format!("http://{}", origin)) {
            Ok(v) => tower_http::cors::AllowOrigin::exact(v),
            Err(_) => tower_http::cors::Any.into(),
        }
    };

    CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
}
