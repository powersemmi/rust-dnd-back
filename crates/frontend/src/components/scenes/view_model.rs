use super::model::{
    DEFAULT_BACKGROUND_OFFSET_X, DEFAULT_BACKGROUND_OFFSET_Y, DEFAULT_BACKGROUND_ROTATION_DEG,
    DEFAULT_BACKGROUND_SCALE, DEFAULT_CELL_SIZE_FEET, DEFAULT_COLUMNS, DEFAULT_ROWS,
    MAX_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_SCALE,
    MIN_BACKGROUND_OFFSET_PX, MIN_BACKGROUND_ROTATION_DEG, MIN_BACKGROUND_SCALE,
    SceneValidationError, validate_grid,
};
use leptos::prelude::*;
use shared::events::{FileRef, SceneGrid};

/// Reactive state for the scene list + editor form.
#[derive(Clone, Copy)]
pub struct ScenesWindowViewModel {
    // Scene selection
    pub selected_scene_id: RwSignal<Option<String>>,

    // Draft fields
    pub draft_name: RwSignal<String>,
    pub draft_columns: RwSignal<String>,
    pub draft_rows: RwSignal<String>,
    pub draft_cell_size_feet: RwSignal<String>,
    pub draft_background: RwSignal<Option<FileRef>>,
    pub draft_background_scale: RwSignal<f32>,
    pub draft_background_offset_x: RwSignal<f32>,
    pub draft_background_offset_y: RwSignal<f32>,
    pub draft_background_rotation_deg: RwSignal<f32>,

    // Background fit editor state
    pub is_background_fit_editor_open: RwSignal<bool>,
    pub is_dragging_background: RwSignal<bool>,
    bg_drag_start_client_x: RwSignal<i32>,
    bg_drag_start_client_y: RwSignal<i32>,
    bg_drag_origin_offset_x: RwSignal<f32>,
    bg_drag_origin_offset_y: RwSignal<f32>,
    pub bg_drag_preview_scale: RwSignal<f64>,

    // Validation error
    pub editor_error: RwSignal<Option<String>>,
}

impl ScenesWindowViewModel {
    pub fn new() -> Self {
        Self {
            selected_scene_id: RwSignal::new(None),
            draft_name: RwSignal::new(String::new()),
            draft_columns: RwSignal::new(DEFAULT_COLUMNS.to_string()),
            draft_rows: RwSignal::new(DEFAULT_ROWS.to_string()),
            draft_cell_size_feet: RwSignal::new(DEFAULT_CELL_SIZE_FEET.to_string()),
            draft_background: RwSignal::new(None),
            draft_background_scale: RwSignal::new(DEFAULT_BACKGROUND_SCALE),
            draft_background_offset_x: RwSignal::new(DEFAULT_BACKGROUND_OFFSET_X),
            draft_background_offset_y: RwSignal::new(DEFAULT_BACKGROUND_OFFSET_Y),
            draft_background_rotation_deg: RwSignal::new(DEFAULT_BACKGROUND_ROTATION_DEG),
            is_background_fit_editor_open: RwSignal::new(false),
            is_dragging_background: RwSignal::new(false),
            bg_drag_start_client_x: RwSignal::new(0),
            bg_drag_start_client_y: RwSignal::new(0),
            bg_drag_origin_offset_x: RwSignal::new(DEFAULT_BACKGROUND_OFFSET_X),
            bg_drag_origin_offset_y: RwSignal::new(DEFAULT_BACKGROUND_OFFSET_Y),
            bg_drag_preview_scale: RwSignal::new(1.0),
            editor_error: RwSignal::new(None),
        }
    }

    pub fn reset_background_fit(&self) {
        self.draft_background_scale.set(DEFAULT_BACKGROUND_SCALE);
        self.draft_background_offset_x
            .set(DEFAULT_BACKGROUND_OFFSET_X);
        self.draft_background_offset_y
            .set(DEFAULT_BACKGROUND_OFFSET_Y);
        self.draft_background_rotation_deg
            .set(DEFAULT_BACKGROUND_ROTATION_DEG);
    }

    pub fn close_background_fit_editor(&self) {
        self.is_background_fit_editor_open.set(false);
        self.is_dragging_background.set(false);
    }

    pub fn reset(&self) {
        self.selected_scene_id.set(None);
        self.draft_name.set(String::new());
        self.draft_columns.set(DEFAULT_COLUMNS.to_string());
        self.draft_rows.set(DEFAULT_ROWS.to_string());
        self.draft_cell_size_feet
            .set(DEFAULT_CELL_SIZE_FEET.to_string());
        self.draft_background.set(None);
        self.reset_background_fit();
        self.close_background_fit_editor();
        self.editor_error.set(None);
    }

