// Pure types, constants, and validation for scene management.
// No signals, no Leptos, no web_sys.

use crate::components::scene_board::model::WORKSPACE_SCENE_CELL_SIZE_PX;

// --- Constants ---

pub const MAX_SCENES_PER_ROOM: usize = 50;
pub const DEFAULT_COLUMNS: &str = "24";
pub const DEFAULT_ROWS: &str = "16";
pub const DEFAULT_CELL_SIZE_FEET: &str = "5";
pub const DEFAULT_SCENE_SPACING_GAP_PX: f32 = 240.0;
pub const FILE_INPUT_ACCEPT: &str = "image/png,image/jpeg,image/webp,image/gif";
pub const DEFAULT_BACKGROUND_SCALE: f32 = 1.0;
pub const DEFAULT_BACKGROUND_OFFSET_X: f32 = 0.0;
pub const DEFAULT_BACKGROUND_OFFSET_Y: f32 = 0.0;
pub const DEFAULT_BACKGROUND_ROTATION_DEG: f32 = 0.0;
pub const MIN_BACKGROUND_SCALE: f32 = 0.1;
pub const MAX_BACKGROUND_SCALE: f32 = 4.0;
pub const MIN_BACKGROUND_OFFSET_PX: f32 = -2000.0;
pub const MAX_BACKGROUND_OFFSET_PX: f32 = 2000.0;
pub const MIN_BACKGROUND_ROTATION_DEG: f32 = -180.0;
pub const MAX_BACKGROUND_ROTATION_DEG: f32 = 180.0;

// Geometry constants for board fit preview
const BOARD_SIDE_PADDING_PX: f64 = 220.0;
const BOARD_TOP_PADDING_PX: f64 = 180.0;
const BOARD_BOTTOM_PADDING_PX: f64 = 140.0;
const MAX_CELL_SIZE_PX: f64 = 72.0;
const MIN_CELL_SIZE_PX: f64 = 18.0;
const FIT_PREVIEW_SIDEBAR_WIDTH_PX: f64 = 560.0;
const FIT_PREVIEW_HORIZONTAL_CHROME_PX: f64 = 120.0;
const FIT_PREVIEW_VERTICAL_CHROME_PX: f64 = 140.0;

// --- Types ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneValidationError {
    EmptyName,
    InvalidGrid,
    #[allow(dead_code)] // reserved for when MAX_SCENES_PER_ROOM check moves to frontend
    LimitReached,
}

#[derive(Clone, Copy)]
pub struct FitPreviewLayout {
    pub cell_size: f64,
    pub board_width: f64,
    pub board_height: f64,
    pub scale: f64,
}

// --- Pure functions ---

/// Returns the default workspace position for the nth scene (0-indexed count).
pub fn default_scene_position(scene_count: usize, columns: u16, rows: u16) -> (f32, f32) {
    let board_width = columns.max(1) as f32 * WORKSPACE_SCENE_CELL_SIZE_PX as f32;
    let board_height = rows.max(1) as f32 * WORKSPACE_SCENE_CELL_SIZE_PX as f32;
    let spacing = board_width.max(board_height) + DEFAULT_SCENE_SPACING_GAP_PX;
    (scene_count as f32 * spacing, 0.0)
}

/// Validates and parses grid fields. Returns `Err(SceneValidationError)` on failure.
pub fn validate_grid(
    name: &str,
    columns_str: &str,
    rows_str: &str,
    cell_size_feet_str: &str,
) -> Result<shared::events::SceneGrid, SceneValidationError> {
    if name.trim().is_empty() {
        return Err(SceneValidationError::EmptyName);
    }

    let parse = |s: &str| s.parse::<u16>().ok();
    let columns = parse(columns_str);
    let rows = parse(rows_str);
    let cell_size_feet = parse(cell_size_feet_str);

    match (columns, rows, cell_size_feet) {
        (Some(c), Some(r), Some(f))
            if (1..=200).contains(&c) && (1..=200).contains(&r) && (1..=100).contains(&f) =>
        {
            Ok(shared::events::SceneGrid {
                columns: c,
                rows: r,
                cell_size_feet: f,
            })
        }
        _ => Err(SceneValidationError::InvalidGrid),
    }
}

