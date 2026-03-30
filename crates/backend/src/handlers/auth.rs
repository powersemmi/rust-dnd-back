use crate::config::get_secret;
use crate::utils::crypto::{decrypt, encrypt};
use crate::utils::jwt::{AuthUser, create_jwt};
use crate::{AppError, AppResult, AppState};
use axum::{Json, extract::State, response::IntoResponse};
use serde_json::json;
use shared::auth::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use sqlx::Error;
use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};

const TOTP_DIGITS: usize = 6;
const TOTP_SKEW: u8 = 1;
const TOTP_STEP_SECONDS: u64 = 30;
const TOTP_ISSUER: &str = "DnD-VTT";

#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Успешная регистрация, возвращает QR-код", body = RegisterResponse),
        (status = 409, description = "Пользователь с таким именем уже существует"),
        (status = 500, description = "Внутренняя ошибка сервера")
    )
)]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<impl IntoResponse> {
    let username = payload.username;
    let secret = Secret::generate_secret();
    let secret_bytes = secret.to_bytes().map_err(|error| {
        AppError::internal(format!(
            "Failed to serialize generated TOTP secret: {error}"
        ))
    })?;
    let secret_str = secret.to_encoded().to_string();

    let totp = build_totp(&secret_bytes, &username)?;
    let qr_code = totp
        .get_qr_base64()
        .map_err(|error| AppError::internal(format!("Failed to generate QR code: {error}")))?;

    let encrypted_secret = encrypt(&secret_str, get_secret("AUTH_SECRET"))
        .map_err(|error| AppError::internal(format!("Failed to encrypt TOTP secret: {error}")))?;

    let result = sqlx::query!(
        r#"
        INSERT INTO users (username, totp_secret)
        VALUES ($1, $2)
        RETURNING id
        "#,
        username,
        encrypted_secret
    )
    .fetch_one(&state.pg_pool)
    .await;

    match result {
        Ok(_) => Ok(Json(RegisterResponse {
            qr_code_base64: qr_code,
            message: "User created. Scan this QR code with Google Authenticator app.".to_string(),
        })),
        Err(Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(AppError::conflict("Username already exists"))
        }
        Err(error) => Err(AppError::internal(error.to_string())),
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Успешный вход, возвращает JWT токен", body = LoginResponse),
        (status = 401, description = "Неверный логин или TOTP код"),
        (status = 500, description = "Внутренняя ошибка сервера")
    )
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let username = payload.username;
    let code = payload.code;

    let user = sqlx::query!(
        "SELECT id, totp_secret FROM users WHERE username = $1",
        &username
    )
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|error| AppError::internal(error.to_string()))?;

    let user = match user {
        Some(user) => user,
        None => return Err(AppError::unauthorized("User not found")),
    };

    let secret = decrypt(&user.totp_secret, get_secret("AUTH_SECRET"))
        .map_err(|error| AppError::internal(format!("Crypto error: {error}")))?;
    let secret_bytes = Secret::Encoded(secret)
        .to_bytes()
        .map_err(|error| AppError::internal(format!("Secret decode error: {error}")))?;

    if verify_totp_code(&secret_bytes, &username, &code)? {
        let token = create_jwt(user.id, username)
            .map_err(|error| AppError::internal(format!("Token creation failed: {error}")))?;
        Ok(Json(LoginResponse { token }))
    } else {
        Err(AppError::unauthorized("Invalid authenticator code"))
    }
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Информация о текущем пользователе")
    )
)]
pub async fn get_me(auth_user: AuthUser) -> impl IntoResponse {
    Json(json!({
        "id": auth_user.user_id,
        "username": auth_user.username,
        "message": "You are authorized!"
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Новый JWT токен", body = LoginResponse),
        (status = 401, description = "Токен невалиден или истёк")
    )
)]
pub async fn refresh_token(auth_user: AuthUser) -> AppResult<impl IntoResponse> {
    let new_token = create_jwt(auth_user.user_id, auth_user.username)
        .map_err(|error| AppError::internal(format!("Token creation failed: {error}")))?;

    Ok(Json(LoginResponse { token: new_token }))
}

fn build_totp(secret_bytes: &[u8], username: &str) -> AppResult<TOTP> {
    TOTP::new(
        Algorithm::SHA1,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_STEP_SECONDS,
        secret_bytes.to_vec(),
        Some(TOTP_ISSUER.to_string()),
        username.to_string(),
    )
    .map_err(|error| AppError::internal(format!("Failed to build TOTP: {error}")))
}

pub(crate) fn verify_totp_code(secret_bytes: &[u8], username: &str, code: &str) -> AppResult<bool> {
    let totp = build_totp(secret_bytes, username)?;
    totp.check_current(code)
        .map_err(|error| AppError::internal(format!("Failed to validate TOTP code: {error}")))
}

#[cfg(test)]
mod tests {
    use super::{build_totp, verify_totp_code};
    use totp_rs::Secret;

    #[test]
    fn verify_totp_accepts_current_code() {
        let secret = Secret::generate_secret();
        let secret_bytes = secret.to_bytes().expect("secret generation must succeed");
        let totp = build_totp(&secret_bytes, "alice").expect("totp must be created");
        let code = totp
            .generate_current()
            .expect("code generation must succeed");

        let is_valid =
            verify_totp_code(&secret_bytes, "alice", &code).expect("verification must succeed");

        assert!(is_valid);
    }

    #[test]
    fn verify_totp_rejects_invalid_code() {
        let secret = Secret::generate_secret();
        let secret_bytes = secret.to_bytes().expect("secret generation must succeed");

        let is_valid =
            verify_totp_code(&secret_bytes, "alice", "000000").expect("verification must succeed");

        assert!(!is_valid);
    }
}
