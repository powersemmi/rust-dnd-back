use crate::components::app::mouse_handler::{
    send_mouse_event_throttled, update_local_cursor_world,
};
use crate::components::cursor::Cursor;
use crate::components::websocket::{CursorSignals, FileTransferState, WsSender};
use crate::config;
use crate::config::Theme;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use shared::events::{ClientEvent, Scene, SceneUpdatePayload};
use web_sys::{MouseEvent, WheelEvent};

const BOARD_SIDE_PADDING_PX: f64 = 220.0;
const BOARD_TOP_PADDING_PX: f64 = 180.0;
const BOARD_BOTTOM_PADDING_PX: f64 = 140.0;
const MAX_CELL_SIZE_PX: f64 = 72.0;
const MIN_CELL_SIZE_PX: f64 = 18.0;
const MIN_ZOOM: f64 = 0.35;
const MAX_ZOOM: f64 = 2.5;
const ZOOM_STEP: f64 = 0.12;
const WORKSPACE_GRID_STEP_PX: f64 = 48.0;
const SNAP_THRESHOLD_PX: f64 = 56.0;
const BOARD_HANDLE_HEIGHT_PX: f64 = 42.0;
const BOARD_HANDLE_GAP_PX: f64 = 14.0;
const BOARD_HANDLE_MAX_WIDTH_PX: f64 = 320.0;
const DRAG_EPSILON_PX: f64 = 1.0;

#[derive(Clone)]
struct SceneLayout {
    scene: Scene,
    cell_size: f64,
    board_width: f64,
    board_height: f64,
}

impl SceneLayout {
    fn center_x(&self) -> f64 {
        f64::from(self.scene.workspace_x)
    }

    fn center_y(&self) -> f64 {
        f64::from(self.scene.workspace_y)
    }

    fn left(&self) -> f64 {
        self.center_x() - self.board_width / 2.0
    }

    fn right(&self) -> f64 {
        self.center_x() + self.board_width / 2.0
    }

    fn top(&self) -> f64 {
        self.center_y() - self.board_height / 2.0
    }

    fn bottom(&self) -> f64 {
        self.center_y() + self.board_height / 2.0
    }

    fn handle_width(&self) -> f64 {
        self.board_width.min(BOARD_HANDLE_MAX_WIDTH_PX)
    }

    fn handle_left(&self) -> f64 {
        self.center_x() - self.handle_width() / 2.0
    }

    fn handle_top(&self) -> f64 {
        self.top() - BOARD_HANDLE_GAP_PX - BOARD_HANDLE_HEIGHT_PX
    }
}

fn viewport_size() -> (f64, f64) {
    let Some(window) = web_sys::window() else {
        return (1280.0, 720.0);
    };

    let width = window
        .inner_width()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(1280.0);
    let height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(720.0);

    (width, height)
}

fn board_metrics(scene: &Scene, viewport_width: f64, viewport_height: f64) -> (f64, f64, f64) {
    let usable_width = (viewport_width - BOARD_SIDE_PADDING_PX).max(320.0);
    let usable_height =
        (viewport_height - BOARD_TOP_PADDING_PX - BOARD_BOTTOM_PADDING_PX).max(240.0);

    let columns = f64::from(scene.grid.columns.max(1));
    let rows = f64::from(scene.grid.rows.max(1));

    let cell_size = (usable_width / columns)
        .min(usable_height / rows)
        .clamp(MIN_CELL_SIZE_PX, MAX_CELL_SIZE_PX);

    let board_width = columns * cell_size;
    let board_height = rows * cell_size;

    (cell_size, board_width, board_height)
}

fn build_scene_layouts(
    scenes: &[Scene],
    viewport_width: f64,
    viewport_height: f64,
) -> Vec<SceneLayout> {
    scenes
        .iter()
        .cloned()
        .map(|scene| {
            let (cell_size, board_width, board_height) =
                board_metrics(&scene, viewport_width, viewport_height);
            SceneLayout {
                scene,
                cell_size,
                board_width,
                board_height,
            }
        })
        .collect()
}

fn viewport_local_point(
    viewport_ref: &NodeRef<html::Div>,
    client_x: i32,
    client_y: i32,
) -> Option<(f64, f64)> {
    let viewport = viewport_ref.get()?;
    let rect = viewport.get_bounding_client_rect();
    let x = (f64::from(client_x) - rect.left()).clamp(0.0, rect.width());
    let y = (f64::from(client_y) - rect.top()).clamp(0.0, rect.height());
    Some((x, y))
}

