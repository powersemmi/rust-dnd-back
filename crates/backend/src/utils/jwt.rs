use crate::config::get_secret;
use crate::error::AppError;
use axum::{RequestPartsExt, extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub exp: u64,
    pub iat: u64,
}

pub fn create_jwt(user_id: Uuid, username: String) -> Result<String, String> {
    let now = Utc::now();
    let expire = now + Duration::hours(24);

    let claims = Claims {
        sub: user_id,
        username,
        exp: timestamp_to_u64(expire.timestamp()),
        iat: timestamp_to_u64(now.timestamp()),
    };

    let secret = get_secret("JWT_SECRET");

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|error| error.to_string())
}

pub fn verify_jwt(token: &str) -> Result<Claims, String> {
    let secret = get_secret("JWT_SECRET");
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|error| format!("Invalid token: {error}"))?;

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
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::unauthorized("Missing bearer token"))?;

        let claims =
            verify_jwt(bearer.token()).map_err(|_| AppError::unauthorized("Invalid token"))?;

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
        })
    }
}

fn timestamp_to_u64(timestamp: i64) -> u64 {
    u64::try_from(timestamp).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{create_jwt, verify_jwt};
    use std::sync::Once;
    use uuid::Uuid;

    fn init_test_env() {
        static INIT: Once = Once::new();
        INIT.call_once(|| unsafe {
            std::env::set_var("JWT_SECRET", "test-jwt-secret-1234567890");
        });
    }

    #[test]
    fn create_and_verify_token_round_trip() {
        init_test_env();

        let user_id = Uuid::new_v4();
        let username = "alice".to_string();
        let token = create_jwt(user_id, username.clone()).expect("token creation must succeed");
        let claims = verify_jwt(&token).expect("token verification must succeed");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
        assert!(claims.exp >= claims.iat);
    }
}
