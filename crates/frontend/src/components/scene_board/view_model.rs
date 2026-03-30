use super::model::step_zoom;
use leptos::prelude::*;

/// Reactive state for the scene board's camera, panning, and drag interactions.
#[derive(Clone, Copy)]
pub struct SceneBoardViewModel {
    // Camera
    pub zoom: RwSignal<f64>,
    pub camera_x: RwSignal<f64>,
    pub camera_y: RwSignal<f64>,
    pub viewport_width: RwSignal<f64>,
    pub viewport_height: RwSignal<f64>,

    // Pan state
    pub is_panning: RwSignal<bool>,
    pub is_space_pressed: RwSignal<bool>,
    pan_start_local_x: RwSignal<f64>,
    pan_start_local_y: RwSignal<f64>,
    pan_origin_camera_x: RwSignal<f64>,
    pan_origin_camera_y: RwSignal<f64>,

    // Scene drag state
    pub dragging_scene_id: RwSignal<Option<String>>,
    drag_start_world_x: RwSignal<f64>,
    drag_start_world_y: RwSignal<f64>,
    drag_origin_scene_x: RwSignal<f64>,
    drag_origin_scene_y: RwSignal<f64>,

    // Selection box
    pub is_selecting: RwSignal<bool>,
    pub selection_start_x: RwSignal<f64>,
    pub selection_start_y: RwSignal<f64>,
    pub selection_end_x: RwSignal<f64>,
    pub selection_end_y: RwSignal<f64>,
}

impl SceneBoardViewModel {
    pub fn new(initial_vw: f64, initial_vh: f64) -> Self {
        Self {
            zoom: RwSignal::new(1.0),
            camera_x: RwSignal::new(0.0),
            camera_y: RwSignal::new(0.0),
            viewport_width: RwSignal::new(initial_vw),
            viewport_height: RwSignal::new(initial_vh),
            is_panning: RwSignal::new(false),
            is_space_pressed: RwSignal::new(false),
            pan_start_local_x: RwSignal::new(0.0),
            pan_start_local_y: RwSignal::new(0.0),
            pan_origin_camera_x: RwSignal::new(0.0),
            pan_origin_camera_y: RwSignal::new(0.0),
            dragging_scene_id: RwSignal::new(None),
            drag_start_world_x: RwSignal::new(0.0),
            drag_start_world_y: RwSignal::new(0.0),
            drag_origin_scene_x: RwSignal::new(0.0),
            drag_origin_scene_y: RwSignal::new(0.0),
            is_selecting: RwSignal::new(false),
            selection_start_x: RwSignal::new(0.0),
            selection_start_y: RwSignal::new(0.0),
            selection_end_x: RwSignal::new(0.0),
            selection_end_y: RwSignal::new(0.0),
        }
    }

    pub fn handle_wheel(&self, delta_y: f64) {
        let new_zoom = step_zoom(self.zoom.get_untracked(), delta_y);
        self.zoom.set(new_zoom);
    }

    pub fn start_pan(&self, local_x: f64, local_y: f64) {
        self.is_panning.set(true);
        self.pan_start_local_x.set(local_x);
        self.pan_start_local_y.set(local_y);
        self.pan_origin_camera_x.set(self.camera_x.get_untracked());
        self.pan_origin_camera_y.set(self.camera_y.get_untracked());
    }

    pub fn update_pan(&self, local_x: f64, local_y: f64) {
        if !self.is_panning.get_untracked() {
            return;
        }
        let dx = local_x - self.pan_start_local_x.get_untracked();
        let dy = local_y - self.pan_start_local_y.get_untracked();
        self.camera_x
            .set(self.pan_origin_camera_x.get_untracked() + dx);
        self.camera_y
            .set(self.pan_origin_camera_y.get_untracked() + dy);
    }

    pub fn end_pan(&self) {
        self.is_panning.set(false);
    }

    pub fn start_scene_drag(
        &self,
        scene_id: String,
        world_x: f64,
        world_y: f64,
        scene_center_x: f64,
        scene_center_y: f64,
    ) {
        self.dragging_scene_id.set(Some(scene_id));
        self.drag_start_world_x.set(world_x);
        self.drag_start_world_y.set(world_y);
        self.drag_origin_scene_x.set(scene_center_x);
        self.drag_origin_scene_y.set(scene_center_y);
    }

    /// Returns the new scene center position based on current world cursor position.
    pub fn compute_scene_drag_position(
        &self,
        current_world_x: f64,
        current_world_y: f64,
    ) -> Option<(String, f64, f64)> {
        let id = self.dragging_scene_id.get_untracked()?;
        let dx = current_world_x - self.drag_start_world_x.get_untracked();
        let dy = current_world_y - self.drag_start_world_y.get_untracked();
        Some((
            id,
            self.drag_origin_scene_x.get_untracked() + dx,
            self.drag_origin_scene_y.get_untracked() + dy,
        ))
    }

    pub fn end_scene_drag(&self) {
        self.dragging_scene_id.set(None);
    }

    pub fn drag_start_world_x(&self) -> f64 {
        self.drag_start_world_x.get_untracked()
    }

    pub fn drag_start_world_y(&self) -> f64 {
        self.drag_start_world_y.get_untracked()
    }

    pub fn drag_origin_scene_x(&self) -> f64 {
        self.drag_origin_scene_x.get_untracked()
    }

    pub fn drag_origin_scene_y(&self) -> f64 {
        self.drag_origin_scene_y.get_untracked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    fn make_vm() -> SceneBoardViewModel {
        SceneBoardViewModel::new(1280.0, 720.0)
    }

    #[test]
    fn initial_zoom_is_one() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            assert_eq!(vm.zoom.get_untracked(), 1.0);
        });
    }

    #[test]
    fn handle_wheel_positive_delta_decreases_zoom() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.handle_wheel(1.0);
            assert!(vm.zoom.get_untracked() < 1.0);
        });
    }

    #[test]
    fn handle_wheel_negative_delta_increases_zoom() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.handle_wheel(-1.0);
            assert!(vm.zoom.get_untracked() > 1.0);
        });
    }

    #[test]
    fn pan_moves_camera() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_pan(100.0, 200.0);
            vm.update_pan(150.0, 250.0);
            assert_eq!(vm.camera_x.get_untracked(), 50.0);
            assert_eq!(vm.camera_y.get_untracked(), 50.0);
        });
    }

    #[test]
    fn end_pan_clears_panning_flag() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_pan(0.0, 0.0);
            vm.end_pan();
            assert!(!vm.is_panning.get_untracked());
        });
    }

    #[test]
    fn scene_drag_computes_delta() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_scene_drag("s1".into(), 100.0, 100.0, 50.0, 50.0);
            let result = vm.compute_scene_drag_position(120.0, 110.0);
            assert!(result.is_some());
            let (id, nx, ny) = result.unwrap();
            assert_eq!(id, "s1");
            assert_eq!(nx, 70.0); // 50 + (120-100)
            assert_eq!(ny, 60.0); // 50 + (110-100)
        });
    }

    #[test]
    fn compute_scene_drag_returns_none_when_not_dragging() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            assert!(vm.compute_scene_drag_position(0.0, 0.0).is_none());
        });
    }
}