    pub fn apply_scene(&self, scene: &shared::events::Scene) {
        self.selected_scene_id.set(Some(scene.id.clone()));
        self.draft_name.set(scene.name.clone());
        self.draft_columns.set(scene.grid.columns.to_string());
        self.draft_rows.set(scene.grid.rows.to_string());
        self.draft_cell_size_feet
            .set(scene.grid.cell_size_feet.to_string());
        self.draft_background.set(scene.background.clone());
        self.draft_background_scale.set(scene.background_scale);
        self.draft_background_offset_x
            .set(scene.background_offset_x);
        self.draft_background_offset_y
            .set(scene.background_offset_y);
        self.draft_background_rotation_deg
            .set(scene.background_rotation_deg);
        self.close_background_fit_editor();
        self.editor_error.set(None);
    }

    /// Validates draft fields and returns the grid, or sets `editor_error` and returns `None`.
    pub fn build_grid(
        &self,
        error_empty_name: String,
        error_invalid_grid: String,
    ) -> Option<SceneGrid> {
        let name = self.draft_name.get_untracked();
        let columns = self.draft_columns.get_untracked();
        let rows = self.draft_rows.get_untracked();
        let cell_size = self.draft_cell_size_feet.get_untracked();

        match validate_grid(&name, &columns, &rows, &cell_size) {
            Ok(grid) => {
                self.editor_error.set(None);
                Some(grid)
            }
            Err(SceneValidationError::EmptyName) => {
                self.editor_error.set(Some(error_empty_name));
                None
            }
            Err(_) => {
                self.editor_error.set(Some(error_invalid_grid));
                None
            }
        }
    }

    pub fn start_background_drag(&self, client_x: i32, client_y: i32, preview_scale: f64) {
        self.is_dragging_background.set(true);
        self.bg_drag_start_client_x.set(client_x);
        self.bg_drag_start_client_y.set(client_y);
        self.bg_drag_origin_offset_x
            .set(self.draft_background_offset_x.get_untracked());
        self.bg_drag_origin_offset_y
            .set(self.draft_background_offset_y.get_untracked());
        self.bg_drag_preview_scale.set(preview_scale.max(0.01));
    }

    pub fn update_background_drag(&self, client_x: i32, client_y: i32) {
        if !self.is_dragging_background.get_untracked() {
            return;
        }
        let scale = self.bg_drag_preview_scale.get_untracked().max(0.01) as f32;
        let dx = (client_x - self.bg_drag_start_client_x.get_untracked()) as f32 / scale;
        let dy = (client_y - self.bg_drag_start_client_y.get_untracked()) as f32 / scale;
        self.draft_background_offset_x.set(
            (self.bg_drag_origin_offset_x.get_untracked() + dx)
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
        );
        self.draft_background_offset_y.set(
            (self.bg_drag_origin_offset_y.get_untracked() + dy)
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
        );
    }

    pub fn stop_background_drag(&self) {
        self.is_dragging_background.set(false);
    }

    pub fn clamp_background_scale(&self, value: f32) -> f32 {
        value.clamp(MIN_BACKGROUND_SCALE, MAX_BACKGROUND_SCALE)
    }

    pub fn clamp_background_rotation(&self, value: f32) -> f32 {
        value.clamp(MIN_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_ROTATION_DEG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn new_has_default_draft_fields() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ScenesWindowViewModel::new();
            assert_eq!(vm.draft_columns.get_untracked(), DEFAULT_COLUMNS);
            assert_eq!(vm.draft_rows.get_untracked(), DEFAULT_ROWS);
            assert!(vm.selected_scene_id.get_untracked().is_none());
        });
    }

    #[test]
    fn reset_clears_all() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ScenesWindowViewModel::new();
            vm.draft_name.set("Test".to_string());
            vm.selected_scene_id.set(Some("id-1".to_string()));
            vm.reset();
            assert_eq!(vm.draft_name.get_untracked(), "");
            assert!(vm.selected_scene_id.get_untracked().is_none());
        });
    }

    #[test]
    fn build_grid_returns_none_on_empty_name() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ScenesWindowViewModel::new();
            let result = vm.build_grid("empty name".to_string(), "invalid grid".to_string());
            assert!(result.is_none());
            assert_eq!(
                vm.editor_error.get_untracked(),
                Some("empty name".to_string())
            );
        });
    }

    #[test]
    fn build_grid_returns_grid_with_valid_input() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ScenesWindowViewModel::new();
            vm.draft_name.set("Battle Arena".to_string());
            let result = vm.build_grid("empty".to_string(), "invalid".to_string());
            assert!(result.is_some());
            let grid = result.unwrap();
            assert_eq!(grid.columns, 24);
        });
    }

    #[test]
    fn start_and_update_background_drag() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ScenesWindowViewModel::new();
            // Set scale to 1.0 so delta is 1:1
            vm.draft_background_offset_x.set(0.0);
            vm.start_background_drag(100, 100, 1.0);
            vm.update_background_drag(150, 120);
            assert_eq!(vm.draft_background_offset_x.get_untracked(), 50.0);
            assert_eq!(vm.draft_background_offset_y.get_untracked(), 20.0);
        });
    }

    #[test]
    fn reset_background_fit_restores_defaults() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ScenesWindowViewModel::new();
            vm.draft_background_scale.set(2.5);
            vm.reset_background_fit();
            assert_eq!(
                vm.draft_background_scale.get_untracked(),
                DEFAULT_BACKGROUND_SCALE
            );
        });
    }
}