fn screen_to_world(
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

fn world_to_screen(
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

fn point_inside_rect(
    point_x: f64,
    point_y: f64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
) -> bool {
    point_x >= left && point_x <= left + width && point_y >= top && point_y <= top + height
}

fn point_inside_board(layout: &SceneLayout, world_x: f64, world_y: f64) -> bool {
    point_inside_rect(
        world_x,
        world_y,
        layout.left(),
        layout.top(),
        layout.board_width,
        layout.board_height,
    )
}

fn point_inside_handle(layout: &SceneLayout, world_x: f64, world_y: f64) -> bool {
    point_inside_rect(
        world_x,
        world_y,
        layout.handle_left(),
        layout.handle_top(),
        layout.handle_width(),
        BOARD_HANDLE_HEIGHT_PX,
    )
}

fn clamp_to_layout(world_x: f64, world_y: f64, layout: &SceneLayout) -> (f64, f64) {
    (
        world_x.clamp(layout.left(), layout.right()),
        world_y.clamp(layout.top(), layout.bottom()),
    )
}

fn selection_box(start_x: f64, start_y: f64, end_x: f64, end_y: f64) -> (f64, f64, f64, f64) {
    let left = start_x.min(end_x);
    let top = start_y.min(end_y);
    let width = (start_x - end_x).abs();
    let height = (start_y - end_y).abs();
    (left, top, width, height)
}

fn board_background(theme_bg: &str) -> String {
    format!(
        "linear-gradient(180deg, rgba(255,255,255,0.06), rgba(0,0,0,0.12)), \
        radial-gradient(circle at top left, rgba(255,255,255,0.08), transparent 30%), \
        {theme_bg}"
    )
}

fn grid_line_width_screen(screen_cell: f64) -> f64 {
    if screen_cell >= 42.0 {
        1.35
    } else if screen_cell >= 20.0 {
        1.15
    } else {
        1.0
    }
}

fn snap_scene_position(
    candidate_x: f64,
    candidate_y: f64,
    dragged_width: f64,
    dragged_height: f64,
    other_layouts: &[SceneLayout],
) -> (f64, f64) {
    let mut best_position = (candidate_x, candidate_y);
    let mut best_score = f64::MAX;

    for layout in other_layouts {
        let horizontal_targets = [
            layout.center_y(),
            layout.top() + dragged_height / 2.0,
            layout.bottom() - dragged_height / 2.0,
        ];
        let vertical_targets = [
            layout.center_x(),
            layout.left() + dragged_width / 2.0,
            layout.right() - dragged_width / 2.0,
        ];

        let neighbor_targets = [
            (layout.right() + dragged_width / 2.0, horizontal_targets[0]),
            (layout.right() + dragged_width / 2.0, horizontal_targets[1]),
            (layout.right() + dragged_width / 2.0, horizontal_targets[2]),
            (layout.left() - dragged_width / 2.0, horizontal_targets[0]),
            (layout.left() - dragged_width / 2.0, horizontal_targets[1]),
            (layout.left() - dragged_width / 2.0, horizontal_targets[2]),
            (vertical_targets[0], layout.bottom() + dragged_height / 2.0),
            (vertical_targets[1], layout.bottom() + dragged_height / 2.0),
            (vertical_targets[2], layout.bottom() + dragged_height / 2.0),
            (vertical_targets[0], layout.top() - dragged_height / 2.0),
            (vertical_targets[1], layout.top() - dragged_height / 2.0),
            (vertical_targets[2], layout.top() - dragged_height / 2.0),
        ];

        for (target_x, target_y) in neighbor_targets {
            let dx = (candidate_x - target_x).abs();
            let dy = (candidate_y - target_y).abs();
            if dx <= SNAP_THRESHOLD_PX && dy <= SNAP_THRESHOLD_PX {
                let score = dx + dy;
                if score < best_score {
                    best_score = score;
                    best_position = (target_x, target_y);
                }
            }
        }
    }

    best_position
}

fn update_scene_position(scenes: RwSignal<Vec<Scene>>, scene_id: &str, x: f64, y: f64) {
    scenes.update(|items| {
        if let Some(scene) = items.iter_mut().find(|scene| scene.id == scene_id) {
            scene.workspace_x = x as f32;
            scene.workspace_y = y as f32;
        }
    });
}

fn send_event(ws_sender: &ReadSignal<Option<WsSender>>, event: ClientEvent) {
    if let Some(sender) = ws_sender.get_untracked() {
        let _ = sender.try_send_event(event);
    }
}

#[component]
pub fn SceneBoard(
    #[prop(into)] scenes: RwSignal<Vec<Scene>>,
    #[prop(into)] active_scene_id: RwSignal<Option<String>>,
    cursors: ReadSignal<std::collections::HashMap<String, CursorSignals>>,
    set_cursors: WriteSignal<std::collections::HashMap<String, CursorSignals>>,
    file_transfer: FileTransferState,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    config: config::Config,
    theme: Theme,
) -> impl IntoView {
    let (initial_width, initial_height) = viewport_size();
    let viewport_width = RwSignal::new(initial_width);
    let viewport_height = RwSignal::new(initial_height);
    let viewport_ref = NodeRef::<html::Div>::new();

    let camera_x = RwSignal::new(0.0f64);
    let camera_y = RwSignal::new(0.0f64);
    let zoom = RwSignal::new(1.0f64);
    let config = StoredValue::new(config);

    let is_panning = RwSignal::new(false);
    let pan_start_client_x = RwSignal::new(0i32);
    let pan_start_client_y = RwSignal::new(0i32);
    let pan_origin_x = RwSignal::new(0.0f64);
    let pan_origin_y = RwSignal::new(0.0f64);

    let dragging_scene_id = RwSignal::new(None::<String>);
    let drag_pointer_start_world_x = RwSignal::new(0.0f64);
    let drag_pointer_start_world_y = RwSignal::new(0.0f64);
    let drag_scene_origin_x = RwSignal::new(0.0f64);
    let drag_scene_origin_y = RwSignal::new(0.0f64);
    let drag_did_move = RwSignal::new(false);

    let is_selecting = RwSignal::new(false);
    let selection_start_x = RwSignal::new(0.0f64);
    let selection_start_y = RwSignal::new(0.0f64);
    let selection_end_x = RwSignal::new(0.0f64);
    let selection_end_y = RwSignal::new(0.0f64);

    Effect::new(move |_| {
        let resize_handle = window_event_listener(ev::resize, move |_| {
            let (width, height) = viewport_size();
            viewport_width.set(width);
            viewport_height.set(height);
        });

        let mouse_move_handle = window_event_listener(ev::mousemove, move |event: MouseEvent| {
            let mut next_camera_x = camera_x.get();
            let mut next_camera_y = camera_y.get();

            if is_panning.get() {
                let dx = event.client_x() - pan_start_client_x.get();
                let dy = event.client_y() - pan_start_client_y.get();
                next_camera_x = pan_origin_x.get() + f64::from(dx);
                next_camera_y = pan_origin_y.get() + f64::from(dy);
                camera_x.set(next_camera_x);
                camera_y.set(next_camera_y);
            }

            let Some((local_x, local_y)) =
                viewport_local_point(&viewport_ref, event.client_x(), event.client_y())
            else {
                return;
            };

            let (world_x, world_y) = screen_to_world(
                local_x,
                local_y,
                viewport_width.get(),
                viewport_height.get(),
                next_camera_x,
                next_camera_y,
                zoom.get(),
            );

            let current_user = username.get_untracked();
            update_local_cursor_world(&current_user, world_x, world_y, set_cursors);
            send_mouse_event_throttled(world_x, world_y, current_user, ws_sender, config);

            if let Some(scene_id) = dragging_scene_id.get() {
                let candidate_x =
                    drag_scene_origin_x.get() + (world_x - drag_pointer_start_world_x.get());
                let candidate_y =
                    drag_scene_origin_y.get() + (world_y - drag_pointer_start_world_y.get());
                let layouts = build_scene_layouts(
                    &scenes.get_untracked(),
                    viewport_width.get_untracked(),
                    viewport_height.get_untracked(),
                );

                let Some(dragged_layout) = layouts
                    .iter()
                    .find(|layout| layout.scene.id == scene_id)
                    .cloned()
                else {
                    return;
                };

                let other_layouts: Vec<SceneLayout> = layouts
                    .into_iter()
                    .filter(|layout| layout.scene.id != scene_id)
                    .collect();
                let (snapped_x, snapped_y) = snap_scene_position(
                    candidate_x,
                    candidate_y,
                    dragged_layout.board_width,
                    dragged_layout.board_height,
                    &other_layouts,
                );

                update_scene_position(scenes, &scene_id, snapped_x, snapped_y);
                drag_did_move.set(
                    (snapped_x - drag_scene_origin_x.get()).abs() > DRAG_EPSILON_PX
                        || (snapped_y - drag_scene_origin_y.get()).abs() > DRAG_EPSILON_PX,
                );
                return;
            }

            if is_selecting.get() {
                let layouts = build_scene_layouts(
                    &scenes.get_untracked(),
                    viewport_width.get_untracked(),
                    viewport_height.get_untracked(),
                );
                let Some(active_layout) = layouts.iter().find(|layout| {
                    Some(layout.scene.id.as_str()) == active_scene_id.get_untracked().as_deref()
                }) else {
                    return;
                };

                let (clamped_x, clamped_y) = clamp_to_layout(world_x, world_y, active_layout);
                selection_end_x.set(clamped_x);
                selection_end_y.set(clamped_y);
            }
        });

        let mouse_up_handle = window_event_listener(ev::mouseup, move |_event: MouseEvent| {
            if let Some(scene_id) = dragging_scene_id.get_untracked()
                && drag_did_move.get_untracked()
                && let Some(scene) = scenes
                    .get_untracked()
                    .into_iter()
                    .find(|scene| scene.id == scene_id)
            {
                send_event(
                    &ws_sender,
                    ClientEvent::SceneUpdate(SceneUpdatePayload {
                        scene,
                        actor: username.get_untracked(),
                    }),
                );
            }

            dragging_scene_id.set(None);
            drag_did_move.set(false);
            is_panning.set(false);
            is_selecting.set(false);
        });

        on_cleanup(move || {
            drop(resize_handle);
            drop(mouse_move_handle);
            drop(mouse_up_handle);
        });
    });

    Effect::new(move |_| {
        let _ = active_scene_id.get();
        is_selecting.set(false);
    });

    view! {
        {move || {
            let active_id = active_scene_id.get();
            let scene_items = scenes.get();
            if scene_items.is_empty() {
                return ().into_any();
            }

            let mut layouts = build_scene_layouts(
                &scene_items,
                viewport_width.get(),
                viewport_height.get(),
            );
            layouts.sort_by_key(|layout| {
                let is_dragging = dragging_scene_id.get().as_deref() == Some(layout.scene.id.as_str());
                let is_active = active_id.as_deref() == Some(layout.scene.id.as_str());
                (is_dragging, is_active)
            });

            let workspace_minor_step = (WORKSPACE_GRID_STEP_PX * zoom.get()).max(18.0);
            let workspace_major_step = workspace_minor_step * 4.0;
            let minor_offset_x =
                (viewport_width.get() / 2.0 + camera_x.get()).rem_euclid(workspace_minor_step);
            let minor_offset_y =
                (viewport_height.get() / 2.0 + camera_y.get()).rem_euclid(workspace_minor_step);
            let major_offset_x =
                (viewport_width.get() / 2.0 + camera_x.get()).rem_euclid(workspace_major_step);
            let major_offset_y =
                (viewport_height.get() / 2.0 + camera_y.get()).rem_euclid(workspace_major_step);
            let world_transform = format!(
                "translate({:.2}px, {:.2}px) scale({:.4})",
                viewport_width.get() / 2.0 + camera_x.get(),
                viewport_height.get() / 2.0 + camera_y.get(),
                zoom.get()
            );
            let current_zoom = zoom.get();
            let cursor_theme = theme.clone();

            let selection_overlay = if is_selecting.get() {
                let (start_screen_x, start_screen_y) = world_to_screen(
                    selection_start_x.get(),
                    selection_start_y.get(),
                    viewport_width.get(),
                    viewport_height.get(),
                    camera_x.get(),
                    camera_y.get(),
                    zoom.get(),
                );
                let (end_screen_x, end_screen_y) = world_to_screen(
                    selection_end_x.get(),
                    selection_end_y.get(),
                    viewport_width.get(),
                    viewport_height.get(),
                    camera_x.get(),
                    camera_y.get(),
                    zoom.get(),
                );
                Some(selection_box(
                    start_screen_x,
                    start_screen_y,
                    end_screen_x,
                    end_screen_y,
                ))
            } else {
                None
            };

            view! {
                <div
                    node_ref=viewport_ref
                    on:mousedown=move |event: MouseEvent| {
                        let Some((local_x, local_y)) =
                            viewport_local_point(&viewport_ref, event.client_x(), event.client_y())
                        else {
                            return;
                        };

                        match event.button() {
                            1 => {
                                event.prevent_default();
                                is_panning.set(true);
                                is_selecting.set(false);
                                dragging_scene_id.set(None);
                                pan_start_client_x.set(event.client_x());
                                pan_start_client_y.set(event.client_y());
                                pan_origin_x.set(camera_x.get());
                                pan_origin_y.set(camera_y.get());
                            }
                            0 => {
                                let (world_x, world_y) = screen_to_world(
                                    local_x,
                                    local_y,
                                    viewport_width.get(),
                                    viewport_height.get(),
                                    camera_x.get(),
                                    camera_y.get(),
                                    zoom.get(),
                                );

                                let layouts = build_scene_layouts(
                                    &scenes.get_untracked(),
                                    viewport_width.get_untracked(),
                                    viewport_height.get_untracked(),
                                );
                                let mut ordered_layouts = layouts;
                                ordered_layouts.sort_by_key(|layout| {
                                    active_scene_id.get_untracked().as_deref()
                                        == Some(layout.scene.id.as_str())
                                });

                                if let Some(layout) = ordered_layouts
                                    .iter()
                                    .rev()
                                    .find(|layout| point_inside_handle(layout, world_x, world_y))
                                {
                                    event.prevent_default();
                                    dragging_scene_id.set(Some(layout.scene.id.clone()));
                                    drag_pointer_start_world_x.set(world_x);
                                    drag_pointer_start_world_y.set(world_y);
                                    drag_scene_origin_x.set(layout.center_x());
                                    drag_scene_origin_y.set(layout.center_y());
                                    drag_did_move.set(false);
                                    is_selecting.set(false);
                                    return;
                                }

                                if let Some(layout) = ordered_layouts
                                    .iter()
                                    .rev()
                                    .find(|layout| point_inside_board(layout, world_x, world_y))
                                {
                                    event.prevent_default();
                                    let is_active = active_scene_id.get_untracked().as_deref()
                                        == Some(layout.scene.id.as_str());
                                    if !is_active {
                                        return;
                                    }

                                    let (clamped_x, clamped_y) = clamp_to_layout(world_x, world_y, layout);
                                    is_selecting.set(true);
                                    dragging_scene_id.set(None);
                                    selection_start_x.set(clamped_x);
                                    selection_start_y.set(clamped_y);
                                    selection_end_x.set(clamped_x);
                                    selection_end_y.set(clamped_y);
                                }
                            }
                            _ => {}
                        }
                    }
                    on:wheel=move |event: WheelEvent| {
                        event.prevent_default();

                        let Some((local_x, local_y)) =
                            viewport_local_point(&viewport_ref, event.client_x(), event.client_y())
                        else {
                            return;
                        };

                        let old_zoom = zoom.get();
                        let next_zoom = if event.delta_y() < 0.0 {
                            (old_zoom * (1.0 + ZOOM_STEP)).min(MAX_ZOOM)
                        } else {
                            (old_zoom / (1.0 + ZOOM_STEP)).max(MIN_ZOOM)
                        };

                        if (next_zoom - old_zoom).abs() < f64::EPSILON {
                            return;
                        }

                        let screen_x = local_x - viewport_width.get() / 2.0;
                        let screen_y = local_y - viewport_height.get() / 2.0;
                        let world_x = (screen_x - camera_x.get()) / old_zoom;
                        let world_y = (screen_y - camera_y.get()) / old_zoom;

                        camera_x.set(screen_x - next_zoom * world_x);
                        camera_y.set(screen_y - next_zoom * world_y);
                        zoom.set(next_zoom);
                    }
                    on:contextmenu=move |event: MouseEvent| event.prevent_default()
                    style=move || format!(
                        "position: absolute; inset: 0; z-index: 1; overflow: hidden; pointer-events: auto; user-select: none; cursor: {}; background: {};",
                        if is_panning.get() {
                            "grabbing"
                        } else if dragging_scene_id.get().is_some() {
                            "move"
                        } else if is_selecting.get() {
                            "crosshair"
                        } else {
                            "grab"
                        },
                        theme.background_color
                    )
                >
                    <div
                        style=format!(
                            "position: absolute; inset: 0; background-color: {}; background-image: \
                            linear-gradient(rgba(255,255,255,0.035) 1px, transparent 1px), \
                            linear-gradient(90deg, rgba(255,255,255,0.035) 1px, transparent 1px), \
                            linear-gradient(rgba(255,255,255,0.08) 1px, transparent 1px), \
                            linear-gradient(90deg, rgba(255,255,255,0.08) 1px, transparent 1px); \
                            background-size: {:.2}px {:.2}px, {:.2}px {:.2}px, {:.2}px {:.2}px, {:.2}px {:.2}px; \
                            background-position: {:.2}px {:.2}px, {:.2}px {:.2}px, {:.2}px {:.2}px, {:.2}px {:.2}px;",
                            theme.background_color,
                            workspace_minor_step,
                            workspace_minor_step,
                            workspace_minor_step,
                            workspace_minor_step,
                            workspace_major_step,
                            workspace_major_step,
                            workspace_major_step,
                            workspace_major_step,
                            minor_offset_x,
                            minor_offset_y,
                            minor_offset_x,
                            minor_offset_y,
                            major_offset_x,
                            major_offset_y,
                            major_offset_x,
                            major_offset_y
                        )
                    />
                    <div style="position: absolute; inset: 0; background: radial-gradient(circle at top, rgba(255,255,255,0.07), transparent 45%), radial-gradient(circle at bottom right, rgba(0,0,0,0.16), transparent 35%);" />

                    <div
                        style=format!(
                            "position: absolute; left: 0; top: 0; width: 0; height: 0; transform: {}; transform-origin: 0 0; pointer-events: none;",
                            world_transform
                        )
                    >
                        {layouts
                            .into_iter()
                            .map(|layout| {
                                let is_active = active_id.as_deref() == Some(layout.scene.id.as_str());
                                let is_dragging = dragging_scene_id.get().as_deref() == Some(layout.scene.id.as_str());
                                let board_background = board_background(theme.ui_bg_primary);
                                let board_border = if is_active { theme.ui_success } else { theme.ui_border };
                                let handle_background = if is_active {
                                    "rgba(0,0,0,0.56)"
                                } else {
                                    "rgba(0,0,0,0.42)"
                                };
                                let blur_filter = if is_active {
                                    "none"
                                } else {
                                    "blur(6px) saturate(0.72) brightness(0.7)"
                                };
                                let board_opacity = if is_active { 1.0 } else { 0.78 };
                                let z_index = if is_dragging {
                                    4
                                } else if is_active {
                                    3
                                } else {
                                    2
                                };
                                let screen_cell = (layout.cell_size * current_zoom).max(1.0);
                                let line_width = grid_line_width_screen(screen_cell) / current_zoom.max(f64::EPSILON);
                                let show_minor_grid = screen_cell >= 8.0;
                                let minor_stroke = if is_active {
                                    "rgba(255,255,255,0.17)"
                                } else {
                                    "rgba(255,255,255,0.12)"
                                };
                                let major_stroke = if is_active {
                                    "rgba(255,255,255,0.06)"
                                } else {
                                    "rgba(255,255,255,0.04)"
                                };
                                let background_image = layout
                                    .scene
                                    .background
                                    .as_ref()
                                    .and_then(|file| {
                                        if file.mime_type.starts_with("image/") {
                                            file_transfer
                                                .file_urls
                                                .get()
                                                .get(&file.hash)
                                                .cloned()
                                        } else {
                                            None
                                        }
                                    });

                                view! {
                                    <>
                                        <div
                                            style=format!(
                                                "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; z-index: {}; padding: 0.55rem 0.85rem; background: {}; border: 1px solid {}; border-radius: 999px; box-shadow: 0 12px 30px rgba(0,0,0,0.24); color: {}; display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; cursor: move; filter: {}; opacity: {:.3};",
                                                layout.handle_left(),
                                                layout.handle_top(),
                                                layout.handle_width(),
                                                BOARD_HANDLE_HEIGHT_PX,
                                                z_index,
                                                handle_background,
                                                board_border,
                                                theme.ui_text_primary,
                                                blur_filter,
                                                board_opacity
                                            )
                                        >
                                            <div style="display: flex; gap: 0.65rem; align-items: center; min-width: 0;">
                                                <span style="font-size: 0.82rem; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                                    {layout.scene.name.clone()}
                                                </span>
                                                <span style=format!("font-size: 0.74rem; color: {}; white-space: nowrap;", theme.ui_text_secondary)>
                                                    {format!("{} x {}", layout.scene.grid.columns, layout.scene.grid.rows)}
                                                </span>
                                            </div>
                                            <span style=format!("font-size: 0.72rem; color: {}; white-space: nowrap;", if is_active { theme.ui_success } else { theme.ui_text_secondary })>
                                                {if is_active { "ACTIVE" } else { "MOVE" }}
                                            </span>
                                        </div>

                                        <div
                                            style=format!(
                                                "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; z-index: {}; border: 2px solid {}; border-radius: 1rem; background: {}; box-shadow: 0 24px 80px rgba(0,0,0,0.30), 0 0 0 1px rgba(255,255,255,0.05); overflow: hidden; filter: {}; opacity: {:.3};",
                                                layout.left(),
                                                layout.top(),
                                                layout.board_width,
                                                layout.board_height,
                                                z_index,
                                                board_border,
                                                board_background,
                                                blur_filter,
                                                board_opacity
                                            )
                                        >
                                            {match background_image {
                                                Some(url) => view! {
                                                    <img
                                                        src=url
                                                        alt=layout.scene.name.clone()
                                                        style=format!(
                                                            "position: absolute; left: 50%; top: 50%; width: {:.2}px; max-width: none; pointer-events: none; transform: translate(-50%, -50%) translate({:.2}px, {:.2}px) scale({:.4}) rotate({:.2}deg); opacity: 0.92;",
                                                            layout.board_width,
                                                            layout.scene.background_offset_x,
                                                            layout.scene.background_offset_y,
                                                            layout.scene.background_scale.max(0.05),
                                                            layout.scene.background_rotation_deg
                                                        )
                                                    />
                                                }.into_any(),
                                                None => ().into_any(),
                                            }}
                                            {if is_active {
                                                view! {
                                                    <svg
                                                        viewBox=format!("0 0 {:.4} {:.4}", layout.board_width, layout.board_height)
                                                        preserveAspectRatio="none"
                                                        style="position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; shape-rendering: geometricPrecision;"
                                                    >
                                                        {if show_minor_grid {
                                                            (0..=layout.scene.grid.columns)
                                                                .filter(|column| column % 5 != 0)
                                                                .map(|column| {
                                                                    let x = f64::from(column) * layout.cell_size;
                                                                    view! {
                                                                        <line
                                                                            x1=format!("{x:.4}")
                                                                            y1="0"
                                                                            x2=format!("{x:.4}")
                                                                            y2=format!("{:.4}", layout.board_height)
                                                                            stroke=minor_stroke
                                                                            stroke-width=format!("{line_width:.4}")
                                                                        />
                                                                    }
                                                                })
                                                                .collect_view()
                                                                .into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                        {if show_minor_grid {
                                                            (0..=layout.scene.grid.rows)
                                                                .filter(|row| row % 5 != 0)
                                                                .map(|row| {
                                                                    let y = f64::from(row) * layout.cell_size;
                                                                    view! {
                                                                        <line
                                                                            x1="0"
                                                                            y1=format!("{y:.4}")
                                                                            x2=format!("{:.4}", layout.board_width)
                                                                            y2=format!("{y:.4}")
                                                                            stroke=minor_stroke
                                                                            stroke-width=format!("{line_width:.4}")
                                                                        />
                                                                    }
                                                                })
                                                                .collect_view()
                                                                .into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                        {(0..=layout.scene.grid.columns)
                                                            .filter(|column| column % 5 == 0)
                                                            .map(|column| {
                                                                let x = f64::from(column) * layout.cell_size;
                                                                view! {
                                                                    <line
                                                                        x1=format!("{x:.4}")
                                                                        y1="0"
                                                                        x2=format!("{x:.4}")
                                                                        y2=format!("{:.4}", layout.board_height)
                                                                        stroke=major_stroke
                                                                        stroke-width=format!("{line_width:.4}")
                                                                    />
                                                                }
                                                            })
                                                            .collect_view()}
                                                        {(0..=layout.scene.grid.rows)
                                                            .filter(|row| row % 5 == 0)
                                                            .map(|row| {
                                                                let y = f64::from(row) * layout.cell_size;
                                                                view! {
                                                                    <line
                                                                        x1="0"
                                                                        y1=format!("{y:.4}")
                                                                        x2=format!("{:.4}", layout.board_width)
                                                                        y2=format!("{y:.4}")
                                                                        stroke=major_stroke
                                                                        stroke-width=format!("{line_width:.4}")
                                                                    />
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </svg>
                                                }.into_any()
                                            } else {
                                                ().into_any()
                                            }}
                                            <div style="position: absolute; inset: 0; box-shadow: inset 0 0 0 1px rgba(255,255,255,0.06);" />
                                            <div
                                                style=format!(
                                                    "position: absolute; left: 1rem; bottom: 1rem; padding: 0.45rem 0.7rem; background: rgba(0,0,0,0.42); border: 1px solid {}; border-radius: 0.75rem; color: {}; font-size: 0.78rem; backdrop-filter: blur(6px);",
                                                    theme.ui_border,
                                                    theme.ui_text_secondary
                                                )
                                            >
                                                {format!(
                                                    "Field: {:.0} x {:.0} ft | {} ft/cell",
                                                    f64::from(layout.scene.grid.columns)
                                                        * f64::from(layout.scene.grid.cell_size_feet),
                                                    f64::from(layout.scene.grid.rows)
                                                        * f64::from(layout.scene.grid.cell_size_feet),
                                                    layout.scene.grid.cell_size_feet
                                                )}
                                            </div>
                                        </div>
                                    </>
                                }
                            })
                            .collect_view()}
                    </div>

                    {move || {
                        if let Some((left, top, width, height)) = selection_overlay {
                            view! {
                                <div
                                    style=format!(
                                        "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; border: 1px dashed {}; background: rgba(37,99,235,0.18); box-shadow: inset 0 0 0 1px rgba(255,255,255,0.06); pointer-events: none; z-index: 5;",
                                        left,
                                        top,
                                        width,
                                        height,
                                        theme.ui_button_primary
                                    )
                                />
                            }
                            .into_any()
                        } else {
                            ().into_any()
                        }
                    }}

                    <For
                        each=move || {
                            cursors.get().into_iter().collect::<Vec<_>>()
                        }
                        key=|(name, _)| name.clone()
                        children=move |(name, cursor_sig)| {
                            let is_me = name == username.get();
                            let visible = {
                                let cursor_visible = cursor_sig.visible;
                                Signal::derive(move || !is_me && cursor_visible.get())
                            };
                            let cursor_x = {
                                let cursor_world_x = cursor_sig.x;
                                let cursor_world_y = cursor_sig.y;
                                Signal::derive(move || {
                                    let (screen_x, _) = world_to_screen(
                                        cursor_world_x.get(),
                                        cursor_world_y.get(),
                                        viewport_width.get(),
                                        viewport_height.get(),
                                        camera_x.get(),
                                        camera_y.get(),
                                        zoom.get(),
                                    );
                                    screen_x
                                })
                            };
                            let cursor_y = {
                                let cursor_world_x = cursor_sig.x;
                                let cursor_world_y = cursor_sig.y;
                                Signal::derive(move || {
                                    let (_, screen_y) = world_to_screen(
                                        cursor_world_x.get(),
                                        cursor_world_y.get(),
                                        viewport_width.get(),
                                        viewport_height.get(),
                                        camera_x.get(),
                                        camera_y.get(),
                                        zoom.get(),
                                    );
                                    screen_y
                                })
                            };

                            view! {
                                <Cursor
                                    username=name
                                    x=cursor_x
                                    y=cursor_y
                                    visible=visible
                                    is_me=is_me
                                    theme=cursor_theme.clone()
                                />
                            }
                        }
                    />

                    <div
                        style=format!(
                            "position: absolute; top: 1rem; left: 50%; transform: translateX(-50%); display: inline-flex; gap: 0.75rem; align-items: center; padding: 0.7rem 0.95rem; background: rgba(0,0,0,0.42); border: 1px solid {}; border-radius: 0.85rem; backdrop-filter: blur(10px); box-shadow: 0 12px 32px rgba(0,0,0,0.20); color: {}; z-index: 6;",
                            theme.ui_border,
                            theme.ui_text_primary
                        )
                    >
                        <div style="font-size: 0.95rem; font-weight: 700;">{"Scene Workspace"}</div>
                        <div style=format!("font-size: 0.8rem; color: {};", theme.ui_text_secondary)>
                            {format!("{} boards", scene_items.len())}
                        </div>
                        <div style=format!("font-size: 0.8rem; color: {};", theme.ui_text_secondary)>
                            {move || match active_scene_id.get() {
                                Some(id) => scenes
                                    .get()
                                    .into_iter()
                                    .find(|scene| scene.id == id)
                                    .map(|scene| format!("active: {}", scene.name))
                                    .unwrap_or_else(|| "active: none".to_string()),
                                None => "active: none".to_string(),
                            }}
                        </div>
                    </div>

                    <div
                        style=format!(
                            "position: absolute; right: 1rem; bottom: 1rem; display: flex; gap: 0.75rem; align-items: center; padding: 0.55rem 0.75rem; background: rgba(0,0,0,0.38); border: 1px solid {}; border-radius: 0.75rem; color: {}; font-size: 0.78rem; backdrop-filter: blur(8px); z-index: 6; flex-wrap: wrap; justify-content: flex-end; max-width: min(90vw, 56rem);",
                            theme.ui_border,
                            theme.ui_text_secondary
                        )
                    >
                        <span>{move || format!("Zoom: {}%", (zoom.get() * 100.0).round() as i32)}</span>
                        <span>"MMB drag: camera"</span>
                        <span>"LMB on board: activate/select"</span>
                        <span>"Drag board header: move + snap"</span>
                        <span>"Inactive boards are blurred"</span>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}
