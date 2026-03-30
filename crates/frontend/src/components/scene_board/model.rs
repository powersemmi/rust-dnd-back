// Pure geometric types and constants for the scene board.
// No signals, no Leptos, no web_sys.

// --- Constants ---

pub const MIN_ZOOM: f64 = 0.35;
pub const MAX_ZOOM: f64 = 2.5;
pub const ZOOM_STEP: f64 = 0.12;
pub const WORKSPACE_GRID_STEP_PX: f64 = 48.0;
pub const WORKSPACE_SCENE_CELL_SIZE_PX: f64 = 48.0;
pub const SNAP_THRESHOLD_PX: f64 = 56.0;
pub const BOARD_HANDLE_HEIGHT_PX: f64 = 58.0;
pub const BOARD_HANDLE_GAP_PX: f64 = 14.0;
pub const BOARD_HANDLE_MAX_WIDTH_PX: f64 = 440.0;
pub const DRAG_EPSILON_PX: f64 = 1.0;
pub const MIN_TOKEN_SIZE_CELLS: u16 = 1;
pub const MAX_TOKEN_SIZE_CELLS: u16 = 16;

// --- Coordinate conversions ---

/// Converts a viewport-local point to world coordinates.
pub fn screen_to_world(
    local_x: f64,
    local_y: f64,
    viewport_width: f64,
    viewport_height: f64,
    camera_x: f64,
    camera_y: f64,
    zoom: f64,
) -> (f64, f64) {
    let screen_x = local_x - viewport_width / 2.0;
    let screen_y = local_y - viewport_height / 2.0;
    ((screen_x - camera_x) / zoom, (screen_y - camera_y) / zoom)
}

/// Converts world coordinates to viewport-local screen coordinates.
pub fn world_to_screen(
    world_x: f64,
    world_y: f64,
    viewport_width: f64,
    viewport_height: f64,
    camera_x: f64,
    camera_y: f64,
    zoom: f64,
) -> (f64, f64) {
    (
        viewport_width / 2.0 + camera_x + world_x * zoom,
        viewport_height / 2.0 + camera_y + world_y * zoom,
    )
}

/// Clamp zoom level within allowed bounds.
pub fn clamp_zoom(zoom: f64) -> f64 {
    zoom.clamp(MIN_ZOOM, MAX_ZOOM)
}

// --- Hit testing ---

pub fn point_inside_rect(
    point_x: f64,
    point_y: f64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
) -> bool {
    point_x >= left && point_x <= left + width && point_y >= top && point_y <= top + height
}

// --- Selection box ---

/// Returns (left, top, width, height) of the selection rectangle, normalizing corner order.
pub fn selection_box(start_x: f64, start_y: f64, end_x: f64, end_y: f64) -> (f64, f64, f64, f64) {
    let left = start_x.min(end_x);
    let top = start_y.min(end_y);
    let width = (start_x - end_x).abs();
    let height = (start_y - end_y).abs();
    (left, top, width, height)
}

// --- Visual helpers ---

pub fn grid_line_width_screen(screen_cell: f64) -> f64 {
    if screen_cell >= 42.0 {
        1.35
    } else if screen_cell >= 20.0 {
        1.15
    } else {
        1.0
    }
}

pub fn board_background(theme_bg: &str) -> String {
    format!(
        "linear-gradient(180deg, rgba(255,255,255,0.06), rgba(0,0,0,0.12)), \
         radial-gradient(circle at top left, rgba(255,255,255,0.08), transparent 30%), \
         {theme_bg}"
    )
}

pub fn scene_shows_contents(
    scene_id: &str,
    active_scene_id: Option<&str>,
    show_inactive_scene_contents: bool,
) -> bool {
    show_inactive_scene_contents || active_scene_id == Some(scene_id)
}

pub fn scene_allows_token_interaction(
    scene_id: &str,
    active_scene_id: Option<&str>,
    show_inactive_scene_contents: bool,
) -> bool {
    scene_shows_contents(scene_id, active_scene_id, show_inactive_scene_contents)
}

pub fn should_broadcast_cursor(
    hovered_scene_id: Option<&str>,
    active_scene_id: Option<&str>,
    show_inactive_scene_contents: bool,
) -> bool {
    if !show_inactive_scene_contents {
        return true;
    }

    match hovered_scene_id {
        Some(scene_id) => active_scene_id == Some(scene_id),
        None => true,
    }
}

