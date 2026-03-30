use super::model::{
    BOARD_HANDLE_GAP_PX, BOARD_HANDLE_HEIGHT_PX, BOARD_HANDLE_MAX_WIDTH_PX, DRAG_EPSILON_PX,
    SNAP_THRESHOLD_PX, WORKSPACE_GRID_STEP_PX, ZOOM_STEP, board_background, centered_token_offset,
    clamp_token_position, clamp_zoom, grid_line_width_screen, point_inside_rect,
    scene_allows_token_interaction, scene_shows_contents, selection_box, should_broadcast_cursor,
    snap_token_position_to_grid, token_position_from_world, token_rect, workspace_board_metrics,
    world_to_screen,
};
use super::storage::{StoredCameraPosition, load_camera_position, save_camera_position};
use super::token_editor::{SceneTokenEditor, SceneTokenEditorDraft, SceneTokenEditorValue};
use super::token_layer::SceneTokenLayer;
use super::token_menu::SceneTokenMenu;
use super::view_model::SceneBoardViewModel;
use super::workspace_hint::WorkspaceHintCard;
use crate::components::app::mouse_handler::{
    send_mouse_event_throttled, update_local_cursor_world,
};
use crate::components::cursor::Cursor;
use crate::components::websocket::{
    CursorSignals, FileTransferState, StoredTokenLibraryItem, WsSender, save_token_library_item,
    token_library_key,
};
use crate::config;
use crate::config::Theme;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{ClientEvent, Scene, SceneUpdatePayload, Token, TokenMovePayload};
use uuid::Uuid;
use web_sys::{MouseEvent, WheelEvent};

// ---------------------------------------------------------------------------
// Local helpers (layout + geometry specific to this view)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SceneLayout {
    scene: Scene,
    cell_size: f64,
    board_width: f64,
    board_height: f64,
}

#[derive(Clone)]
struct TokenMenuState {
    scene_id: String,
    token_id: String,
    token: Token,
    token_name: String,
    screen_x: f64,
    screen_y: f64,
}

const TOKEN_DRAG_EPSILON_CELLS: f32 = 0.02;

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
    let w = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);
    let h = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(720.0);
    (w, h)
}

fn viewport_local_point(
    viewport_ref: &NodeRef<html::Div>,
    client_x: i32,
    client_y: i32,
) -> Option<(f64, f64)> {
    let vp = viewport_ref.get()?;
    let rect = vp.get_bounding_client_rect();
    let x = (f64::from(client_x) - rect.left()).clamp(0.0, rect.width());
    let y = (f64::from(client_y) - rect.top()).clamp(0.0, rect.height());
    Some((x, y))
}

fn build_scene_layouts(scenes: &[Scene]) -> Vec<SceneLayout> {
    scenes
        .iter()
        .cloned()
        .map(|scene| {
            let (cell_size, board_width, board_height) =
                workspace_board_metrics(scene.grid.columns, scene.grid.rows);
            SceneLayout {
                scene,
                cell_size,
                board_width,
                board_height,
            }
        })
        .collect()
}

fn point_inside_board(layout: &SceneLayout, wx: f64, wy: f64) -> bool {
    point_inside_rect(
        wx,
        wy,
        layout.left(),
        layout.top(),
        layout.board_width,
        layout.board_height,
    )
}

fn point_inside_handle(layout: &SceneLayout, wx: f64, wy: f64) -> bool {
    point_inside_rect(
        wx,
        wy,
        layout.handle_left(),
        layout.handle_top(),
        layout.handle_width(),
        BOARD_HANDLE_HEIGHT_PX,
    )
}

fn token_hit(layout: &SceneLayout, wx: f64, wy: f64) -> Option<Token> {
    layout
        .scene
        .tokens
        .iter()
        .rev()
        .find(|token| {
            let (left, top, width, height) = token_rect(
                layout.left(),
                layout.top(),
                layout.cell_size,
                token.x,
                token.y,
                token.width_cells,
                token.height_cells,
            );
            point_inside_rect(wx, wy, left, top, width, height)
        })
        .cloned()
}

fn clamp_to_layout(wx: f64, wy: f64, layout: &SceneLayout) -> (f64, f64) {
    (
        wx.clamp(layout.left(), layout.right()),
        wy.clamp(layout.top(), layout.bottom()),
    )
}

fn snap_scene_position(
    candidate_x: f64,
    candidate_y: f64,
    dragged_width: f64,
    dragged_height: f64,
    other_layouts: &[SceneLayout],
) -> (f64, f64) {
    let mut best = (candidate_x, candidate_y);
    let mut best_score = f64::MAX;

    for layout in other_layouts {
        let h_targets = [
            layout.center_y(),
            layout.top() + dragged_height / 2.0,
            layout.bottom() - dragged_height / 2.0,
        ];
        let v_targets = [
            layout.center_x(),
            layout.left() + dragged_width / 2.0,
            layout.right() - dragged_width / 2.0,
        ];

        let neighbor_targets = [
            (layout.right() + dragged_width / 2.0, h_targets[0]),
            (layout.right() + dragged_width / 2.0, h_targets[1]),
            (layout.right() + dragged_width / 2.0, h_targets[2]),
            (layout.left() - dragged_width / 2.0, h_targets[0]),
            (layout.left() - dragged_width / 2.0, h_targets[1]),
            (layout.left() - dragged_width / 2.0, h_targets[2]),
            (v_targets[0], layout.bottom() + dragged_height / 2.0),
            (v_targets[1], layout.bottom() + dragged_height / 2.0),
            (v_targets[2], layout.bottom() + dragged_height / 2.0),
            (v_targets[0], layout.top() - dragged_height / 2.0),
            (v_targets[1], layout.top() - dragged_height / 2.0),
            (v_targets[2], layout.top() - dragged_height / 2.0),
        ];

        for (tx, ty) in neighbor_targets {
            let dx = (candidate_x - tx).abs();
            let dy = (candidate_y - ty).abs();
            if dx <= SNAP_THRESHOLD_PX && dy <= SNAP_THRESHOLD_PX {
                let score = dx + dy;
                if score < best_score {
                    best_score = score;
                    best = (tx, ty);
                }
            }
        }
    }

    best
}

