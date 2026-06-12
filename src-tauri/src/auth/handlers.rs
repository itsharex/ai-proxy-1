use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::middleware::AuthClaims;
use crate::auth::types::{AuthUser, Claims, LoginRequest, LoginResponse};
use crate::db::get_pool;
use crate::server::api::{err_json, ok, ApiError, ApiResponse};

const JWT_SECRET_ENV: &str = "AI_PROXY_JWT_SECRET";
const TOKEN_EXPIRATION_SECONDS: i64 = 24 * 3600;

fn get_jwt_secret() -> String {
    std::env::var(JWT_SECRET_ENV)
        .unwrap_or_else(|_| "ai-proxy-default-jwt-secret-change-in-production".to_string())
}

fn generate_token(
    user_id: &str,
    username: &str,
    role: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret();
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + TOKEN_EXPIRATION_SECONDS as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        exp: expiration,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub async fn login(
    Json(body): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, Json<ApiError>> {
    let pool = get_pool().await;

    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT id, password_hash, role FROM users WHERE username = ?")
            .bind(&body.username)
            .fetch_optional(pool)
            .await
            .map_err(|e| err_json(format!("Database error: {}", e)))?;

    let (user_id, password_hash, role) = match row {
        Some(r) => r,
        None => return Err(err_json("Invalid username or password")),
    };

    let valid = bcrypt::verify(&body.password, &password_hash)
        .map_err(|e| err_json(format!("Password verification error: {}", e)))?;

    if !valid {
        return Err(err_json("Invalid username or password"));
    }

    let token = generate_token(&user_id, &body.username, &role)
        .map_err(|e| err_json(format!("Token generation error: {}", e)))?;

    Ok(ok(LoginResponse {
        token,
        expires_in: TOKEN_EXPIRATION_SECONDS,
        user: AuthUser {
            id: user_id,
            username: body.username,
            role,
        },
    }))
}

pub async fn me(claims: AuthClaims) -> Result<Json<ApiResponse<AuthUser>>, Json<ApiError>> {
    Ok(ok(AuthUser {
        id: claims.sub,
        username: claims.username,
        role: claims.role,
    }))
}

pub async fn logout() -> Json<ApiResponse<serde_json::Value>> {
    ok(json!({ "success": true }))
}

pub fn auth_routes() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}