// --- Board metrics ---

/// Computes the stable world-space size of a scene board.
/// This must not depend on viewport size, otherwise scene positions diverge across clients.
pub fn workspace_board_metrics(columns: u16, rows: u16) -> (f64, f64, f64) {
    let cols = f64::from(columns.max(1));
    let rws = f64::from(rows.max(1));
    let cell_size = WORKSPACE_SCENE_CELL_SIZE_PX;
    (cell_size, cols * cell_size, rws * cell_size)
}

pub fn clamp_token_dimension(size: u16) -> u16 {
    size.clamp(MIN_TOKEN_SIZE_CELLS, MAX_TOKEN_SIZE_CELLS)
}

pub fn token_rect(
    board_left: f64,
    board_top: f64,
    cell_size: f64,
    cell_x: f32,
    cell_y: f32,
    width_cells: u16,
    height_cells: u16,
) -> (f64, f64, f64, f64) {
    let width = f64::from(clamp_token_dimension(width_cells)) * cell_size;
    let height = f64::from(clamp_token_dimension(height_cells)) * cell_size;
    (
        board_left + f64::from(cell_x) * cell_size,
        board_top + f64::from(cell_y) * cell_size,
        width,
        height,
    )
}

fn clamp_token_axis(position: f64, axis_cells: u16, token_span_cells: u16) -> f32 {
    let span = clamp_token_dimension(token_span_cells).min(axis_cells);
    let max_position = f64::from(axis_cells.saturating_sub(span));
    position.clamp(0.0, max_position) as f32
}

pub fn token_position_from_world(
    world_x: f64,
    world_y: f64,
    board_left: f64,
    board_top: f64,
    cell_size: f64,
    columns: u16,
    rows: u16,
    token_width_cells: u16,
    token_height_cells: u16,
    cursor_offset_x: f64,
    cursor_offset_y: f64,
) -> (f32, f32) {
    let raw_left = (world_x - cursor_offset_x - board_left) / cell_size;
    let raw_top = (world_y - cursor_offset_y - board_top) / cell_size;
    (
        clamp_token_axis(raw_left, columns, token_width_cells),
        clamp_token_axis(raw_top, rows, token_height_cells),
    )
}

pub fn snap_token_position_to_grid(
    x: f32,
    y: f32,
    columns: u16,
    rows: u16,
    token_width_cells: u16,
    token_height_cells: u16,
) -> (f32, f32) {
    (
        clamp_token_axis(f64::from(x).round(), columns, token_width_cells),
        clamp_token_axis(f64::from(y).round(), rows, token_height_cells),
    )
}

pub fn clamp_token_position(
    x: f32,
    y: f32,
    columns: u16,
    rows: u16,
    token_width_cells: u16,
    token_height_cells: u16,
) -> (f32, f32) {
    (
        clamp_token_axis(f64::from(x), columns, token_width_cells),
        clamp_token_axis(f64::from(y), rows, token_height_cells),
    )
}

