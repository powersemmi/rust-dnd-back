/// Pure geometric calculations for drag and resize operations.
/// No signals, no Leptos imports - fully testable without a reactive runtime.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowSize {
    pub width: i32,
    pub height: i32,
}

impl WindowSize {
    #[allow(dead_code)] // used in tests only
    pub fn clamp(&self, min_width: i32, min_height: i32) -> Self {
        Self {
            width: self.width.max(min_width),
            height: self.height.max(min_height),
        }
    }
}

/// Computes new window position after a drag delta.
pub fn apply_drag(
    start_pos: WindowPosition,
    drag_origin_x: i32,
    drag_origin_y: i32,
    current_x: i32,
    current_y: i32,
) -> WindowPosition {
    WindowPosition {
        x: start_pos.x + (current_x - drag_origin_x),
        y: start_pos.y + (current_y - drag_origin_y),
    }
}

/// Computes new window size after a resize delta, clamped to minimum dimensions.
pub fn apply_resize(
    start_size: WindowSize,
    resize_origin_x: i32,
    resize_origin_y: i32,
    current_x: i32,
    current_y: i32,
    min_width: i32,
    min_height: i32,
) -> WindowSize {
    WindowSize {
        width: (start_size.width + (current_x - resize_origin_x)).max(min_width),
        height: (start_size.height + (current_y - resize_origin_y)).max(min_height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_drag_moves_window_by_delta() {
        let start = WindowPosition { x: 100, y: 200 };
        let result = apply_drag(start, 50, 60, 80, 100);
        assert_eq!(result, WindowPosition { x: 130, y: 240 });
    }

    #[test]
    fn apply_drag_negative_delta_moves_up_left() {
        let start = WindowPosition { x: 100, y: 100 };
        let result = apply_drag(start, 100, 100, 80, 70);
        assert_eq!(result, WindowPosition { x: 80, y: 70 });
    }

    #[test]
    fn apply_resize_grows_window() {
        let start = WindowSize {
            width: 400,
            height: 300,
        };
        let result = apply_resize(start, 400, 300, 500, 450, 100, 100);
        assert_eq!(
            result,
            WindowSize {
                width: 500,
                height: 450
            }
        );
    }

    #[test]
    fn apply_resize_respects_min_width() {
        let start = WindowSize {
            width: 400,
            height: 300,
        };
        let result = apply_resize(start, 400, 300, 100, 300, 300, 200);
        assert_eq!(result.width, 300); // clamped to min_width
    }

    #[test]
    fn apply_resize_respects_min_height() {
        let start = WindowSize {
            width: 400,
            height: 300,
        };
        let result = apply_resize(start, 400, 300, 400, 100, 300, 200);
        assert_eq!(result.height, 200); // clamped to min_height
    }

    #[test]
    fn window_size_clamp_already_within_bounds() {
        let size = WindowSize {
            width: 500,
            height: 400,
        };
        assert_eq!(size.clamp(300, 200), size);
    }
}