fn update_scene_position(scenes: RwSignal<Vec<Scene>>, id: &str, x: f64, y: f64) {
    scenes.update(|items| {
        if let Some(scene) = items.iter_mut().find(|s| s.id == id) {
            scene.workspace_x = x as f32;
            scene.workspace_y = y as f32;
        }
    });
}

fn update_token_position(scenes: RwSignal<Vec<Scene>>, id: &str, x: f32, y: f32) {
    scenes.update(|items| {
        for scene in items {
            if let Some(token) = scene.tokens.iter_mut().find(|token| token.id == id) {
                token.x = x;
                token.y = y;
                break;
            }
        }
    });
}

fn place_library_token(
    scenes: RwSignal<Vec<Scene>>,
    scene_id: &str,
    token: &StoredTokenLibraryItem,
    x: f32,
    y: f32,
) -> Option<Scene> {
    let mut updated_scene = None::<Scene>;

    scenes.update(|items| {
        let Some(scene) = items.iter_mut().find(|scene| scene.id == scene_id) else {
            return;
        };

        scene.tokens.push(Token {
            id: Uuid::new_v4().to_string(),
            name: token.name.clone(),
            image: token.image.clone(),
            x,
            y,
            width_cells: token.width_cells,
            height_cells: token.height_cells,
        });
        updated_scene = Some(scene.clone());
    });

    updated_scene
}

fn remove_token_from_scene(
    scenes: RwSignal<Vec<Scene>>,
    scene_id: &str,
    token_id: &str,
) -> Option<Scene> {
    let mut updated_scene = None::<Scene>;

    scenes.update(|items| {
        let Some(scene) = items.iter_mut().find(|scene| scene.id == scene_id) else {
            return;
        };

        let original_len = scene.tokens.len();
        scene.tokens.retain(|token| token.id != token_id);
        if scene.tokens.len() != original_len {
            updated_scene = Some(scene.clone());
        }
    });

    updated_scene
}

fn update_token_details(
    scenes: RwSignal<Vec<Scene>>,
    scene_id: &str,
    token_id: &str,
    name: &str,
    width_cells: u16,
    height_cells: u16,
) -> Option<Scene> {
    let mut updated_scene = None::<Scene>;

    scenes.update(|items| {
        let Some(scene) = items.iter_mut().find(|scene| scene.id == scene_id) else {
            return;
        };

        let columns = scene.grid.columns;
        let rows = scene.grid.rows;
        let Some(token) = scene.tokens.iter_mut().find(|token| token.id == token_id) else {
            return;
        };

        token.name = name.to_string();
        token.width_cells = width_cells;
        token.height_cells = height_cells;
        let (x, y) =
            clamp_token_position(token.x, token.y, columns, rows, width_cells, height_cells);
        token.x = x;
        token.y = y;
        updated_scene = Some(scene.clone());
    });

    updated_scene
}