pub fn centered_token_offset(cell_size: f64, width_cells: u16, height_cells: u16) -> (f64, f64) {
    (
        f64::from(clamp_token_dimension(width_cells)) * cell_size / 2.0,
        f64::from(clamp_token_dimension(height_cells)) * cell_size / 2.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_world_and_back_roundtrip() {
        let (vw, vh) = (1280.0, 720.0);
        let (cx, cy, zoom) = (50.0, -30.0, 1.5);
        let world = (100.0, -200.0);

        let screen = world_to_screen(world.0, world.1, vw, vh, cx, cy, zoom);
        let back = screen_to_world(screen.0, screen.1, vw, vh, cx, cy, zoom);

        assert!((back.0 - world.0).abs() < 1e-9);
        assert!((back.1 - world.1).abs() < 1e-9);
    }

    #[test]
    fn clamp_zoom_enforces_bounds() {
        assert_eq!(clamp_zoom(0.1), MIN_ZOOM);
        assert_eq!(clamp_zoom(99.0), MAX_ZOOM);
        assert_eq!(clamp_zoom(1.0), 1.0);
    }

    #[test]
    fn selection_box_normalizes_corners() {
        let (l, t, w, h) = selection_box(50.0, 80.0, 10.0, 20.0);
        assert_eq!(l, 10.0);
        assert_eq!(t, 20.0);
        assert_eq!(w, 40.0);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn point_inside_rect_basic() {
        assert!(point_inside_rect(5.0, 5.0, 0.0, 0.0, 10.0, 10.0));
        assert!(!point_inside_rect(15.0, 5.0, 0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn workspace_board_metrics_are_viewport_independent() {
        let a = workspace_board_metrics(24, 16);
        let b = workspace_board_metrics(24, 16);
        assert_eq!(a, b);
        assert_eq!(a.0, WORKSPACE_SCENE_CELL_SIZE_PX);
    }

    #[test]
    fn grid_line_width_thresholds() {
        assert_eq!(grid_line_width_screen(50.0), 1.35);
        assert_eq!(grid_line_width_screen(30.0), 1.15);
        assert_eq!(grid_line_width_screen(10.0), 1.0);
    }

    #[test]
    fn inactive_scene_contents_are_hidden_by_default() {
        assert!(!scene_shows_contents("scene-b", Some("scene-a"), false));
    }

    #[test]
    fn active_scene_contents_are_always_visible() {
        assert!(scene_shows_contents("scene-a", Some("scene-a"), false));
    }

    #[test]
    fn inactive_scene_contents_can_be_revealed_by_setting() {
        assert!(scene_shows_contents("scene-b", Some("scene-a"), true));
    }

    #[test]
    fn token_interaction_is_blocked_on_hidden_inactive_scene() {
        assert!(!scene_allows_token_interaction(
            "scene-b",
            Some("scene-a"),
            false
        ));
    }

    #[test]
    fn token_interaction_is_allowed_when_inactive_contents_are_revealed() {
        assert!(scene_allows_token_interaction(
            "scene-b",
            Some("scene-a"),
            true
        ));
    }

    #[test]
    fn cursor_broadcast_stays_enabled_outside_any_scene() {
        assert!(should_broadcast_cursor(None, Some("scene-a"), true));
    }

    #[test]
    fn cursor_broadcast_stays_enabled_on_active_scene() {
        assert!(should_broadcast_cursor(
            Some("scene-a"),
            Some("scene-a"),
            true
        ));
    }

    #[test]
    fn cursor_broadcast_is_disabled_on_inactive_scene_when_revealed() {
        assert!(!should_broadcast_cursor(
            Some("scene-b"),
            Some("scene-a"),
            true
        ));
    }

    #[test]
    fn cursor_broadcast_is_unchanged_without_reveal_setting() {
        assert!(should_broadcast_cursor(
            Some("scene-b"),
            Some("scene-a"),
            false
        ));
    }

    #[test]
    fn token_rect_scales_with_cell_size() {
        let rect = token_rect(100.0, 200.0, 40.0, 2.0, 3.0, 2, 3);
        assert_eq!(rect, (180.0, 320.0, 80.0, 120.0));
    }

    #[test]
    fn token_position_from_world_clamps_to_board_bounds() {
        let (x, y) =
            token_position_from_world(999.0, 999.0, 100.0, 100.0, 40.0, 10, 10, 2, 3, 10.0, 10.0);
        assert_eq!((x, y), (8.0, 7.0));
    }

    #[test]
    fn token_position_from_world_keeps_fractional_cells() {
        let (x, y) =
            token_position_from_world(170.0, 198.0, 100.0, 100.0, 40.0, 10, 10, 1, 1, 10.0, 10.0);
        assert_eq!((x, y), (1.5, 2.2));
    }

    #[test]
    fn snap_token_position_to_grid_rounds_fractional_cells() {
        let (x, y) = snap_token_position_to_grid(2.49, 3.51, 10, 10, 1, 2);
        assert_eq!((x, y), (2.0, 4.0));
    }

    #[test]
    fn clamp_token_position_preserves_fractional_offset() {
        let (x, y) = clamp_token_position(8.75, 7.5, 10, 10, 2, 3);
        assert_eq!((x, y), (8.0, 7.0));
    }

    #[test]
    fn centered_token_offset_uses_token_dimensions() {
        assert_eq!(centered_token_offset(48.0, 2, 3), (48.0, 72.0));
    }
}
