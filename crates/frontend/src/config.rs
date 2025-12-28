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
