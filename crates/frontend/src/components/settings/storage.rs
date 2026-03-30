const WORKSPACE_HINT_STORAGE_KEY: &str = "settings:show_workspace_hint";
const INACTIVE_SCENE_CONTENTS_STORAGE_KEY: &str = "settings:show_inactive_scene_contents";

fn decode_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

pub fn load_workspace_hint_visibility() -> Option<bool> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok().flatten()?;
    let value = storage
        .get_item(WORKSPACE_HINT_STORAGE_KEY)
        .ok()
        .flatten()?;
    decode_bool(&value)
}

pub fn save_workspace_hint_visibility(is_visible: bool) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let value = if is_visible { "true" } else { "false" };
    let _ = storage.set_item(WORKSPACE_HINT_STORAGE_KEY, value);
}

pub fn load_inactive_scene_contents_visibility() -> Option<bool> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok().flatten()?;
    let value = storage
        .get_item(INACTIVE_SCENE_CONTENTS_STORAGE_KEY)
        .ok()
        .flatten()?;
    decode_bool(&value)
}

pub fn save_inactive_scene_contents_visibility(is_visible: bool) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let value = if is_visible { "true" } else { "false" };
    let _ = storage.set_item(INACTIVE_SCENE_CONTENTS_STORAGE_KEY, value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_bool_parses_known_values() {
        assert_eq!(decode_bool("true"), Some(true));
        assert_eq!(decode_bool("false"), Some(false));
    }

    #[test]
    fn decode_bool_rejects_unknown_values() {
        assert_eq!(decode_bool("1"), None);
        assert_eq!(decode_bool("nope"), None);
    }
}
