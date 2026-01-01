use serde::{Deserialize, Serialize};
use shared::events::RoomState;

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalStorageData {
    pub state: RoomState,
}

fn get_storage_key(room_name: &str) -> String {
    format!("dnd_room_state:{}", room_name)
}

pub fn load_state(room_name: &str) -> Option<LocalStorageData> {
    let key = get_storage_key(room_name);
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item(&key) {
                return serde_json::from_str(&json).ok();
            }
        }
    }
    None
}

pub fn save_state(room_name: &str, state: &RoomState) {
    let key = get_storage_key(room_name);
    let data = LocalStorageData {
        state: state.clone(),
    };
    if let Ok(json) = serde_json::to_string(&data) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(&key, &json);
            }
        }
    }
}
