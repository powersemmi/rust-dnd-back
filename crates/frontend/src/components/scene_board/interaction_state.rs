// State structs and view-layer constants for the scene board.
// These are plain data types with no business logic.

use shared::events::{NoteVisibility, Scene, Token};

// ---------------------------------------------------------------------------
// Local interaction state structs
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SceneLayout {
    pub scene: Scene,
    pub cell_size: f64,
    pub board_width: f64,
    pub board_height: f64,
}

#[derive(Clone)]
pub struct TokenMenuState {
    pub scene_id: String,
    pub token_id: String,
    pub token: Token,
    pub token_name: String,
    pub screen_x: f64,
    pub screen_y: f64,
}

#[derive(Clone)]
pub struct BoardNoteSelection {
    pub note_id: String,
    pub visibility: NoteVisibility,
}

#[derive(Clone)]
pub struct BoardNoteEditorDraft {
    pub note_id: String,
    pub visibility: NoteVisibility,
    pub body: String,
}

#[derive(Clone)]
pub struct BoardNoteDragState {
    pub note_id: String,
    pub visibility: NoteVisibility,
    pub pointer_offset_x: f64,
    pub pointer_offset_y: f64,
    pub start_note_x: f64,
    pub start_note_y: f64,
}

#[derive(Clone)]
pub struct BoardNoteResizeState {
    pub note_id: String,
    pub visibility: NoteVisibility,
    pub start_world_x: f64,
    pub start_world_y: f64,
    pub start_width_px: f64,
    pub start_height_px: f64,
}

#[derive(Clone)]
pub struct BoardNoteClickState {
    pub note_id: String,
    pub visibility: NoteVisibility,
    pub at_ms: f64,
}

// ---------------------------------------------------------------------------
// Board-note view constants
// ---------------------------------------------------------------------------

pub const TOKEN_DRAG_EPSILON_CELLS: f32 = 0.02;
pub const BOARD_NOTE_MIN_WIDTH_PX: f64 = 180.0;
pub const BOARD_NOTE_MIN_HEIGHT_PX: f64 = 140.0;
pub const BOARD_NOTE_MAX_WIDTH_PX: f64 = 2100.0;
pub const BOARD_NOTE_MAX_HEIGHT_PX: f64 = 2100.0;
pub const BOARD_NOTE_TOOLBAR_HEIGHT_PX: f64 = 38.0;
pub const BOARD_NOTE_RESIZE_HANDLE_PX: f64 = 16.0;
pub const BOARD_NOTE_EDIT_PADDING_PX: f64 = 22.0;
pub const BOARD_NOTE_DOUBLE_CLICK_MS: f64 = 320.0;
pub const BOARD_NOTE_MIN_FONT_SIZE_PT: f64 = 8.0;
pub const BOARD_NOTE_MAX_FONT_SIZE_PT: f64 = 72.0;
pub const BOARD_NOTE_FONT_SIZE_STEP_PT: f64 = 2.0;
pub const BOARD_NOTE_COLORS: [&str; 5] = ["#F8EE96", "#F7C5D5", "#BDEBD3", "#C7DCF9", "#F6D0A6"];
