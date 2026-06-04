use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::future::Future;
use std::pin::Pin;

use crate::auth::types::Claims;

const JWT_SECRET_ENV: &str = "AI_PROXY_JWT_SECRET";

#[derive(Debug, Clone)]
pub struct AuthClaims {
    pub sub: String,
    pub username: String,
    pub role: String,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthClaims {
    type Rejection = Response;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let secret = std::env::var(JWT_SECRET_ENV)
                .unwrap_or_else(|_| "ai-proxy-default-jwt-secret-change-in-production".to_string());

            let token = parts
                .headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .unwrap_or("");

            if token.is_empty() {
                return Err((StatusCode::UNAUTHORIZED, "Missing token").into_response());
            }

            let decoded = jsonwebtoken::decode::<Claims>(
                token,
                &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
                &jsonwebtoken::Validation::default(),
            );

            match decoded {
                Ok(token_data) => {
                    let claims = token_data.claims;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as usize;
                    if claims.exp < now {
                        return Err((StatusCode::UNAUTHORIZED, "Token expired").into_response());
                    }
                    Ok(AuthClaims {
                        sub: claims.sub,
                        username: claims.username,
                        role: claims.role,
                    })
                }
                Err(_) => Err((StatusCode::UNAUTHORIZED, "Invalid token").into_response()),
            }
        })
    }
}

pub async fn jwt_auth_middleware(req: Request<Body>, next: Next) -> Response {
    let (mut parts, body) = req.into_parts();

    match AuthClaims::from_request_parts(&mut parts, &()).await {
        Ok(claims) => {
            let mut req = Request::from_parts(parts, body);
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(response) => response,
    }
}
