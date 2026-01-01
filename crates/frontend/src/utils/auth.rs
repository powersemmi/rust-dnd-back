use serde::Deserialize;

/// Проверяет, истёк ли JWT токен
pub fn is_token_expired(token: &str) -> bool {
    // Парсим JWT (формат: header.payload.signature)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return true; // Невалидный формат = истёкший
    }

    // Декодируем payload (Base64)
    let payload = parts[1];
    let window = match web_sys::window() {
        Some(w) => w,
        None => return true,
    };

    let decoded = match window.atob(payload) {
        Ok(d) => d,
        Err(_) => return true,
    };

    #[derive(Deserialize)]
    struct TokenPayload {
        exp: u64,
    }

    let payload: TokenPayload = match serde_json::from_str(&decoded) {
        Ok(p) => p,
        Err(_) => return true,
    };

    // Получаем текущее время в секундах
    let now = js_sys::Date::now() / 1000.0;

    // Токен истёк, если текущее время больше exp
    now >= payload.exp as f64
}

/// Загружает токен из localStorage и проверяет его валидность
pub fn load_and_validate_token() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let token = storage.get_item("jwt_token").ok()??;

    if is_token_expired(&token) {
        let _ = storage.remove_item("jwt_token");
        let _ = storage.remove_item("username");
        None
    } else {
        Some(token)
    }
}

/// Загружает username из localStorage
pub fn load_username() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item("username").ok()?
}
