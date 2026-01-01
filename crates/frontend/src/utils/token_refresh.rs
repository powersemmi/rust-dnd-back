use gloo_timers::callback::Interval;
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize, Deserialize)]
struct RefreshResponse {
    token: String,
}

/// Начинает периодическое обновление токена
/// Обновляет токен каждые 20 часов (токен живет 24 часа)
pub fn start_token_refresh(back_url: &'static str, api_path: &'static str) {
    let refresh_interval_ms = 20 * 60 * 60 * 1000; // 20 часов в миллисекундах

    debug!("Starting token refresh timer (every 20 hours)");

    // Создаём интервал для периодического обновления
    let _interval = Interval::new(refresh_interval_ms, move || {
        spawn_local(async move {
            if let Err(e) = refresh_token_request(back_url, api_path).await {
                error!("Failed to refresh token: {}", e);
            }
        });
    });

    // Важно: interval должен остаться в памяти, иначе таймер остановится
    // Используем forget, чтобы он жил до конца сессии
    std::mem::forget(_interval);
}

async fn refresh_token_request(back_url: &str, api_path: &str) -> Result<(), String> {
    debug!("Attempting to refresh token...");

    // Получаем текущий токен из localStorage
    let token = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("jwt_token").ok().flatten())
        .ok_or_else(|| "No token found in localStorage".to_string())?;

    let url = format!("{}{}/auth/refresh", back_url, api_path);

    // Отправляем запрос на обновление
    let response = gloo_net::http::Request::post(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        if status == 401 {
            warn!("Token refresh failed: unauthorized. Redirecting to login...");
            // Удаляем старый токен
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.remove_item("jwt_token");
                    let _ = storage.remove_item("username");
                }
                // Перенаправляем на главную (где будет форма логина)
                let _ = window.location().set_href("/");
            }
        }

        return Err(format!("HTTP {}: {}", status, error_text));
    }

    let refresh_response: RefreshResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Сохраняем новый токен
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            storage
                .set_item("jwt_token", &refresh_response.token)
                .map_err(|_| "Failed to save new token".to_string())?;
            debug!("Token refreshed successfully");
        }
    }

    Ok(())
}