fn sort_token_library_items(items: &mut [StoredTokenLibraryItem]) {
    items.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn send_event(ws_sender: &ReadSignal<Option<WsSender>>, event: ClientEvent) {
    if let Some(sender) = ws_sender.get_untracked() {
        let _ = sender.try_send_event(event);
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn SceneBoard(
    room_id: ReadSignal<String>,
    #[prop(into)] scenes: RwSignal<Vec<Scene>>,
    #[prop(into)] active_scene_id: RwSignal<Option<String>>,
    #[prop(into)] show_workspace_hint: RwSignal<bool>,
    #[prop(into)] show_inactive_scene_contents: RwSignal<bool>,
    #[prop(into)] token_library_items: RwSignal<Vec<StoredTokenLibraryItem>>,
    #[prop(into)] dragging_library_token_id: RwSignal<Option<String>>,
    cursors: ReadSignal<std::collections::HashMap<String, CursorSignals>>,
    set_cursors: WriteSignal<std::collections::HashMap<String, CursorSignals>>,
    file_transfer: FileTransferState,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    config: config::Config,
    theme: Theme,
) -> impl IntoView {
    let (initial_vw, initial_vh) = viewport_size();
    let initial_room_id = room_id.get_untracked();
    let initial_camera = load_camera_position(&initial_room_id).unwrap_or(StoredCameraPosition {
        x: 0.0,
        y: 0.0,
        zoom: 1.0,
    });
    let vm = SceneBoardViewModel::new(
        initial_vw,
        initial_vh,
        initial_camera.x,
        initial_camera.y,
        clamp_zoom(initial_camera.zoom),
    );
    let viewport_ref = NodeRef::<html::Div>::new();
    let config = StoredValue::new(config);
    let drag_did_move = RwSignal::new(false);
    let token_drag_did_move = RwSignal::new(false);
    let loaded_room_id = RwSignal::new(initial_room_id);
    let token_menu = RwSignal::new(None::<TokenMenuState>);
    let token_editor = RwSignal::new(None::<SceneTokenEditorDraft>);

    Effect::new(move |_| {
        let current_room_id = room_id.get();
        let camera = load_camera_position(&current_room_id).unwrap_or(StoredCameraPosition {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        });
        vm.set_view_transform(camera.x, camera.y, clamp_zoom(camera.zoom));
        loaded_room_id.set(current_room_id);
        token_menu.set(None);
        token_editor.set(None);
    });

    {
        Effect::new(move |_| {
            let current_room_id = room_id.get();
            if current_room_id.is_empty() || loaded_room_id.get() != current_room_id {
                return;
            }
            save_camera_position(
                &current_room_id,
                StoredCameraPosition {
                    x: vm.camera_x.get(),
                    y: vm.camera_y.get(),
                    zoom: vm.zoom.get(),
                },
            );
        });
    }

    // Global event listeners
    Effect::new(move |_| {
        let resize_handle = window_event_listener(ev::resize, move |_| {
            let (w, h) = viewport_size();
            vm.viewport_width.set(w);
            vm.viewport_height.set(h);
        });

        let mouse_move_handle = window_event_listener(ev::mousemove, move |event: MouseEvent| {
            let Some((local_x, local_y)) =
                viewport_local_point(&viewport_ref, event.client_x(), event.client_y())
            else {
                return;
            };

            vm.update_pointer(local_x, local_y);
            vm.update_pan(local_x, local_y);

            let (world_x, world_y) = super::model::screen_to_world(
                local_x,
                local_y,
                vm.viewport_width.get(),
                vm.viewport_height.get(),
                vm.camera_x.get(),
                vm.camera_y.get(),
                vm.zoom.get(),
            );

            let layouts = build_scene_layouts(&scenes.get_untracked());
            let hovered_scene_id = layouts
                .iter()
                .rev()
                .find(|layout| point_inside_board(layout, world_x, world_y))
                .map(|layout| layout.scene.id.as_str());
            let current_user = username.get_untracked();
            update_local_cursor_world(&current_user, world_x, world_y, set_cursors);
            if should_broadcast_cursor(
                hovered_scene_id,
                active_scene_id.get_untracked().as_deref(),
                show_inactive_scene_contents.get_untracked(),
            ) {
                send_mouse_event_throttled(world_x, world_y, current_user, ws_sender, config);
            }

            if let Some(scene_id) = vm.dragging_scene_id.get() {
                let Some((_, candidate_x, candidate_y)) =
                    vm.compute_scene_drag_position(world_x, world_y)
                else {
                    return;
                };

                let Some(dragged_layout) = layouts.iter().find(|l| l.scene.id == scene_id).cloned()
                else {
                    return;
                };
                let other_layouts: Vec<_> = layouts
                    .into_iter()
                    .filter(|l| l.scene.id != scene_id)
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
                    (snapped_x - vm.drag_origin_scene_x()).abs() > DRAG_EPSILON_PX
                        || (snapped_y - vm.drag_origin_scene_y()).abs() > DRAG_EPSILON_PX,
                );
                return;
            }

            if let Some(token_id) = vm.dragging_token_id.get() {
                let Some(token_layout) = layouts
                    .iter()
                    .find(|layout| layout.scene.tokens.iter().any(|token| token.id == token_id))
                else {
                    return;
                };

                let (mut token_x, mut token_y) = token_position_from_world(
                    world_x,
                    world_y,
                    token_layout.left(),
                    token_layout.top(),
                    token_layout.cell_size,
                    token_layout.scene.grid.columns,
                    token_layout.scene.grid.rows,
                    vm.token_drag_width_cells(),
                    vm.token_drag_height_cells(),
                    vm.token_drag_offset_x(),
                    vm.token_drag_offset_y(),
                );
                if !event.ctrl_key() {
                    (token_x, token_y) = snap_token_position_to_grid(
                        token_x,
                        token_y,
                        token_layout.scene.grid.columns,
                        token_layout.scene.grid.rows,
                        vm.token_drag_width_cells(),
                        vm.token_drag_height_cells(),
                    );
                }

                update_token_position(scenes, &token_id, token_x, token_y);
                token_drag_did_move.set(
                    (token_x - vm.token_drag_origin_x()).abs() > TOKEN_DRAG_EPSILON_CELLS
                        || (token_y - vm.token_drag_origin_y()).abs() > TOKEN_DRAG_EPSILON_CELLS,
                );
                return;
            }

            if vm.is_selecting.get_untracked() {
                let Some(active_layout) = layouts.iter().find(|l| {
                    Some(l.scene.id.as_str()) == active_scene_id.get_untracked().as_deref()
                }) else {
                    return;
                };
                let (cx, cy) = clamp_to_layout(world_x, world_y, active_layout);
                vm.selection_end_x.set(cx);
                vm.selection_end_y.set(cy);
            }
        });

        let mouse_up_handle = window_event_listener(ev::mouseup, move |event: MouseEvent| {
            let local_point =
                viewport_local_point(&viewport_ref, event.client_x(), event.client_y());

            if let Some(scene_id) = vm.dragging_scene_id.get_untracked()
                && drag_did_move.get_untracked()
                && let Some(scene) = scenes
                    .get_untracked()
                    .into_iter()
                    .find(|s| s.id == scene_id)
            {
                send_event(
                    &ws_sender,
                    ClientEvent::SceneUpdate(SceneUpdatePayload {
                        scene,
                        actor: username.get_untracked(),
                    }),
                );
            }

            if let Some(token_id) = vm.dragging_token_id.get_untracked() {
                if token_drag_did_move.get_untracked() {
                    if let Some(token) = scenes
                        .get_untracked()
                        .iter()
                        .flat_map(|scene| scene.tokens.iter())
                        .find(|token| token.id == token_id)
                        .cloned()
                    {
                        send_event(
                            &ws_sender,
                            ClientEvent::TokenMove(TokenMovePayload {
                                token_id,
                                x: token.x,
                                y: token.y,
                                actor: username.get_untracked(),
                            }),
                        );
                    }
                } else if let Some((local_x, local_y)) = local_point
                    && let Some((scene_id, token)) =
                        scenes.get_untracked().iter().find_map(|scene| {
                            scene
                                .tokens
                                .iter()
                                .find(|token| token.id == token_id)
                                .map(|token| (scene.id.clone(), token.clone()))
                        })
                {
                    token_menu.set(Some(TokenMenuState {
                        scene_id,
                        token_id,
                        token_name: token.name.clone(),
                        token,
                        screen_x: local_x + 14.0,
                        screen_y: local_y + 14.0,
                    }));
                }
            }

            if let Some(library_token_id) = dragging_library_token_id.get_untracked()
                && let Some(item) = token_library_items
                    .get_untracked()
                    .into_iter()
                    .find(|item| item.id == library_token_id)
            {
                let layouts = build_scene_layouts(&scenes.get_untracked());
                let (world_x, world_y) = super::model::screen_to_world(
                    vm.pointer_local_x.get_untracked(),
                    vm.pointer_local_y.get_untracked(),
                    vm.viewport_width.get_untracked(),
                    vm.viewport_height.get_untracked(),
                    vm.camera_x.get_untracked(),
                    vm.camera_y.get_untracked(),
                    vm.zoom.get_untracked(),
                );
                let active_id = active_scene_id.get_untracked();
                let allow_inactive = show_inactive_scene_contents.get_untracked();
                if let Some(target_layout) = layouts.iter().rev().find(|layout| {
                    point_inside_board(layout, world_x, world_y)
                        && scene_allows_token_interaction(
                            layout.scene.id.as_str(),
                            active_id.as_deref(),
                            allow_inactive,
                        )
                }) {
                    let (offset_x, offset_y) = centered_token_offset(
                        target_layout.cell_size,
                        item.width_cells,
                        item.height_cells,
                    );
                    let (mut token_x, mut token_y) = token_position_from_world(
                        world_x,
                        world_y,
                        target_layout.left(),
                        target_layout.top(),
                        target_layout.cell_size,
                        target_layout.scene.grid.columns,
                        target_layout.scene.grid.rows,
                        item.width_cells,
                        item.height_cells,
                        offset_x,
                        offset_y,
                    );
                    if !event.ctrl_key() {
                        (token_x, token_y) = snap_token_position_to_grid(
                            token_x,
                            token_y,
                            target_layout.scene.grid.columns,
                            target_layout.scene.grid.rows,
                            item.width_cells,
                            item.height_cells,
                        );
                    }
                    if let Some(scene) = place_library_token(
                        scenes,
                        &target_layout.scene.id,
                        &item,
                        token_x,
                        token_y,
                    ) {
                        send_event(
                            &ws_sender,
                            ClientEvent::SceneUpdate(SceneUpdatePayload {
                                scene,
                                actor: username.get_untracked(),
                            }),
                        );
                    }
                }
            }

            vm.end_scene_drag();
            vm.end_token_drag();
            dragging_library_token_id.set(None);
            drag_did_move.set(false);
            token_drag_did_move.set(false);
            vm.end_pan();
            vm.is_selecting.set(false);
        });

        on_cleanup(move || {
            drop(resize_handle);
            drop(mouse_move_handle);
            drop(mouse_up_handle);
        });
    });

    // Clear selection when active scene changes
    Effect::new(move |_| {
        let _ = active_scene_id.get();
        vm.is_selecting.set(false);
        vm.end_token_drag();
        dragging_library_token_id.set(None);
        token_menu.set(None);
        token_editor.set(None);
    });

    view! {
        {move || {
            let active_id = active_scene_id.get();
            let show_inactive_contents = show_inactive_scene_contents.get();
            let scene_items = scenes.get();
            if scene_items.is_empty() {
                return ().into_any();
            }

            let mut layouts = build_scene_layouts(&scene_items);
            layouts.sort_by_key(|layout| {
                let is_dragging = vm.dragging_scene_id.get().as_deref() == Some(layout.scene.id.as_str());
                let is_active = active_id.as_deref() == Some(layout.scene.id.as_str());
                (is_dragging, is_active)
            });

            let zoom = vm.zoom.get();
            let cam_x = vm.camera_x.get();
            let cam_y = vm.camera_y.get();
            let vw = vm.viewport_width.get();
            let vh = vm.viewport_height.get();

            let workspace_minor_step = (WORKSPACE_GRID_STEP_PX * zoom).max(18.0);
            let workspace_major_step = workspace_minor_step * 4.0;
            let minor_offset_x = (vw / 2.0 + cam_x).rem_euclid(workspace_minor_step);
            let minor_offset_y = (vh / 2.0 + cam_y).rem_euclid(workspace_minor_step);
            let major_offset_x = (vw / 2.0 + cam_x).rem_euclid(workspace_major_step);
            let major_offset_y = (vh / 2.0 + cam_y).rem_euclid(workspace_major_step);
            let world_transform = format!(
                "translate({:.2}px, {:.2}px) scale({:.4})",
                vw / 2.0 + cam_x,
                vh / 2.0 + cam_y,
                zoom
            );
            let cursor_theme = theme.clone();
            let token_menu_theme = theme.clone();
            let token_editor_theme = theme.clone();
            let workspace_hint_theme = theme.clone();
            let file_urls = file_transfer.file_urls.get();

            let selection_overlay = if vm.is_selecting.get() {
                let (ssx, ssy) = world_to_screen(
                    vm.selection_start_x.get(), vm.selection_start_y.get(),
                    vw, vh, cam_x, cam_y, zoom,
                );
                let (sex, sey) = world_to_screen(
                    vm.selection_end_x.get(), vm.selection_end_y.get(),
                    vw, vh, cam_x, cam_y, zoom,
                );
                Some(selection_box(ssx, ssy, sex, sey))
            } else {
                None
            };

            view! {
                <div
                    node_ref=viewport_ref
                    on:mousedown=move |event: MouseEvent| {
                        let Some((local_x, local_y)) =
                            viewport_local_point(&viewport_ref, event.client_x(), event.client_y())
                        else { return; };
                        vm.update_pointer(local_x, local_y);
                        token_menu.set(None);

                        match event.button() {
                            // Middle-click pan
                            1 => {
                                event.prevent_default();
                                vm.is_selecting.set(false);
                                vm.end_scene_drag();
                                vm.end_token_drag();
                                dragging_library_token_id.set(None);
                                vm.start_pan(local_x, local_y);
                            }
                            // Left-click: drag handle or start selection
                            0 => {
                                let (world_x, world_y) = super::model::screen_to_world(
                                    local_x, local_y,
                                    vm.viewport_width.get(), vm.viewport_height.get(),
                                    vm.camera_x.get(), vm.camera_y.get(), vm.zoom.get(),
                                );
                                let layouts = build_scene_layouts(&scenes.get_untracked());
                                let mut ordered = layouts;
                                ordered.sort_by_key(|l| {
                                    active_scene_id.get_untracked().as_deref() == Some(l.scene.id.as_str())
                                });

                                if let Some(layout) = ordered.iter().rev()
                                    .find(|l| point_inside_handle(l, world_x, world_y))
                                {
                                    event.prevent_default();
                                    vm.start_scene_drag(
                                        layout.scene.id.clone(),
                                        world_x, world_y,
                                        layout.center_x(), layout.center_y(),
                                    );
                                    drag_did_move.set(false);
                                    vm.is_selecting.set(false);
                                    return;
                                }

                                if let Some(layout) = ordered.iter().rev()
                                    .find(|l| point_inside_board(l, world_x, world_y))
                                {
                                    event.prevent_default();
                                    let is_active = active_scene_id.get_untracked().as_deref()
                                        == Some(layout.scene.id.as_str());
                                    let can_interact = scene_allows_token_interaction(
                                        layout.scene.id.as_str(),
                                        active_scene_id.get_untracked().as_deref(),
                                        show_inactive_scene_contents.get_untracked(),
                                    );

                                    if let Some(token) = token_hit(layout, world_x, world_y) {
                                        if !can_interact {
                                            return;
                                        }
                                        let (token_left, token_top, _, _) = token_rect(
                                            layout.left(),
                                            layout.top(),
                                            layout.cell_size,
                                            token.x,
                                            token.y,
                                            token.width_cells,
                                            token.height_cells,
                                        );
                                        vm.is_selecting.set(false);
                                        vm.end_scene_drag();
                                        dragging_library_token_id.set(None);
                                        vm.start_token_drag(
                                            token.id.clone(),
                                            token.width_cells,
                                            token.height_cells,
                                            world_x - token_left,
                                            world_y - token_top,
                                            token.x,
                                            token.y,
                                        );
                                        token_drag_did_move.set(false);
                                        return;
                                    }

                                    if !is_active {
                                        return;
                                    }

                                    let (cx, cy) = clamp_to_layout(world_x, world_y, layout);
                                    vm.is_selecting.set(true);
                                    vm.end_scene_drag();
                                    vm.end_token_drag();
                                    dragging_library_token_id.set(None);
                                    vm.selection_start_x.set(cx);
                                    vm.selection_start_y.set(cy);
                                    vm.selection_end_x.set(cx);
                                    vm.selection_end_y.set(cy);
                                }
                            }
                            _ => {}
                        }
                    }
                    on:wheel=move |event: WheelEvent| {
                        event.prevent_default();
                        let Some((local_x, local_y)) =
                            viewport_local_point(&viewport_ref, event.client_x(), event.client_y())
                        else { return; };

                        let old_zoom = vm.zoom.get();
                        let next_zoom = if event.delta_y() < 0.0 {
                            clamp_zoom(old_zoom * (1.0 + ZOOM_STEP))
                        } else {
                            clamp_zoom(old_zoom / (1.0 + ZOOM_STEP))
                        };

                        if (next_zoom - old_zoom).abs() < f64::EPSILON { return; }

                        // Focal-point zoom: keep world point under cursor stationary
                        let screen_x = local_x - vm.viewport_width.get() / 2.0;
                        let screen_y = local_y - vm.viewport_height.get() / 2.0;
                        let world_x = (screen_x - vm.camera_x.get()) / old_zoom;
                        let world_y = (screen_y - vm.camera_y.get()) / old_zoom;
                        vm.camera_x.set(screen_x - next_zoom * world_x);
                        vm.camera_y.set(screen_y - next_zoom * world_y);
                        vm.zoom.set(next_zoom);
                    }
                    on:contextmenu=move |event: MouseEvent| event.prevent_default()
                    style=move || format!(
                        "position: absolute; inset: 0; z-index: 1; overflow: hidden; \
                         pointer-events: auto; user-select: none; cursor: {}; background: {};",
                        if vm.is_panning.get() { "grabbing" }
                        else if vm.dragging_scene_id.get().is_some() { "move" }
                        else if vm.dragging_token_id.get().is_some() { "grabbing" }
                        else if dragging_library_token_id.get().is_some() { "copy" }
                        else if vm.is_selecting.get() { "crosshair" }
                        else { "grab" },
                        theme.background_color
                    )
                >
                    // Background grid
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
                            workspace_minor_step, workspace_minor_step,
                            workspace_minor_step, workspace_minor_step,
                            workspace_major_step, workspace_major_step,
                            workspace_major_step, workspace_major_step,
                            minor_offset_x, minor_offset_y,
                            minor_offset_x, minor_offset_y,
                            major_offset_x, major_offset_y,
                            major_offset_x, major_offset_y
                        )
                    />
                    <div style="position: absolute; inset: 0; background: radial-gradient(circle at top, rgba(255,255,255,0.07), transparent 45%), radial-gradient(circle at bottom right, rgba(0,0,0,0.16), transparent 35%);" />

                    // World-space container (all boards)
                    <div style=format!(
                        "position: absolute; left: 0; top: 0; width: 0; height: 0; transform: {}; transform-origin: 0 0; pointer-events: none;",
                        world_transform
                    )>
                        {layouts.into_iter().map(|layout| {
                            let is_active = active_id.as_deref() == Some(layout.scene.id.as_str());
                            let is_dragging = vm.dragging_scene_id.get().as_deref() == Some(layout.scene.id.as_str());
                            let show_scene_contents = scene_shows_contents(
                                layout.scene.id.as_str(),
                                active_id.as_deref(),
                                show_inactive_contents,
                            );
                            let board_bg = board_background(theme.ui_bg_primary);
                            let board_border = if is_active { theme.ui_success } else { theme.ui_border };
                            let handle_background = if is_active { "rgba(0,0,0,0.56)" } else { "rgba(0,0,0,0.42)" };
                            let blur_filter = if show_scene_contents { "none" } else { "blur(6px) saturate(0.72) brightness(0.7)" };
                            let board_opacity = if show_scene_contents { 1.0 } else { 0.78_f64 };
                            let z_index = if is_dragging { 4 } else if is_active { 3 } else { 2 };
                            let screen_cell = (layout.cell_size * zoom).max(1.0);
                            let line_width = grid_line_width_screen(screen_cell) / zoom.max(f64::EPSILON);
                            let show_minor_grid = screen_cell >= 8.0;
                            let minor_stroke = if is_active { "rgba(255,255,255,0.17)" } else { "rgba(255,255,255,0.12)" };
                            let major_stroke = if is_active { "rgba(255,255,255,0.06)" } else { "rgba(255,255,255,0.04)" };
                            let background_image = layout.scene.background.as_ref().and_then(|file| {
                                if file.mime_type.starts_with("image/") {
                                    file_urls.get(&file.hash).cloned()
                                } else {
                                    None
                                }
                            });

                            view! {
                                <>
                                    // Drag handle
                                    <div style=format!(
                                        "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; \
                                         z-index: {}; padding: 0.55rem 0.85rem; background: {}; border: 1px solid {}; \
                                         border-radius: 999px; box-shadow: 0 12px 30px rgba(0,0,0,0.24); color: {}; \
                                         display: flex; flex-direction: column; justify-content: center; gap: 0.15rem; \
                                         cursor: move; filter: {}; opacity: {:.3};",
                                        layout.handle_left(), layout.handle_top(),
                                        layout.handle_width(), BOARD_HANDLE_HEIGHT_PX,
                                        z_index, handle_background, board_border,
                                        theme.ui_text_primary, blur_filter, board_opacity
                                    )>
                                        <div style="display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; min-width: 0;">
                                            <div style="display: flex; gap: 0.65rem; align-items: center; min-width: 0;">
                                                <span style="font-size: 0.82rem; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                                    {layout.scene.name.clone()}
                                                </span>
                                                <span style=format!("font-size: 0.74rem; color: {}; white-space: nowrap;", theme.ui_text_secondary)>
                                                    {format!("{} x {}", layout.scene.grid.columns, layout.scene.grid.rows)}
                                                </span>
                                            </div>
                                            <span style=format!(
                                                "font-size: 0.72rem; color: {}; white-space: nowrap;",
                                                if is_active { theme.ui_success } else { theme.ui_text_secondary }
                                            )>
                                                {if is_active { "ACTIVE" } else { "MOVE" }}
                                            </span>
                                        </div>
                                        <div style=format!(
                                            "font-size: 0.69rem; color: {}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                            theme.ui_text_secondary
                                        )>
                                            {if show_scene_contents {
                                                format!(
                                                    "Field: {:.0} x {:.0} ft | {} ft/cell | {} tokens",
                                                    f64::from(layout.scene.grid.columns) * f64::from(layout.scene.grid.cell_size_feet),
                                                    f64::from(layout.scene.grid.rows) * f64::from(layout.scene.grid.cell_size_feet),
                                                    layout.scene.grid.cell_size_feet,
                                                    layout.scene.tokens.len()
                                                )
                                            } else {
                                                format!(
                                                    "Field: {:.0} x {:.0} ft | {} ft/cell",
                                                    f64::from(layout.scene.grid.columns) * f64::from(layout.scene.grid.cell_size_feet),
                                                    f64::from(layout.scene.grid.rows) * f64::from(layout.scene.grid.cell_size_feet),
                                                    layout.scene.grid.cell_size_feet
                                                )
                                            }}
                                        </div>
                                    </div>

                                    // Board tile
                                    <div style=format!(
                                        "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; \
                                         z-index: {}; border: 2px solid {}; border-radius: 1rem; background: {}; \
                                         box-shadow: 0 24px 80px rgba(0,0,0,0.30), 0 0 0 1px rgba(255,255,255,0.05); \
                                         overflow: hidden; filter: {}; opacity: {:.3};",
                                        layout.left(), layout.top(),
                                        layout.board_width, layout.board_height,
                                        z_index, board_border, board_bg, blur_filter, board_opacity
                                    )>
                                        {match background_image {
                                            Some(url) => view! {
                                                <img
                                                    src=url
                                                    alt=layout.scene.name.clone()
                                                    style=format!(
                                                        "position: absolute; left: 50%; top: 50%; width: {:.2}px; max-width: none; \
                                                         pointer-events: none; transform: translate(-50%, -50%) translate({:.2}px, {:.2}px) \
                                                         scale({:.4}) rotate({:.2}deg); opacity: 0.92;",
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
                                        {if show_scene_contents {
                                            view! {
                                                <svg
                                                    viewBox=format!("0 0 {:.4} {:.4}", layout.board_width, layout.board_height)
                                                    preserveAspectRatio="none"
                                                    style="position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; shape-rendering: geometricPrecision;"
                                                >
                                                    {if show_minor_grid {
                                                        (0..=layout.scene.grid.columns)
                                                            .filter(|c| c % 5 != 0)
                                                            .map(|c| {
                                                                let x = f64::from(c) * layout.cell_size;
                                                                view! {
                                                                    <line x1=format!("{x:.4}") y1="0" x2=format!("{x:.4}") y2=format!("{:.4}", layout.board_height)
                                                                        stroke=minor_stroke stroke-width=format!("{line_width:.4}") />
                                                                }
                                                            })
                                                            .collect_view()
                                                            .into_any()
                                                    } else { ().into_any() }}
                                                    {if show_minor_grid {
                                                        (0..=layout.scene.grid.rows)
                                                            .filter(|r| r % 5 != 0)
                                                            .map(|r| {
                                                                let y = f64::from(r) * layout.cell_size;
                                                                view! {
                                                                    <line x1="0" y1=format!("{y:.4}") x2=format!("{:.4}", layout.board_width) y2=format!("{y:.4}")
                                                                        stroke=minor_stroke stroke-width=format!("{line_width:.4}") />
                                                                }
                                                            })
                                                            .collect_view()
                                                            .into_any()
                                                    } else { ().into_any() }}
                                                    {(0..=layout.scene.grid.columns)
                                                        .filter(|c| c % 5 == 0)
                                                        .map(|c| {
                                                            let x = f64::from(c) * layout.cell_size;
                                                            view! {
                                                                <line x1=format!("{x:.4}") y1="0" x2=format!("{x:.4}") y2=format!("{:.4}", layout.board_height)
                                                                    stroke=major_stroke stroke-width=format!("{line_width:.4}") />
                                                            }
                                                        })
                                                        .collect_view()}
                                                    {(0..=layout.scene.grid.rows)
                                                        .filter(|r| r % 5 == 0)
                                                        .map(|r| {
                                                            let y = f64::from(r) * layout.cell_size;
                                                            view! {
                                                                <line x1="0" y1=format!("{y:.4}") x2=format!("{:.4}", layout.board_width) y2=format!("{y:.4}")
                                                                    stroke=major_stroke stroke-width=format!("{line_width:.4}") />
                                                            }
                                                        })
                                                        .collect_view()}
                                                </svg>
                                            }.into_any()
                                        } else { ().into_any() }}
                                        <SceneTokenLayer
                                            tokens=if show_scene_contents { layout.scene.tokens.clone() } else { Vec::new() }
                                            cell_size=layout.cell_size
                                            dragging_token_id=vm.dragging_token_id.get()
                                            file_urls=file_urls.clone()
                                            theme=theme.clone()
                                        />
                                        <div style="position: absolute; inset: 0; box-shadow: inset 0 0 0 1px rgba(255,255,255,0.06);" />
                                    </div>
                                </>
                            }
                        }).collect_view()}
                    </div>

                    // Selection overlay
                    {move || {
                        if let Some((left, top, width, height)) = selection_overlay {
                            view! {
                                <div style=format!(
                                    "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; \
                                     border: 1px dashed {}; background: rgba(37,99,235,0.18); \
                                     box-shadow: inset 0 0 0 1px rgba(255,255,255,0.06); pointer-events: none; z-index: 5;",
                                    left, top, width, height, theme.ui_button_primary
                                ) />
                            }.into_any()
                        } else {
                            ().into_any()
                        }
                    }}

                    {move || {
                        token_menu.get().map(|menu| {
                            let menu_for_save = menu.clone();
                            let menu_for_delete = menu.clone();
                            let menu_theme = token_menu_theme.clone();
                            view! {
                                <SceneTokenMenu
                                    token_name=menu.token_name
                                    screen_x=menu.screen_x
                                    screen_y=menu.screen_y
                                    on_edit=Callback::new(move |_| {
                                        token_editor.set(Some(SceneTokenEditorDraft {
                                            scene_id: menu.scene_id.clone(),
                                            token_id: menu.token_id.clone(),
                                            name: menu.token.name.clone(),
                                            width_cells: menu.token.width_cells.to_string(),
                                            height_cells: menu.token.height_cells.to_string(),
                                        }));
                                        token_menu.set(None);
                                    })
                                    on_save_to_library=Callback::new(move |_| {
                                        let current_room_id = room_id.get_untracked();
                                        if current_room_id.is_empty() {
                                            token_menu.set(None);
                                            return;
                                        }

                                        let item = StoredTokenLibraryItem {
                                            key: token_library_key(&current_room_id, &menu_for_save.token.id),
                                            room_name: current_room_id,
                                            id: menu_for_save.token.id.clone(),
                                            name: menu_for_save.token.name.clone(),
                                            image: menu_for_save.token.image.clone(),
                                            width_cells: menu_for_save.token.width_cells,
                                            height_cells: menu_for_save.token.height_cells,
                                        };
                                        spawn_local(async move {
                                            if save_token_library_item(&item).await.is_ok() {
                                                token_library_items.update(|items| {
                                                    match items.iter_mut().find(|existing| existing.id == item.id) {
                                                        Some(existing) => *existing = item.clone(),
                                                        None => items.push(item.clone()),
                                                    }
                                                    sort_token_library_items(items);
                                                });
                                            }
                                        });
                                        token_menu.set(None);
                                    })
                                    on_delete=Callback::new(move |_| {
                                        if let Some(scene) = remove_token_from_scene(
                                            scenes,
                                            &menu_for_delete.scene_id,
                                            &menu_for_delete.token_id,
                                        ) {
                                            send_event(
                                                &ws_sender,
                                                ClientEvent::SceneUpdate(SceneUpdatePayload {
                                                    scene,
                                                    actor: username.get_untracked(),
                                                }),
                                            );
                                        }
                                        token_menu.set(None);
                                    })
                                    on_close=Callback::new(move |_| token_menu.set(None))
                                    theme=menu_theme
                                />
                            }.into_any()
                        }).unwrap_or_else(|| ().into_any())
                    }}

                    <SceneTokenEditor
                        draft=token_editor
                        on_save=Callback::new(move |value: SceneTokenEditorValue| {
                            if let Some(scene) = update_token_details(
                                scenes,
                                &value.scene_id,
                                &value.token_id,
                                &value.name,
                                value.width_cells,
                                value.height_cells,
                            ) {
                                send_event(
                                    &ws_sender,
                                    ClientEvent::SceneUpdate(SceneUpdatePayload {
                                        scene,
                                        actor: username.get_untracked(),
                                    }),
                                );
                            }
                            token_editor.set(None);
                        })
                        on_close=Callback::new(move |_| token_editor.set(None))
                        theme=token_editor_theme
                    />

                    // Cursor overlays
                    <For
                        each=move || { cursors.get().into_iter().collect::<Vec<_>>() }
                        key=|(name, _)| name.clone()
                        children=move |(name, cursor_sig)| {
                            let is_me = name == username.get();
                            let visible = {
                                let cursor_visible = cursor_sig.visible;
                                Signal::derive(move || !is_me && cursor_visible.get())
                            };
                            let cursor_x = {
                                let cwx = cursor_sig.x;
                                let cwy = cursor_sig.y;
                                Signal::derive(move || {
                                    let (sx, _) = world_to_screen(
                                        cwx.get(), cwy.get(),
                                        vm.viewport_width.get(), vm.viewport_height.get(),
                                        vm.camera_x.get(), vm.camera_y.get(), vm.zoom.get(),
                                    );
                                    sx
                                })
                            };
                            let cursor_y = {
                                let cwx = cursor_sig.x;
                                let cwy = cursor_sig.y;
                                Signal::derive(move || {
                                    let (_, sy) = world_to_screen(
                                        cwx.get(), cwy.get(),
                                        vm.viewport_width.get(), vm.viewport_height.get(),
                                        vm.camera_x.get(), vm.camera_y.get(), vm.zoom.get(),
                                    );
                                    sy
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

                    // Top status bar
                    <div style=format!(
                        "position: absolute; top: 1rem; left: 50%; transform: translateX(-50%); \
                         display: inline-flex; gap: 0.75rem; align-items: center; padding: 0.7rem 0.95rem; \
                         background: rgba(0,0,0,0.42); border: 1px solid {}; border-radius: 0.85rem; \
                         backdrop-filter: blur(10px); box-shadow: 0 12px 32px rgba(0,0,0,0.20); \
                         color: {}; z-index: 6;",
                        theme.ui_border, theme.ui_text_primary
                    )>
                        <div style="font-size: 0.95rem; font-weight: 700;">{"Scene Workspace"}</div>
                        <div style=format!("font-size: 0.8rem; color: {};", theme.ui_text_secondary)>
                            {format!("{} boards", scene_items.len())}
                        </div>
                        <div style=format!("font-size: 0.8rem; color: {};", theme.ui_text_secondary)>
                            {move || match active_scene_id.get() {
                                Some(id) => scenes.get().into_iter()
                                    .find(|s| s.id == id)
                                    .map(|s| format!("active: {}", s.name))
                                    .unwrap_or_else(|| "active: none".to_string()),
                                None => "active: none".to_string(),
                            }}
                        </div>
                    </div>

                    <Show when=move || show_workspace_hint.get()>
                        <WorkspaceHintCard
                            zoom_percent=Signal::derive(move || (vm.zoom.get() * 100.0).round() as i32)
                            on_close=Callback::new(move |_| show_workspace_hint.set(false))
                            theme=workspace_hint_theme.clone()
                        />
                    </Show>

                </div>
            }.into_any()
        }}
    }
}
