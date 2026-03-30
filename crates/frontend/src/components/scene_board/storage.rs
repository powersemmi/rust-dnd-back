use serde::{Deserialize, Serialize};

const CAMERA_STORAGE_KEY_PREFIX: &str = "scene_board_camera:";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StoredCameraPosition {
    pub x: f64,
    pub y: f64,
    #[serde(default = "default_zoom")]
    pub zoom: f64,
}

const fn default_zoom() -> f64 {
    1.0
}

fn storage_key(room_id: &str) -> String {
    format!("{CAMERA_STORAGE_KEY_PREFIX}{room_id}")
}

fn encode_camera_position(position: StoredCameraPosition) -> Result<String, serde_json::Error> {
    serde_json::to_string(&position)
}

fn decode_camera_position(value: &str) -> Option<StoredCameraPosition> {
    serde_json::from_str(value).ok()
}

pub fn load_camera_position(room_id: &str) -> Option<StoredCameraPosition> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok().flatten()?;
    let value = storage.get_item(&storage_key(room_id)).ok().flatten()?;
    decode_camera_position(&value)
}

pub fn save_camera_position(room_id: &str, position: StoredCameraPosition) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let Ok(value) = encode_camera_position(position) else {
        return;
    };
    let _ = storage.set_item(&storage_key(room_id), &value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_key_is_prefixed_by_room() {
        assert_eq!(storage_key("alpha"), "scene_board_camera:alpha");
    }

    #[test]
    fn camera_position_round_trip() {
        let position = StoredCameraPosition {
            x: 12.5,
            y: -99.0,
            zoom: 1.35,
        };
        let encoded = encode_camera_position(position).unwrap();
        assert_eq!(decode_camera_position(&encoded), Some(position));
    }

    #[test]
    fn decode_camera_position_rejects_invalid_json() {
        assert_eq!(decode_camera_position("not-json"), None);
    }

    #[test]
    fn decode_camera_position_supports_legacy_payload_without_zoom() {
        assert_eq!(
            decode_camera_position(r#"{"x":10.0,"y":-5.0}"#),
            Some(StoredCameraPosition {
                x: 10.0,
                y: -5.0,
                zoom: 1.0,
            })
        );
    }
}
