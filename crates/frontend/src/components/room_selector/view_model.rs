use super::model::{RoomInput, RoomValidationError};
use leptos::prelude::*;

/// Reactive state and business logic for the room selector form.
#[derive(Clone, Copy)]
pub struct RoomSelectorViewModel {
    pub room_id: RwSignal<String>,
    pub error_message: RwSignal<Option<String>>,
}

impl RoomSelectorViewModel {
    /// Creates a new ViewModel, loading the last room from localStorage if available.
    pub fn new() -> Self {
        let last_room = load_last_room();
        Self::new_with_room(last_room)
    }

    /// Creates a new ViewModel with the given initial room ID (used in tests).
    pub fn new_with_room(room_id: String) -> Self {
        Self {
            room_id: RwSignal::new(room_id),
            error_message: RwSignal::new(None),
        }
    }

    /// Validates the current room_id. Returns `Some(id)` if valid.
    pub fn validate(&self) -> Option<String> {
        let input = RoomInput {
            room_id: self.room_id.get_untracked(),
        };
        match input.validate() {
            Ok(()) => Some(input.room_id),
            Err(RoomValidationError::EmptyRoomId) => {
                self.error_message
                    .set(Some("Room ID cannot be empty.".into()));
                None
            }
        }
    }

    /// Persists the room ID and invokes `on_selected` if validation passes.
    pub fn submit(&self, on_selected: Callback<String>) {
        let Some(room_id) = self.validate() else {
            return;
        };
        save_last_room(&room_id);
        on_selected.run(room_id);
    }
}

fn load_last_room() -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("last_room_id").ok().flatten())
        .unwrap_or_default()
}

fn save_last_room(room_id: &str) {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let _ = storage.set_item("last_room_id", room_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn validate_returns_none_on_empty_room() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = RoomSelectorViewModel::new_with_room(String::new());
            let result = vm.validate();
            assert!(result.is_none());
            assert!(vm.error_message.get_untracked().is_some());
        });
    }

    #[test]
    fn validate_returns_room_id_on_valid_input() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = RoomSelectorViewModel::new_with_room("dungeon-room-1".into());
            let result = vm.validate();
            assert_eq!(result, Some("dungeon-room-1".to_string()));
        });
    }
}
