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
    // Auth form colors
    pub auth_form_bg: &'static str,
    pub auth_input_bg: &'static str,
    pub auth_input_border: &'static str,
    pub auth_input_text: &'static str,
    pub auth_error_bg: &'static str,
    pub auth_error_border: &'static str,
    pub auth_error_text: &'static str,
    pub auth_success_bg: &'static str,
    pub auth_success_border: &'static str,
    pub auth_success_text: &'static str,
    pub auth_button_login: &'static str,
    pub auth_button_register: &'static str,
    pub auth_button_room: &'static str,
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
    while i < bytes.len() {
        let digit = bytes[i].wrapping_sub(b'0');
        assert!(digit < 10, "Invalid digit in numeric string");
        result = result * 10 + digit as u32;
        i += 1;
    }
    result
}

const fn parse_u64(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut result = 0u64;
    let mut i = 0;
    while i < bytes.len() {
        let digit = bytes[i].wrapping_sub(b'0');
        assert!(digit < 10, "Invalid digit in numeric string");
        result = result * 10 + digit as u64;
        i += 1;
    }
    result
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            my_cursor_color: env!("MY_CURSOR_COLOR"),
            other_cursor_color: env!("OTHER_CURSOR_COLOR"),
            cursor_size: parse_u32(CURSOR_SIZE_STR),
            mouse_throttle_ms: parse_u64(MOUSE_THROTTLE_MS_STR),
            background_color: env!("BACKGROUND_COLOR"),
            cursor_transition: env!("CURSOR_TRANSITION"),
            auth_form_bg: env!("AUTH_FORM_BG"),
            auth_input_bg: env!("AUTH_INPUT_BG"),
            auth_input_border: env!("AUTH_INPUT_BORDER"),
            auth_input_text: env!("AUTH_INPUT_TEXT"),
            auth_error_bg: env!("AUTH_ERROR_BG"),
            auth_error_border: env!("AUTH_ERROR_BORDER"),
            auth_error_text: env!("AUTH_ERROR_TEXT"),
            auth_success_bg: env!("AUTH_SUCCESS_BG"),
            auth_success_border: env!("AUTH_SUCCESS_BORDER"),
            auth_success_text: env!("AUTH_SUCCESS_TEXT"),
            auth_button_login: env!("AUTH_BUTTON_LOGIN"),
            auth_button_register: env!("AUTH_BUTTON_REGISTER"),
            auth_button_room: env!("AUTH_BUTTON_ROOM"),
        }
    }
}

impl Default for Api {
    fn default() -> Self {
        Self {
            back_url: env!("BACK_URL"),
            ws_path: env!("WS_PATH"),
            api_path: env!("API_PATH"),
        }
    }
}
