use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct RegisterRequest {
    pub username: String,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct RegisterResponse {
    pub qr_code_base64: String, // Картинка QR-кода, чтобы показать юзеру
    pub message: String,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct LoginRequest {
    pub username: String,
    pub code: String, // 6 цифр из приложения
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct LoginResponse {
    pub token: String, // В будущем здесь будет JWT, пока просто заглушка
}
