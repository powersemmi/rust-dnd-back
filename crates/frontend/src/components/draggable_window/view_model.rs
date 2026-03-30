use super::model::{WindowPosition, WindowSize, apply_drag, apply_resize};
use leptos::prelude::*;

/// Reactive state for a draggable, resizable floating window.
#[derive(Clone, Copy)]
pub struct DraggableWindowViewModel {
    pub pos_x: RwSignal<i32>,
    pub pos_y: RwSignal<i32>,
    pub width: RwSignal<i32>,
    pub height: RwSignal<i32>,

    // Drag state
    pub is_dragging: RwSignal<bool>,
    drag_start_x: RwSignal<i32>,
    drag_start_y: RwSignal<i32>,
    window_start_x: RwSignal<i32>,
    window_start_y: RwSignal<i32>,

    // Resize state
    pub is_resizing: RwSignal<bool>,
    resize_start_x: RwSignal<i32>,
    resize_start_y: RwSignal<i32>,
    size_start_w: RwSignal<i32>,
    size_start_h: RwSignal<i32>,

    min_width: i32,
    min_height: i32,
}

impl DraggableWindowViewModel {
    pub fn new(
        initial_x: i32,
        initial_y: i32,
        initial_width: i32,
        initial_height: i32,
        min_width: i32,
        min_height: i32,
    ) -> Self {
        Self {
            pos_x: RwSignal::new(initial_x),
            pos_y: RwSignal::new(initial_y),
            width: RwSignal::new(initial_width),
            height: RwSignal::new(initial_height),
            is_dragging: RwSignal::new(false),
            drag_start_x: RwSignal::new(0),
            drag_start_y: RwSignal::new(0),
            window_start_x: RwSignal::new(0),
            window_start_y: RwSignal::new(0),
            is_resizing: RwSignal::new(false),
            resize_start_x: RwSignal::new(0),
            resize_start_y: RwSignal::new(0),
            size_start_w: RwSignal::new(0),
            size_start_h: RwSignal::new(0),
            min_width,
            min_height,
        }
    }

    pub fn start_drag(&self, client_x: i32, client_y: i32) {
        self.is_dragging.set(true);
        self.drag_start_x.set(client_x);
        self.drag_start_y.set(client_y);
        self.window_start_x.set(self.pos_x.get_untracked());
        self.window_start_y.set(self.pos_y.get_untracked());
    }

    pub fn update_drag(&self, client_x: i32, client_y: i32) {
        if !self.is_dragging.get_untracked() {
            return;
        }
        let start = WindowPosition {
            x: self.window_start_x.get_untracked(),
            y: self.window_start_y.get_untracked(),
        };
        let new_pos = apply_drag(
            start,
            self.drag_start_x.get_untracked(),
            self.drag_start_y.get_untracked(),
            client_x,
            client_y,
        );
        self.pos_x.set(new_pos.x);
        self.pos_y.set(new_pos.y);
    }

    pub fn start_resize(&self, client_x: i32, client_y: i32) {
        self.is_resizing.set(true);
        self.resize_start_x.set(client_x);
        self.resize_start_y.set(client_y);
        self.size_start_w.set(self.width.get_untracked());
        self.size_start_h.set(self.height.get_untracked());
    }

    pub fn update_resize(&self, client_x: i32, client_y: i32) {
        if !self.is_resizing.get_untracked() {
            return;
        }
        let start = WindowSize {
            width: self.size_start_w.get_untracked(),
            height: self.size_start_h.get_untracked(),
        };
        let new_size = apply_resize(
            start,
            self.resize_start_x.get_untracked(),
            self.resize_start_y.get_untracked(),
            client_x,
            client_y,
            self.min_width,
            self.min_height,
        );
        self.width.set(new_size.width);
        self.height.set(new_size.height);
    }

    pub fn end_interaction(&self) {
        self.is_dragging.set(false);
        self.is_resizing.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    fn make_vm() -> DraggableWindowViewModel {
        DraggableWindowViewModel::new(100, 200, 400, 300, 200, 150)
    }

    #[test]
    fn start_drag_records_origin() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_drag(300, 400);
            assert!(vm.is_dragging.get_untracked());
            assert_eq!(vm.pos_x.get_untracked(), 100);
        });
    }

    #[test]
    fn update_drag_moves_position() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_drag(300, 400);
            vm.update_drag(350, 450); // +50, +50
            assert_eq!(vm.pos_x.get_untracked(), 150);
            assert_eq!(vm.pos_y.get_untracked(), 250);
        });
    }

    #[test]
    fn end_interaction_clears_drag_and_resize() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_drag(0, 0);
            vm.end_interaction();
            assert!(!vm.is_dragging.get_untracked());
        });
    }

    #[test]
    fn update_resize_clamps_to_minimum() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_resize(800, 600);
            vm.update_resize(100, 100); // huge shrink
            assert_eq!(vm.width.get_untracked(), 200); // min_width
            assert_eq!(vm.height.get_untracked(), 150); // min_height
        });
    }

    #[test]
    fn update_drag_does_nothing_when_not_dragging() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.update_drag(999, 999);
            assert_eq!(vm.pos_x.get_untracked(), 100); // unchanged
        });
    }
}
