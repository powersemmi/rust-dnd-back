use crate::config::get_jwt_secret;
use axum::{
    extract::FromRequestParts, http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid, // Subject (ID пользователя)
    pub username: String,
    pub exp: usize, // Expiration time (когда токен протухнет)
    pub iat: usize, // Issued at (когда создан)
}

/// Создание нового JWT токена
pub fn create_jwt(user_id: Uuid, username: String) -> Result<String, String> {
    let now = Utc::now();
    // Токен живет 24 часа
    let expire = now + Duration::hours(24);

    let claims = Claims {
        sub: user_id,
        username,
        exp: expire.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let secret = get_jwt_secret();

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

pub fn verify_jwt(token: &str) -> Result<Claims, String> {
    let secret = get_jwt_secret();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    Ok(token_data.claims)
}

pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Ищем заголовок Authorization: Bearer <token>
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Missing bearer token"})),
                )
                    .into_response()
            })?;

        // 2. Используем нашу новую функцию verify_jwt
        let claims = verify_jwt(bearer.token()).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            )
                .into_response()
        })?;

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
        })
    }
}
