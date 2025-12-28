use crate::AppState;
use crate::config::get_auth_secret;
use crate::utils::crypto::{decrypt, encrypt};
use crate::utils::jwt::{AuthUser, create_jwt};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use shared::auth::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use sqlx::Error;
use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};

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
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Генерируем секретный ключ
    let secret = Secret::generate_secret();
    let secret_bytes = secret.to_bytes().unwrap();
    // Конвертируем в String для сохранения в БД (обычно это Base32 строка)
    let secret_str = secret.to_encoded().to_string();

    // 2. Создаем объект TOTP для генерации QR
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("DnD-VTT".to_string()), // Issuer (название приложения в телефоне)
        payload.username.clone(),    // Account name (логин юзера в телефоне)
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 3. Генерируем QR-код в Base64 (png)
    let qr_code = totp
        .get_qr_base64()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let encrypted_secret = encrypt(&secret_str, get_auth_secret())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // 4. Сохраняем пользователя в БД
    // ВАЖНО: В реальном проекте секрет лучше шифровать перед записью в БД
    let result = sqlx::query!(
        r#"
        INSERT INTO users (username, totp_secret)
        VALUES ($1, $2)
        RETURNING id
        "#,
        payload.username,
        encrypted_secret
    )
    .fetch_one(state.get_pg_pool())
    .await;

    match result {
        Ok(_) => Ok(Json(RegisterResponse {
            qr_code_base64: qr_code,
            message: "User created. Scan this QR code with Google Authenticator app.".to_string(),
        })),
        Err(Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err((StatusCode::CONFLICT, "Username already exists".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
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
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Ищем пользователя и его секрет
    let user = sqlx::query!(
        "SELECT id, totp_secret FROM users WHERE username = $1",
        payload.username
    )
    .fetch_optional(state.get_pg_pool())
    .await
    .map_err(|e: Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = match user {
        Some(u) => u,
        None => return Err((StatusCode::UNAUTHORIZED, "User not found".to_string())),
    };

    let secret = decrypt(&user.totp_secret, get_auth_secret()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Crypto error: {}", e),
        )
    })?;

    let secret = Secret::Encoded(secret).to_bytes().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Secret decode error: {}", e),
        )
    })?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some("DnD-App".to_string()),
        payload.username.clone(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 3. Проверяем код
    // check_current вернет true, если код валиден прямо сейчас
    let is_valid = totp
        .check_current(&payload.code)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if is_valid {
        let token = create_jwt(user.id, payload.username).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Token creation failed: {}", e),
            )
        })?;
        Ok(Json(LoginResponse { token }))
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            "Invalid authenticator code".to_string(),
        ))
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