/// Computes board metrics (cell_size, board_width, board_height) for display.
pub fn board_metrics(
    columns: u16,
    rows: u16,
    viewport_width: f64,
    viewport_height: f64,
) -> (f64, f64, f64) {
    let usable_width = (viewport_width - BOARD_SIDE_PADDING_PX).max(320.0);
    let usable_height =
        (viewport_height - BOARD_TOP_PADDING_PX - BOARD_BOTTOM_PADDING_PX).max(240.0);
    let cols = f64::from(columns.max(1));
    let rows_f = f64::from(rows.max(1));
    let cell_size = (usable_width / cols)
        .min(usable_height / rows_f)
        .clamp(MIN_CELL_SIZE_PX, MAX_CELL_SIZE_PX);
    (cell_size, cols * cell_size, rows_f * cell_size)
}

/// Computes the fit-preview layout (board metrics + scale) for the background fit editor.
pub fn fit_preview_layout(
    columns: u16,
    rows: u16,
    viewport_width: f64,
    viewport_height: f64,
) -> FitPreviewLayout {
    let (cell_size, board_width, board_height) =
        board_metrics(columns, rows, viewport_width, viewport_height);
    let available_width =
        (viewport_width - FIT_PREVIEW_SIDEBAR_WIDTH_PX - FIT_PREVIEW_HORIZONTAL_CHROME_PX)
            .max(280.0);
    let available_height = (viewport_height - FIT_PREVIEW_VERTICAL_CHROME_PX).max(240.0);
    let scale = (available_width / board_width)
        .min(available_height / board_height)
        .min(1.0);
    FitPreviewLayout {
        cell_size,
        board_width,
        board_height,
        scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scene_position_first() {
        assert_eq!(default_scene_position(0, 24, 16), (0.0, 0.0));
    }

    #[test]
    fn default_scene_position_increments_by_board_size_plus_gap() {
        let (x, y) = default_scene_position(2, 24, 16);
        let expected_spacing =
            24.0 * WORKSPACE_SCENE_CELL_SIZE_PX as f32 + DEFAULT_SCENE_SPACING_GAP_PX;
        assert_eq!(x, expected_spacing * 2.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn validate_grid_ok() {
        let result = validate_grid("Test", "24", "16", "5");
        assert!(result.is_ok());
        let grid = result.unwrap();
        assert_eq!(grid.columns, 24);
        assert_eq!(grid.rows, 16);
        assert_eq!(grid.cell_size_feet, 5);
    }

    #[test]
    fn validate_grid_empty_name() {
        let result = validate_grid("  ", "24", "16", "5");
        assert!(matches!(result, Err(SceneValidationError::EmptyName)));
    }

    #[test]
    fn validate_grid_invalid_columns() {
        let result = validate_grid("Test", "abc", "16", "5");
        assert!(matches!(result, Err(SceneValidationError::InvalidGrid)));
    }

    #[test]
    fn validate_grid_out_of_range() {
        let result = validate_grid("Test", "0", "16", "5");
        assert!(matches!(result, Err(SceneValidationError::InvalidGrid)));
    }

    #[test]
    fn board_metrics_clamps_cell_size() {
        let (cell, _, _) = board_metrics(1, 1, 320.0, 240.0);
        assert!(cell >= MIN_CELL_SIZE_PX && cell <= MAX_CELL_SIZE_PX);
    }

    #[test]
    fn fit_preview_scale_at_most_one() {
        let layout = fit_preview_layout(24, 16, 1280.0, 720.0);
        assert!(layout.scale <= 1.0);
    }
}
