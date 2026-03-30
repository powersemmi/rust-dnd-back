use super::model::{
    DEFAULT_TOKEN_HEIGHT_CELLS, DEFAULT_TOKEN_WIDTH_CELLS, TokenLibraryValidationError,
    validate_token_form,
};
use crate::components::websocket::{StoredTokenLibraryItem, token_library_key};
use leptos::prelude::*;
use shared::events::FileRef;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct TokensWindowViewModel {
    pub editing_id: RwSignal<Option<String>>,
    pub draft_name: RwSignal<String>,
    pub draft_width_cells: RwSignal<String>,
    pub draft_height_cells: RwSignal<String>,
    pub draft_image: RwSignal<Option<FileRef>>,
    pub error: RwSignal<Option<String>>,
}

impl TokensWindowViewModel {
    pub fn new() -> Self {
        Self {
            editing_id: RwSignal::new(None),
            draft_name: RwSignal::new(String::new()),
            draft_width_cells: RwSignal::new(DEFAULT_TOKEN_WIDTH_CELLS.to_string()),
            draft_height_cells: RwSignal::new(DEFAULT_TOKEN_HEIGHT_CELLS.to_string()),
            draft_image: RwSignal::new(None),
            error: RwSignal::new(None),
        }
    }

    pub fn reset(&self) {
        self.editing_id.set(None);
        self.draft_name.set(String::new());
        self.draft_width_cells
            .set(DEFAULT_TOKEN_WIDTH_CELLS.to_string());
        self.draft_height_cells
            .set(DEFAULT_TOKEN_HEIGHT_CELLS.to_string());
        self.draft_image.set(None);
        self.error.set(None);
    }

    pub fn apply_item(&self, item: &StoredTokenLibraryItem) {
        self.editing_id.set(Some(item.id.clone()));
        self.draft_name.set(item.name.clone());
        self.draft_width_cells.set(item.width_cells.to_string());
        self.draft_height_cells.set(item.height_cells.to_string());
        self.draft_image.set(Some(item.image.clone()));
        self.error.set(None);
    }

    pub fn build_item(
        &self,
        room_name: &str,
        empty_name_error: &str,
        missing_image_error: &str,
        invalid_dimensions_error: &str,
    ) -> Option<StoredTokenLibraryItem> {
        let name = self.draft_name.get_untracked();
        let width_cells = self.draft_width_cells.get_untracked();
        let height_cells = self.draft_height_cells.get_untracked();
        let image = self.draft_image.get_untracked();

        let (validated_width_cells, validated_height_cells) =
            match validate_token_form(&name, &width_cells, &height_cells, image.is_some()) {
                Ok(dimensions) => dimensions,
                Err(TokenLibraryValidationError::EmptyName) => {
                    self.error.set(Some(empty_name_error.to_string()));
                    return None;
                }
                Err(TokenLibraryValidationError::MissingImage) => {
                    self.error.set(Some(missing_image_error.to_string()));
                    return None;
                }
                Err(TokenLibraryValidationError::InvalidDimensions) => {
                    self.error.set(Some(invalid_dimensions_error.to_string()));
                    return None;
                }
            };

        let id = self
            .editing_id
            .get_untracked()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let item = StoredTokenLibraryItem {
            key: token_library_key(room_name, &id),
            room_name: room_name.to_string(),
            id,
            name: name.trim().to_string(),
            image: image.expect("validated image presence"),
            width_cells: validated_width_cells,
            height_cells: validated_height_cells,
        };
        self.error.set(None);
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    fn file_ref() -> FileRef {
        FileRef {
            hash: "hash".to_string(),
            mime_type: "image/png".to_string(),
            file_name: "goblin.png".to_string(),
            size: 128,
        }
    }

    #[test]
    fn build_item_creates_record_with_room_key() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = TokensWindowViewModel::new();
            vm.draft_name.set("Goblin".to_string());
            vm.draft_width_cells.set("2".to_string());
            vm.draft_height_cells.set("3".to_string());
            vm.draft_image.set(Some(file_ref()));

            let item = vm.build_item("room-a", "empty", "image", "size").unwrap();

            assert_eq!(item.room_name, "room-a");
            assert_eq!(item.name, "Goblin");
            assert_eq!(item.width_cells, 2);
            assert_eq!(item.height_cells, 3);
            assert!(item.key.starts_with("room-a:"));
        });
    }

    #[test]
    fn apply_item_populates_draft_fields() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = TokensWindowViewModel::new();
            let item = StoredTokenLibraryItem {
                key: "room:t1".to_string(),
                room_name: "room".to_string(),
                id: "t1".to_string(),
                name: "Knight".to_string(),
                image: file_ref(),
                width_cells: 2,
                height_cells: 4,
            };

            vm.apply_item(&item);

            assert_eq!(vm.editing_id.get_untracked(), Some("t1".to_string()));
            assert_eq!(vm.draft_name.get_untracked(), "Knight");
            assert_eq!(vm.draft_width_cells.get_untracked(), "2");
            assert_eq!(vm.draft_height_cells.get_untracked(), "4");
            assert!(vm.draft_image.get_untracked().is_some());
        });
    }
}
