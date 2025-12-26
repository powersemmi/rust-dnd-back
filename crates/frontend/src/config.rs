/// Конфигурация фронтенда

/// URL WebSocket сервера (можно переопределить через переменную окружения)
pub fn ws_url() -> String {
    option_env!("WS_URL")
        .unwrap_or("ws://localhost:3000")
        .to_string()
}

/// Путь к WebSocket endpoint для комнаты
pub const WS_ROOM_PATH: &str = "/ws/room";

/// Цвет курсора текущего пользователя
pub const MY_CURSOR_COLOR: &str = "#4CAF50"; // Зеленый

/// Цвет курсора других пользователей
pub const OTHER_CURSOR_COLOR: &str = "#FF5722"; // Красный

/// Размер курсора (ширина и высота в пикселях)
pub const CURSOR_SIZE: u32 = 24;

/// Время троттлинга отправки событий движения мыши (в миллисекундах)
pub const MOUSE_THROTTLE_MS: u64 = 10;

/// ID комнаты по умолчанию
pub const DEFAULT_ROOM_ID: &str = "lobby";

/// Цвет фона приложения
pub const BACKGROUND_COLOR: &str = "#333";

/// Время перехода курсора (CSS transition)
pub const CURSOR_TRANSITION: &str = "transform 0.1s linear";
