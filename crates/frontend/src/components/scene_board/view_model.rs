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

    // Token drag state
    pub dragging_token_id: RwSignal<Option<String>>,
    token_drag_width_cells: RwSignal<u16>,
    token_drag_height_cells: RwSignal<u16>,
    token_drag_offset_x: RwSignal<f64>,
    token_drag_offset_y: RwSignal<f64>,
    token_drag_origin_x: RwSignal<f32>,
    token_drag_origin_y: RwSignal<f32>,

    pub pointer_local_x: RwSignal<f64>,
    pub pointer_local_y: RwSignal<f64>,

    // Selection box
    pub is_selecting: RwSignal<bool>,
    pub selection_start_x: RwSignal<f64>,
    pub selection_start_y: RwSignal<f64>,
    pub selection_end_x: RwSignal<f64>,
    pub selection_end_y: RwSignal<f64>,
}

impl SceneBoardViewModel {
    pub fn new(
        initial_vw: f64,
        initial_vh: f64,
        initial_camera_x: f64,
        initial_camera_y: f64,
        initial_zoom: f64,
    ) -> Self {
        Self {
            zoom: RwSignal::new(initial_zoom),
            camera_x: RwSignal::new(initial_camera_x),
            camera_y: RwSignal::new(initial_camera_y),
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
            dragging_token_id: RwSignal::new(None),
            token_drag_width_cells: RwSignal::new(1),
            token_drag_height_cells: RwSignal::new(1),
            token_drag_offset_x: RwSignal::new(0.0),
            token_drag_offset_y: RwSignal::new(0.0),
            token_drag_origin_x: RwSignal::new(0.0),
            token_drag_origin_y: RwSignal::new(0.0),
            pointer_local_x: RwSignal::new(0.0),
            pointer_local_y: RwSignal::new(0.0),
            is_selecting: RwSignal::new(false),
            selection_start_x: RwSignal::new(0.0),
            selection_start_y: RwSignal::new(0.0),
            selection_end_x: RwSignal::new(0.0),
            selection_end_y: RwSignal::new(0.0),
        }
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

    pub fn set_view_transform(&self, x: f64, y: f64, zoom: f64) {
        self.camera_x.set(x);
        self.camera_y.set(y);
        self.zoom.set(zoom);
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

    pub fn start_token_drag(
        &self,
        token_id: String,
        token_width_cells: u16,
        token_height_cells: u16,
        offset_x: f64,
        offset_y: f64,
        origin_x: f32,
        origin_y: f32,
    ) {
        self.dragging_token_id.set(Some(token_id));
        self.token_drag_width_cells.set(token_width_cells);
        self.token_drag_height_cells.set(token_height_cells);
        self.token_drag_offset_x.set(offset_x);
        self.token_drag_offset_y.set(offset_y);
        self.token_drag_origin_x.set(origin_x);
        self.token_drag_origin_y.set(origin_y);
    }

    pub fn end_token_drag(&self) {
        self.dragging_token_id.set(None);
    }

    pub fn token_drag_width_cells(&self) -> u16 {
        self.token_drag_width_cells.get_untracked()
    }

    pub fn token_drag_height_cells(&self) -> u16 {
        self.token_drag_height_cells.get_untracked()
    }

    pub fn token_drag_offset_x(&self) -> f64 {
        self.token_drag_offset_x.get_untracked()
    }

    pub fn token_drag_offset_y(&self) -> f64 {
        self.token_drag_offset_y.get_untracked()
    }

    pub fn token_drag_origin_x(&self) -> f32 {
        self.token_drag_origin_x.get_untracked()
    }

    pub fn token_drag_origin_y(&self) -> f32 {
        self.token_drag_origin_y.get_untracked()
    }

    pub fn update_pointer(&self, local_x: f64, local_y: f64) {
        self.pointer_local_x.set(local_x);
        self.pointer_local_y.set(local_y);
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
        SceneBoardViewModel::new(1280.0, 720.0, 0.0, 0.0, 1.0)
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
    fn constructor_uses_initial_camera_position() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = SceneBoardViewModel::new(1280.0, 720.0, 42.0, -13.5, 1.75);
            assert_eq!(vm.camera_x.get_untracked(), 42.0);
            assert_eq!(vm.camera_y.get_untracked(), -13.5);
            assert_eq!(vm.zoom.get_untracked(), 1.75);
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

    #[test]
    fn token_drag_stores_offsets_and_size() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.start_token_drag("token-1".into(), 2, 3, 14.0, 18.0, 3.0, 4.0);
            assert_eq!(vm.dragging_token_id.get_untracked(), Some("token-1".into()));
            assert_eq!(vm.token_drag_width_cells(), 2);
            assert_eq!(vm.token_drag_height_cells(), 3);
            assert_eq!(vm.token_drag_offset_x(), 14.0);
            assert_eq!(vm.token_drag_offset_y(), 18.0);
            assert_eq!(vm.token_drag_origin_x(), 3.0);
            assert_eq!(vm.token_drag_origin_y(), 4.0);
        });
    }
}
