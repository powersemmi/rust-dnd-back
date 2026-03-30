/// Pure geometric types and constants for the scene board.
/// No signals, no Leptos, no web_sys.

// --- Constants ---

pub const BOARD_SIDE_PADDING_PX: f64 = 220.0;
pub const BOARD_TOP_PADDING_PX: f64 = 180.0;
pub const BOARD_BOTTOM_PADDING_PX: f64 = 140.0;
pub const MAX_CELL_SIZE_PX: f64 = 72.0;
pub const MIN_CELL_SIZE_PX: f64 = 18.0;
pub const MIN_ZOOM: f64 = 0.35;
pub const MAX_ZOOM: f64 = 2.5;
pub const ZOOM_STEP: f64 = 0.12;
pub const WORKSPACE_GRID_STEP_PX: f64 = 48.0;
pub const SNAP_THRESHOLD_PX: f64 = 56.0;
pub const BOARD_HANDLE_HEIGHT_PX: f64 = 42.0;
pub const BOARD_HANDLE_GAP_PX: f64 = 14.0;
pub const BOARD_HANDLE_MAX_WIDTH_PX: f64 = 320.0;
pub const DRAG_EPSILON_PX: f64 = 1.0;

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

/// Compute the next zoom level based on wheel delta direction.
pub fn step_zoom(current: f64, delta_y: f64) -> f64 {
    clamp_zoom(current - delta_y.signum() * ZOOM_STEP)
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

// --- Board metrics ---

/// Computes (cell_size, board_width, board_height) for a given scene and viewport size.
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
    let rws = f64::from(rows.max(1));

    let cell_size = (usable_width / cols)
        .min(usable_height / rws)
        .clamp(MIN_CELL_SIZE_PX, MAX_CELL_SIZE_PX);

    (cell_size, cols * cell_size, rws * cell_size)
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
    fn step_zoom_increases_on_negative_delta() {
        let next = step_zoom(1.0, -1.0); // scroll up = zoom in
        assert!(next > 1.0);
    }

    #[test]
    fn step_zoom_decreases_on_positive_delta() {
        let next = step_zoom(1.0, 1.0); // scroll down = zoom out
        assert!(next < 1.0);
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
    fn board_metrics_clamps_cell_size() {
        let (cell, _, _) = board_metrics(1, 1, 320.0, 240.0);
        assert!(cell >= MIN_CELL_SIZE_PX);
        assert!(cell <= MAX_CELL_SIZE_PX);
    }

    #[test]
    fn grid_line_width_thresholds() {
        assert_eq!(grid_line_width_screen(50.0), 1.35);
        assert_eq!(grid_line_width_screen(30.0), 1.15);
        assert_eq!(grid_line_width_screen(10.0), 1.0);
    }
}
