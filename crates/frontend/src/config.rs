#[derive(Clone)]
pub struct Config {
    pub theme: Theme,
    pub api: Api,
}

#[derive(Clone)]
pub struct Theme {
    pub my_cursor_color: &'static str,
    pub other_cursor_color: &'static str,
    pub cursor_size: u32,
    pub mouse_throttle_ms: u64,
    pub background_color: &'static str,
    pub cursor_transition: &'static str,
    // Common UI colors (reusable across all components)
    pub ui_bg_primary: &'static str, // #2a2a2a - main background for cards, inputs
    pub ui_bg_secondary: &'static str, // #374151 - secondary background
    pub ui_border: &'static str,     // #444 - borders
    pub ui_text_primary: &'static str, // white - main text
    pub ui_text_secondary: &'static str, // #9ca3af - secondary/muted text
    pub ui_text_muted: &'static str, // #666 - very muted text
    pub ui_button_primary: &'static str, // #2563eb - primary action buttons
    pub ui_button_danger: &'static str, // #ef4444 - danger/delete buttons
    pub ui_success: &'static str,    // #10b981 - success states
    pub ui_notification: &'static str, // #fbbf24 - notification/alert color (yellow)
}

#[derive(Clone)]
pub struct Api {
    pub back_url: &'static str,
    pub ws_path: &'static str,
    pub api_path: &'static str,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            api: Api::default(),
        }
    }
}

const CURSOR_SIZE_STR: &str = env!("CURSOR_SIZE");
const MOUSE_THROTTLE_MS_STR: &str = env!("MOUSE_THROTTLE_MS");

const fn parse_u32(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut result = 0u32;
    let mut i = 0;
    // Пропускаем кавычки если они есть
    while i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
        i += 1;
    }
    while i < bytes.len() {
        let digit = bytes[i].wrapping_sub(b'0');
        if digit < 10 {
            result = result * 10 + digit as u32;
        }
        i += 1;
    }
    result
}

const fn parse_u64(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut result = 0u64;
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
        i += 1;
    }
    while i < bytes.len() {
        let digit = bytes[i].wrapping_sub(b'0');
        if digit < 10 {
            result = result * 10 + digit as u64;
        }
        i += 1;
    }
    result
}

macro_rules! color_env {
    ($name:expr) => {
        env!($name).trim_matches(|c| c == '"' || c == '\'')
    };
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            my_cursor_color: color_env!("MY_CURSOR_COLOR"),
            other_cursor_color: color_env!("OTHER_CURSOR_COLOR"),
            cursor_size: parse_u32(CURSOR_SIZE_STR),
            mouse_throttle_ms: parse_u64(MOUSE_THROTTLE_MS_STR),
            background_color: color_env!("BACKGROUND_COLOR"),
            cursor_transition: color_env!("CURSOR_TRANSITION"),
            ui_bg_primary: color_env!("UI_BG_PRIMARY"),
            ui_bg_secondary: color_env!("UI_BG_SECONDARY"),
            ui_border: color_env!("UI_BORDER"),
            ui_text_primary: color_env!("UI_TEXT_PRIMARY"),
            ui_text_secondary: color_env!("UI_TEXT_SECONDARY"),
            ui_text_muted: color_env!("UI_TEXT_MUTED"),
            ui_button_primary: color_env!("UI_BUTTON_PRIMARY"),
            ui_button_danger: color_env!("UI_BUTTON_DANGER"),
            ui_success: color_env!("UI_SUCCESS"),
            ui_notification: color_env!("UI_NOTIFICATION"),
        }
    }
}

impl Default for Api {
    fn default() -> Self {
        Self {
            back_url: color_env!("BACK_URL"),
            ws_path: color_env!("WS_PATH"),
            api_path: color_env!("API_PATH"),
        }
    }
}
