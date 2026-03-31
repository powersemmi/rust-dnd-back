use super::board_note_helpers::{
    apply_local_note_delete, apply_local_note_upsert, board_note_body_height, board_note_meta,
    board_note_title_font_size_pt, clear_board_note_editor_state, collect_board_notes,
    commit_board_note_draft, current_time_ms, find_matching_note, find_note_by_ref,
    note_matches, persist_note_delete, persist_note_upsert,
};
use super::board_toolbar::{
    AttentionPingAnimation, BoardToolbar, PointerTrailOverlay, RulerOverlay,
};
use super::interaction_state::{
    BOARD_NOTE_COLORS, BOARD_NOTE_DOUBLE_CLICK_MS, BOARD_NOTE_FONT_SIZE_STEP_PT,
    BOARD_NOTE_MAX_FONT_SIZE_PT, BOARD_NOTE_MAX_HEIGHT_PX, BOARD_NOTE_MAX_WIDTH_PX,
    BOARD_NOTE_MIN_FONT_SIZE_PT, BOARD_NOTE_MIN_HEIGHT_PX, BOARD_NOTE_MIN_WIDTH_PX,
    BOARD_NOTE_RESIZE_HANDLE_PX, BoardNoteClickState, BoardNoteEditorDraft, BoardNoteDragState,
    BoardNoteResizeState, BoardNoteSelection, TOKEN_DRAG_EPSILON_CELLS, TokenMenuState,
};
use super::model::{
    BOARD_HANDLE_HEIGHT_PX, BoardTool, DRAG_EPSILON_PX, WORKSPACE_GRID_STEP_PX, ZOOM_STEP,
    board_background, centered_token_offset, clamp_zoom, grid_line_width_screen, ruler_distance,
    scene_allows_token_interaction, scene_shows_contents, selection_box, should_broadcast_cursor,
    snap_token_position_to_grid, token_position_from_world, token_rect, world_to_screen,
};
use super::scene_geometry::{
    board_note_hit, build_scene_layouts, clamp_to_layout, place_library_token,
    point_inside_board, point_inside_board_note_content, point_inside_handle,
    remove_token_from_scene, send_event, snap_scene_position, sort_token_library_items, token_hit,
    update_scene_position, update_token_details, update_token_position, viewport_local_point,
    viewport_size,
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
use crate::components::notes::model::{
    BOARD_NOTE_DRAG_MIME, can_delete_note, can_edit_note, note_heading_and_body,
    render_note_html,
};
use crate::components::websocket::{
    CursorSignals, FileTransferState, StoredTokenLibraryItem, WsSender, save_token_library_item,
    token_library_key,
};
use crate::config;
use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{
    AttentionPingPayload, ClientEvent, DirectMessagePayload, NoteBoardPosition, NotePayload,
    NoteVisibility, Scene, SceneUpdatePayload, TokenMovePayload,
};
#[cfg(test)]
use shared::events::NoteBoardStyle;
use web_sys::{DragEvent, MouseEvent, WheelEvent};


// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn SceneBoard(
    room_id: ReadSignal<String>,
    #[prop(into)] scenes: RwSignal<Vec<Scene>>,
    #[prop(into)] active_scene_id: RwSignal<Option<String>>,
    #[prop(into)] public_notes: RwSignal<Vec<NotePayload>>,
    #[prop(into)] private_notes: RwSignal<Vec<NotePayload>>,
    #[prop(into)] direct_notes: RwSignal<Vec<NotePayload>>,
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
    /// Usernames of remote users who have activated the pointer tool.
    #[prop(into)] board_pointers: RwSignal<std::collections::HashSet<String>>,
    #[prop(into)] attention_pings: RwSignal<Vec<AttentionPingPayload>>,
    /// Received direct messages; available for a future DM panel component.
    #[allow(unused_variables)]
    #[prop(into)] direct_messages: RwSignal<Vec<DirectMessagePayload>>,
) -> impl IntoView {
    let i18n = use_i18n();
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
    let board_note_body_ref = NodeRef::<html::Textarea>::new();
    let config = StoredValue::new(config);
    let drag_did_move = RwSignal::new(false);
    let token_drag_did_move = RwSignal::new(false);
    let loaded_room_id = RwSignal::new(initial_room_id);
    let token_menu = RwSignal::new(None::<TokenMenuState>);
    let token_editor = RwSignal::new(None::<SceneTokenEditorDraft>);
    let selected_board_note = RwSignal::new(None::<BoardNoteSelection>);
    let board_note_editor = RwSignal::new(None::<BoardNoteEditorDraft>);
    let board_note_editor_error = RwSignal::new(None::<String>);
    let board_note_drag = RwSignal::new(None::<BoardNoteDragState>);
    let board_note_resize = RwSignal::new(None::<BoardNoteResizeState>);
    let board_note_drag_did_move = RwSignal::new(false);
    let board_note_resize_did_move = RwSignal::new(false);
    let board_note_last_click = RwSignal::new(None::<BoardNoteClickState>);
    let board_note_focus_request = RwSignal::new(None::<BoardNoteSelection>);

    // Per-user trail accumulation: cursor world positions for users in pointer mode.
    // Keyed by username; value is world (x, y) points capped at 64 entries.
    // Updated by a reactive Effect watching cursors + board_pointers.
    let pointer_trails: RwSignal<std::collections::HashMap<String, Vec<(f64, f64)>>> =
        RwSignal::new(std::collections::HashMap::new());

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
        selected_board_note.set(None);
        clear_board_note_editor_state(
            board_note_editor,
            board_note_editor_error,
            board_note_focus_request,
        );
        board_note_drag.set(None);
        board_note_resize.set(None);
        board_note_last_click.set(None);
    });

    Effect::new(move |_| {
        let Some(request) = board_note_focus_request.get() else {
            return;
        };
        let Some(editor) = board_note_editor.get() else {
            board_note_focus_request.set(None);
            return;
        };
        if editor.note_id != request.note_id || editor.visibility != request.visibility {
            board_note_focus_request.set(None);
            return;
        }
        let Some(textarea) = board_note_body_ref.get() else {
            return;
        };
        let _ = textarea.focus();
        board_note_focus_request.set(None);
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

    // Broadcast BoardPointer toggle events when the tool is activated or deactivated.
    // Only ONE event per toggle — no per-move data; trail is derived locally from cursors.
    {
        Effect::new(move |was_pointer: Option<bool>| {
            let is_pointer = vm.active_tool.get() == BoardTool::Pointer;
            let was_pointer = was_pointer.unwrap_or(false);
            if is_pointer != was_pointer {
                let event = ClientEvent::BoardPointer(shared::events::BoardPointerPayload {
                    username: username.get_untracked(),
                    active: is_pointer,
                });
                if let Some(sender) = ws_sender.get_untracked() {
                    let _ = sender.try_send_event(event);
                }
            }
            is_pointer
        });
    }

    // Accumulate cursor world positions into per-user trails for all active pointer users.
    // Runs whenever any cursor position changes OR the set of pointer users changes.
    {
        Effect::new(move |_| {
            let active_users = board_pointers.get();
            // Collect current positions BEFORE calling update() to track reactive deps properly.
            let positions: Vec<(String, f64, f64)> = active_users
                .iter()
                .filter_map(|user| {
                    cursors
                        .get()
                        .get(user)
                        .map(|c| (user.clone(), c.x.get(), c.y.get()))
                })
                .collect();
            pointer_trails.update(|trails| {
                // Remove trails for users who left pointer mode.
                trails.retain(|user, _| active_users.contains(user.as_str()));
                for (user, x, y) in &positions {
                    let trail = trails.entry(user.clone()).or_default();
                    let changed = trail
                        .last()
                        .map(|(px, py)| (*px - x).abs() + (*py - y).abs() > 0.5)
                        .unwrap_or(true);
                    if changed {
                        trail.push((*x, *y));
                        if trail.len() > 64 {
                            let excess = trail.len() - 64;
                            trail.drain(0..excess);
                        }
                    }
                }
            });
        });
    }

    // When ruler tool is deactivated, clear its measurement points.
    {
        Effect::new(move |_| {
            if vm.active_tool.get() != BoardTool::Ruler {
                vm.ruler_start.set(None);
                vm.ruler_end.set(None);
            }
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
                send_mouse_event_throttled(
                    world_x,
                    world_y,
                    current_user.clone(),
                    ws_sender,
                    config,
                );
            }

            // Pointer tool: no per-move event needed.
            // The trail is accumulated locally on each receiver from the MOUSE_EVENT stream.

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

            if let Some(drag_state) = board_note_drag.get() {
                let Some(mut note) = find_note_by_ref(
                    &drag_state.note_id,
                    &drag_state.visibility,
                    &public_notes.get_untracked(),
                    &private_notes.get_untracked(),
                    &direct_notes.get_untracked(),
                ) else {
                    return;
                };

                note.board_position = Some(NoteBoardPosition {
                    world_x: world_x - drag_state.pointer_offset_x,
                    world_y: world_y - drag_state.pointer_offset_y,
                });
                note.updated_at_ms = current_time_ms();
                if let Some(position) = note.board_position.as_ref() {
                    board_note_drag_did_move.set(
                        (position.world_x - drag_state.start_note_x).abs() > DRAG_EPSILON_PX
                            || (position.world_y - drag_state.start_note_y).abs() > DRAG_EPSILON_PX,
                    );
                }
                apply_local_note_upsert(public_notes, private_notes, direct_notes, note);
                return;
            }

            if let Some(resize_state) = board_note_resize.get() {
                let Some(mut note) = find_note_by_ref(
                    &resize_state.note_id,
                    &resize_state.visibility,
                    &public_notes.get_untracked(),
                    &private_notes.get_untracked(),
                    &direct_notes.get_untracked(),
                ) else {
                    return;
                };

                let width_px = (resize_state.start_width_px
                    + (world_x - resize_state.start_world_x))
                    .clamp(BOARD_NOTE_MIN_WIDTH_PX, BOARD_NOTE_MAX_WIDTH_PX);
                let height_px = (resize_state.start_height_px
                    + (world_y - resize_state.start_world_y))
                    .clamp(BOARD_NOTE_MIN_HEIGHT_PX, BOARD_NOTE_MAX_HEIGHT_PX);
                note.board_style.width_px = width_px;
                note.board_style.height_px = height_px;
                note.updated_at_ms = current_time_ms();
                board_note_resize_did_move.set(
                    (width_px - resize_state.start_width_px).abs() > DRAG_EPSILON_PX
                        || (height_px - resize_state.start_height_px).abs() > DRAG_EPSILON_PX,
                );
                apply_local_note_upsert(public_notes, private_notes, direct_notes, note);
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

            if let Some(drag_state) = board_note_drag.get_untracked()
                && board_note_drag_did_move.get_untracked()
                && let Some(note) = find_note_by_ref(
                    &drag_state.note_id,
                    &drag_state.visibility,
                    &public_notes.get_untracked(),
                    &private_notes.get_untracked(),
                    &direct_notes.get_untracked(),
                )
            {
                persist_note_upsert(&ws_sender, &room_id, &username, note);
            }

            if let Some(resize_state) = board_note_resize.get_untracked()
                && board_note_resize_did_move.get_untracked()
                && let Some(note) = find_note_by_ref(
                    &resize_state.note_id,
                    &resize_state.visibility,
                    &public_notes.get_untracked(),
                    &private_notes.get_untracked(),
                    &direct_notes.get_untracked(),
                )
            {
                persist_note_upsert(&ws_sender, &room_id, &username, note);
            }

            vm.end_scene_drag();
            vm.end_token_drag();
            board_note_drag.set(None);
            board_note_resize.set(None);
            dragging_library_token_id.set(None);
            drag_did_move.set(false);
            token_drag_did_move.set(false);
            board_note_drag_did_move.set(false);
            board_note_resize_did_move.set(false);
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
        selected_board_note.set(None);
        clear_board_note_editor_state(
            board_note_editor,
            board_note_editor_error,
            board_note_focus_request,
        );
        board_note_drag.set(None);
        board_note_resize.set(None);
        board_note_last_click.set(None);
    });

    Effect::new(move |_| {
        let current_notes = collect_board_notes(
            &public_notes.get(),
            &private_notes.get(),
            &direct_notes.get(),
        );
        let selected = selected_board_note.get();
        if let Some(selected) = selected
            && !current_notes
                .iter()
                .any(|note| note_matches(note, &selected.note_id, &selected.visibility))
        {
            selected_board_note.set(None);
            clear_board_note_editor_state(
                board_note_editor,
                board_note_editor_error,
                board_note_focus_request,
            );
            board_note_drag.set(None);
            board_note_resize.set(None);
            board_note_last_click.set(None);
        }
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
            let toolbar_theme_main = theme.clone();
            let toolbar_theme_pointer = theme.clone();
            let toolbar_theme_ping = theme.clone();
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
                    on:dragover=move |event: DragEvent| {
                        event.prevent_default();
                        if let Some(data_transfer) = event.data_transfer() {
                            data_transfer.set_drop_effect("move");
                        }
                    }
                    on:drop=move |event: DragEvent| {
                        event.prevent_default();
                        let Some(data_transfer) = event.data_transfer() else {
                            return;
                        };
                        let Ok(raw_payload) = data_transfer.get_data(BOARD_NOTE_DRAG_MIME) else {
                            return;
                        };
                        let Ok(drag_note) = serde_json::from_str::<NotePayload>(&raw_payload) else {
                            return;
                        };
                        if !can_edit_note(&drag_note, &username.get_untracked()) {
                            return;
                        }
                        let Some((local_x, local_y)) =
                            viewport_local_point(&viewport_ref, event.client_x(), event.client_y())
                        else {
                            return;
                        };
                        let (world_x, world_y) = super::model::screen_to_world(
                            local_x,
                            local_y,
                            vm.viewport_width.get_untracked(),
                            vm.viewport_height.get_untracked(),
                            vm.camera_x.get_untracked(),
                            vm.camera_y.get_untracked(),
                            vm.zoom.get_untracked(),
                        );
                        let Some(mut updated_note) = find_matching_note(
                            &drag_note,
                            &public_notes.get_untracked(),
                            &private_notes.get_untracked(),
                            &direct_notes.get_untracked(),
                        ) else {
                            return;
                        };
                        updated_note.board_position = Some(NoteBoardPosition { world_x, world_y });
                        updated_note.updated_at_ms = current_time_ms();
                        let dropped_note_selection = BoardNoteSelection {
                            note_id: updated_note.id.clone(),
                            visibility: updated_note.visibility.clone(),
                        };
                        apply_local_note_upsert(
                            public_notes,
                            private_notes,
                            direct_notes,
                            updated_note.clone(),
                        );
                        persist_note_upsert(&ws_sender, &room_id, &username, updated_note);

                        selected_board_note.set(Some(dropped_note_selection));
                        clear_board_note_editor_state(
                            board_note_editor,
                            board_note_editor_error,
                            board_note_focus_request,
                        );
                    }
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

                                // Attention ping: Alt+LMB broadcasts a ping to all users.
                                if event.alt_key() {
                                    event.prevent_default();
                                    let ping = ClientEvent::AttentionPing(
                                        shared::events::AttentionPingPayload {
                                            username: username.get_untracked(),
                                            position: shared::events::WorldPoint {
                                                x: world_x,
                                                y: world_y,
                                            },
                                        },
                                    );
                                    if let Some(sender) = ws_sender.get_untracked() {
                                        let _ = sender.try_send_event(ping);
                                    }
                                    return;
                                }

                                // Ruler tool: clicks set start/end measurement points.
                                if vm.active_tool.get_untracked() == BoardTool::Ruler {
                                    event.prevent_default();
                                    match (
                                        vm.ruler_start.get_untracked(),
                                        vm.ruler_end.get_untracked(),
                                    ) {
                                        (None, _) | (Some(_), Some(_)) => {
                                            // First click or reset after full measurement.
                                            vm.ruler_start.set(Some((world_x, world_y)));
                                            vm.ruler_end.set(None);
                                        }
                                        (Some(_), None) => {
                                            // Second click: complete the measurement.
                                            vm.ruler_end.set(Some((world_x, world_y)));
                                        }
                                    }
                                    return;
                                }

                                let board_notes = collect_board_notes(
                                    &public_notes.get_untracked(),
                                    &private_notes.get_untracked(),
                                    &direct_notes.get_untracked(),
                                );
                                if let Some(note) = board_note_hit(&board_notes, world_x, world_y) {
                                    if let Some(draft) = board_note_editor.get_untracked()
                                        && (draft.note_id != note.id
                                            || draft.visibility != note.visibility)
                                    {
                                        if !commit_board_note_draft(
                                            &draft,
                                            board_note_editor_error,
                                            public_notes,
                                            private_notes,
                                            direct_notes,
                                            &ws_sender,
                                            &room_id,
                                            &username,
                                        ) {
                                            return;
                                        }
                                        clear_board_note_editor_state(
                                            board_note_editor,
                                            board_note_editor_error,
                                            board_note_focus_request,
                                        );
                                    }
                                    event.prevent_default();
                                    vm.is_selecting.set(false);
                                    vm.end_scene_drag();
                                    vm.end_token_drag();
                                    dragging_library_token_id.set(None);
                                    let note_selection = BoardNoteSelection {
                                        note_id: note.id.clone(),
                                        visibility: note.visibility.clone(),
                                    };
                                    selected_board_note.set(Some(note_selection));
                                    if event.button() != 0 || !can_edit_note(&note, &username.get_untracked()) {
                                        return;
                                    }
                                    if board_note_editor
                                        .get_untracked()
                                        .as_ref()
                                        .is_some_and(|draft| {
                                            draft.note_id == note.id
                                                && draft.visibility == note.visibility
                                        })
                                    {
                                        return;
                                    }
                                    let Some(position) = note.board_position.as_ref() else {
                                        return;
                                    };
                                    board_note_drag.set(Some(BoardNoteDragState {
                                        note_id: note.id.clone(),
                                        visibility: note.visibility.clone(),
                                        pointer_offset_x: world_x - position.world_x,
                                        pointer_offset_y: world_y - position.world_y,
                                        start_note_x: position.world_x,
                                        start_note_y: position.world_y,
                                    }));
                                    board_note_drag_did_move.set(false);
                                    return;
                                }
                                if let Some(draft) = board_note_editor.get_untracked() {
                                    if !commit_board_note_draft(
                                        &draft,
                                        board_note_editor_error,
                                        public_notes,
                                        private_notes,
                                        direct_notes,
                                        &ws_sender,
                                        &room_id,
                                        &username,
                                    ) {
                                        return;
                                    }
                                    clear_board_note_editor_state(
                                        board_note_editor,
                                        board_note_editor_error,
                                        board_note_focus_request,
                                    );
                                }
                                selected_board_note.set(None);
                                clear_board_note_editor_state(
                                    board_note_editor,
                                    board_note_editor_error,
                                    board_note_focus_request,
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

                        {collect_board_notes(
                            &public_notes.get(),
                            &private_notes.get(),
                            &direct_notes.get(),
                        )
                        .into_iter()
                        .map(|note| {
                            let Some(position) = note.board_position.clone() else {
                                return ().into_any();
                            };
                            let can_drag = can_edit_note(&note, &username.get_untracked());
                            let (note_title_for_display, note_body_for_display) =
                                note_heading_and_body(&note.body);
                            let rendered_html = render_note_html(&note_body_for_display);
                            let note_position = position.clone();
                            let note_style = note.board_style.clone();
                            let note_style_color = note.board_style.color.clone();
                            let note_id_for_style = note.id.clone();
                            let note_visibility_for_style = note.visibility.clone();
                            let note_id_for_selection = note.id.clone();
                            let note_visibility_for_selection = note.visibility.clone();
                            let note_id_for_body = note.id.clone();
                            let note_visibility_for_body = note.visibility.clone();
                            let note_board_style_for_body = note.board_style.clone();
                            let note_id_for_resize = note.id.clone();
                            let note_visibility_for_resize = note.visibility.clone();
                            let note_id_for_container = note.id.clone();
                            let note_visibility_for_container = note.visibility.clone();
                            let note_for_select = BoardNoteSelection {
                                note_id: note.id.clone(),
                                visibility: note.visibility.clone(),
                            };
                            let note_for_resize = note.clone();
                            let open_editor_note_id = note.id.clone();
                            let open_editor_note_visibility = note.visibility.clone();
                            let open_editor_note_body = note.body.clone();
                            view! {
                                <article
                                    on:mousedown=move |event: MouseEvent| {
                                        event.stop_propagation();
                                        selected_board_note.set(Some(note_for_select.clone()));
                                        token_menu.set(None);
                                        if event.button() != 0 || !can_drag {
                                            return;
                                        }
                                        let Some((local_x, local_y)) = viewport_local_point(
                                            &viewport_ref,
                                            event.client_x(),
                                            event.client_y(),
                                        ) else {
                                            return;
                                        };
                                        let (world_x, world_y) = super::model::screen_to_world(
                                            local_x,
                                            local_y,
                                            vm.viewport_width.get_untracked(),
                                            vm.viewport_height.get_untracked(),
                                            vm.camera_x.get_untracked(),
                                            vm.camera_y.get_untracked(),
                                            vm.zoom.get_untracked(),
                                        );
                                        let now_ms = current_time_ms();
                                        let is_double_click = board_note_last_click
                                            .get_untracked()
                                            .as_ref()
                                            .is_some_and(|last| {
                                                last.note_id == open_editor_note_id
                                                    && last.visibility == open_editor_note_visibility
                                                    && now_ms - last.at_ms <= BOARD_NOTE_DOUBLE_CLICK_MS
                                            });
                                        if !is_double_click
                                            || !point_inside_board_note_content(&note, world_x, world_y)
                                        {
                                            board_note_last_click.set(Some(BoardNoteClickState {
                                                note_id: open_editor_note_id.clone(),
                                                visibility: open_editor_note_visibility.clone(),
                                                at_ms: now_ms,
                                            }));
                                            return;
                                        }
                                        board_note_last_click.set(None);
                                        board_note_editor.set(Some(BoardNoteEditorDraft {
                                            note_id: open_editor_note_id.clone(),
                                            visibility: open_editor_note_visibility.clone(),
                                            body: open_editor_note_body.clone(),
                                        }));
                                        board_note_focus_request.set(Some(BoardNoteSelection {
                                            note_id: open_editor_note_id.clone(),
                                            visibility: open_editor_note_visibility.clone(),
                                        }));
                                        board_note_editor_error.set(None);
                                        selected_board_note.set(Some(BoardNoteSelection {
                                            note_id: open_editor_note_id.clone(),
                                            visibility: open_editor_note_visibility.clone(),
                                        }));
                                    }
                                    on:wheel=move |event: WheelEvent| {
                                        event.stop_propagation();
                                    }
                                    style=move || {
                                        let is_selected = selected_board_note
                                            .get()
                                            .as_ref()
                                            .is_some_and(|selected| {
                                                selected.note_id == note_id_for_selection
                                                    && selected.visibility == note_visibility_for_selection
                                            });
                                        let is_editing = board_note_editor
                                            .get()
                                            .as_ref()
                                            .is_some_and(|draft| {
                                                draft.note_id == note_id_for_container
                                                    && draft.visibility == note_visibility_for_container
                                            });
                                        format!(
                                            "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; \
                                         background: {}; border: 1px solid {}; border-radius: 0.9rem; \
                                         box-shadow: {}; color: #2f240d; z-index: {}; overflow: hidden; user-select: {}; \
                                         pointer-events: auto;",
                                            note_position.world_x,
                                            note_position.world_y,
                                            note_style.width_px,
                                            note_style.height_px,
                                            note_style_color,
                                            if is_selected {
                                                &theme.ui_button_primary
                                            } else {
                                                "rgba(123,95,32,0.35)"
                                            },
                                            if is_selected {
                                                "0 0 0 2px rgba(37,99,235,0.18), 0 16px 28px rgba(0,0,0,0.22)"
                                            } else {
                                                "0 12px 24px rgba(0,0,0,0.18)"
                                            },
                                            if is_selected { 8 } else { 6 },
                                            if is_editing { "text" } else { "none" }
                                        )
                                    }
                                >
                                    {move || {
                                        let is_selected = selected_board_note
                                            .get()
                                            .as_ref()
                                            .is_some_and(|selected| {
                                                selected.note_id == note_id_for_style
                                                    && selected.visibility == note_visibility_for_style
                                            });
                                        if !is_selected {
                                            return ().into_any();
                                        }

                                        view! {
                                            <>
                                                <div style="position: absolute; inset: -0.45rem; border: 0.12rem solid #6c7cff; border-radius: 1rem; pointer-events: none;" />
                                                <div style="position: absolute; top: -0.62rem; left: -0.62rem; width: 0.6rem; height: 0.6rem; background: white; border: 0.08rem solid #94a3b8; border-radius: 999px; pointer-events: none;" />
                                                <div style="position: absolute; top: -0.62rem; left: calc(50% - 0.3rem); width: 0.6rem; height: 0.6rem; background: #6c7cff; border: 0.08rem solid white; border-radius: 999px; pointer-events: none;" />
                                                <div style="position: absolute; top: -0.62rem; right: -0.62rem; width: 0.6rem; height: 0.6rem; background: white; border: 0.08rem solid #94a3b8; border-radius: 999px; pointer-events: none;" />
                                                <div style="position: absolute; top: calc(50% - 0.3rem); left: -0.95rem; width: 0.45rem; height: 0.45rem; background: #6c7cff; border: 0.08rem solid white; border-radius: 999px; pointer-events: none;" />
                                                <div style="position: absolute; top: calc(50% - 0.3rem); right: -0.95rem; width: 0.45rem; height: 0.45rem; background: #6c7cff; border: 0.08rem solid white; border-radius: 999px; pointer-events: none;" />
                                                <div style="position: absolute; bottom: -0.62rem; left: -0.62rem; width: 0.6rem; height: 0.6rem; background: white; border: 0.08rem solid #94a3b8; border-radius: 999px; pointer-events: none;" />
                                                <div style="position: absolute; bottom: -0.95rem; left: calc(50% - 0.225rem); width: 0.45rem; height: 0.45rem; background: #6c7cff; border: 0.08rem solid white; border-radius: 999px; pointer-events: none;" />
                                                <div style="position: absolute; bottom: -0.62rem; right: -0.62rem; width: 0.6rem; height: 0.6rem; background: white; border: 0.08rem solid #94a3b8; border-radius: 999px; pointer-events: none;" />
                                            </>
                                        }.into_any()
                                    }}

                                    {move || {
                                        let is_editing = board_note_editor
                                            .get()
                                            .as_ref()
                                            .is_some_and(|draft| {
                                                draft.note_id == note_id_for_body
                                                    && draft.visibility == note_visibility_for_body
                                            });
                                        let is_selected = selected_board_note
                                            .get()
                                            .as_ref()
                                            .is_some_and(|selected| {
                                                selected.note_id == note_id_for_body
                                                    && selected.visibility == note_visibility_for_body
                                            });
                                        let body_height_px =
                                            board_note_body_height(&note_board_style_for_body, is_selected, is_editing);
                                        let body_font_size_pt = note_board_style_for_body.font_size_pt;
                                        let title_font_size_pt =
                                            board_note_title_font_size_pt(body_font_size_pt);
                                        let editor_note_id = note_id_for_body.clone();
                                        let editor_note_visibility = note_visibility_for_body.clone();
                                        let editor_note_id_for_body = editor_note_id.clone();
                                        let editor_note_visibility_for_body =
                                            editor_note_visibility.clone();
                                        let editor_note_id_for_body_input = editor_note_id.clone();
                                        let editor_note_visibility_for_body_input =
                                            editor_note_visibility.clone();
                                        if is_editing {
                                            view! {
                                                <div style="position: relative; z-index: 1; display: flex; flex-direction: column; gap: 0.5rem; height: 100%; padding: 0.85rem 0.95rem;">
                                                    <textarea
                                                        node_ref=board_note_body_ref
                                                        on:mousedown=move |event: MouseEvent| {
                                                            event.stop_propagation();
                                                        }
                                                        on:mouseup=move |event: MouseEvent| {
                                                            event.stop_propagation();
                                                        }
                                                        on:click=move |event: MouseEvent| {
                                                            event.stop_propagation();
                                                        }
                                                        prop:value=move || {
                                                            board_note_editor
                                                                .get()
                                                                .filter(|draft| {
                                                                    draft.note_id == editor_note_id_for_body
                                                                        && draft.visibility == editor_note_visibility_for_body
                                                                })
                                                                .map(|draft| draft.body)
                                                                .unwrap_or_default()
                                                        }
                                                        on:input=move |event| {
                                                            let value = event_target_value(&event);
                                                            board_note_editor.update(|draft| {
                                                                if let Some(draft) = draft.as_mut()
                                                                    && draft.note_id == editor_note_id_for_body_input
                                                                    && draft.visibility == editor_note_visibility_for_body_input
                                                                {
                                                                    draft.body = value.clone();
                                                                }
                                                            });
                                                        }
                                                        style=format!(
                                                            "width: 100%; min-height: {:.2}px; height: {:.2}px; resize: none; padding: 0.55rem; border: 1px solid rgba(0,0,0,0.12); border-radius: 0.45rem; background: rgba(255,255,255,0.55); box-sizing: border-box; font-family: inherit; line-height: 1.35; font-size: {:.2}pt;",
                                                            body_height_px,
                                                            body_height_px,
                                                            body_font_size_pt
                                                        )
                                                    ></textarea>
                                                    {move || {
                                                        board_note_editor_error
                                                            .get()
                                                            .map(|error| {
                                                                view! {
                                                                    <div style="font-size: 0.72rem; color: #991b1b;">{error}</div>
                                                                }
                                                            })
                                                    }}
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div
                                                    style="height: 100%; padding: 1rem 1.1rem; display: flex; align-items: stretch; justify-content: flex-start; text-align: left; cursor: text;"
                                                >
                                                    <div style=format!(
                                                        "display: flex; flex-direction: column; align-items: stretch; justify-content: flex-start; gap: 0.8rem; width: 100%; min-height: {:.2}px; max-height: {:.2}px; overflow: auto;",
                                                        body_height_px,
                                                        body_height_px
                                                    )>
                                                        {if !note_title_for_display.trim().is_empty() {
                                                            view! {
                                                                <div style=format!("width: 100%; font-size: {:.2}pt; line-height: 1.18; font-weight: 700; word-break: break-word; white-space: pre-wrap; text-align: left;", title_font_size_pt)>
                                                                    {note_title_for_display.clone()}
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                        {if !note_body_for_display.trim().is_empty() {
                                                            view! {
                                                                <div
                                                                    inner_html=rendered_html.clone()
                                                                    style=format!("font-size: {:.2}pt; line-height: 1.35; word-break: break-word; width: 100%;", body_font_size_pt)
                                                                ></div>
                                                            }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                    </div>
                                                </div>
                                            }.into_any()
                                        }
                                    }}

                                    {move || {
                                        let is_selected = selected_board_note
                                            .get()
                                            .as_ref()
                                            .is_some_and(|selected| {
                                                selected.note_id == note_id_for_resize
                                                    && selected.visibility == note_visibility_for_resize
                                            });
                                        if !is_selected || !can_drag {
                                            return ().into_any();
                                        }
                                        let resize_note_id = note_for_resize.id.clone();
                                        let resize_note_visibility = note_for_resize.visibility.clone();
                                        let resize_note_width = note_for_resize.board_style.width_px;
                                        let resize_note_height = note_for_resize.board_style.height_px;

                                        view! {
                                            <button
                                                on:mousedown=move |event: MouseEvent| {
                                                    event.stop_propagation();
                                                    if event.button() != 0 {
                                                        return;
                                                    }
                                                    let Some((local_x, local_y)) = viewport_local_point(
                                                        &viewport_ref,
                                                        event.client_x(),
                                                        event.client_y(),
                                                    ) else {
                                                        return;
                                                    };
                                                    let (world_x, world_y) = super::model::screen_to_world(
                                                        local_x,
                                                        local_y,
                                                        vm.viewport_width.get_untracked(),
                                                        vm.viewport_height.get_untracked(),
                                                        vm.camera_x.get_untracked(),
                                                        vm.camera_y.get_untracked(),
                                                        vm.zoom.get_untracked(),
                                                    );
                                                    board_note_resize.set(Some(BoardNoteResizeState {
                                                        note_id: resize_note_id.clone(),
                                                        visibility: resize_note_visibility.clone(),
                                                        start_world_x: world_x,
                                                        start_world_y: world_y,
                                                        start_width_px: resize_note_width,
                                                        start_height_px: resize_note_height,
                                                    }));
                                                    board_note_resize_did_move.set(false);
                                                }
                                                style=format!(
                                                    "position: absolute; right: 0.35rem; bottom: 0.35rem; width: {:.2}px; height: {:.2}px; border: none; border-radius: 0.35rem; background: rgba(0,0,0,0.18); cursor: nwse-resize;",
                                                    BOARD_NOTE_RESIZE_HANDLE_PX,
                                                    BOARD_NOTE_RESIZE_HANDLE_PX
                                                )
                                            />
                                        }.into_any()
                                    }}
                                </article>
                            }
                            .into_any()
                        })
                        .collect_view()}

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
                        let Some(selected) = selected_board_note.get() else {
                            return ().into_any();
                        };
                        let Some(note) = find_note_by_ref(
                            &selected.note_id,
                            &selected.visibility,
                            &public_notes.get(),
                            &private_notes.get(),
                            &direct_notes.get(),
                        ) else {
                            return ().into_any();
                        };
                        let Some(position) = note.board_position.clone() else {
                            return ().into_any();
                        };
                        let current_username = username.get_untracked();
                        let can_drag = can_edit_note(&note, &current_username);
                        let can_delete = can_delete_note(&note, &current_username);
                        let can_delete_direct_for_recipient = can_delete
                            && !can_drag
                            && matches!(note.visibility, NoteVisibility::Direct(_));
                        let is_editing = board_note_editor
                            .get()
                            .as_ref()
                            .is_some_and(|draft| {
                                draft.note_id == note.id && draft.visibility == note.visibility
                            });
                        let note_meta = board_note_meta(&note, &username.get_untracked());
                        let drag_note_id = note.id.clone();
                        let drag_note_visibility = note.visibility.clone();
                        let drag_note_position = position.clone();
                        let note_for_unpin = note.clone();
                        let unpin_note_id = note_for_unpin.id.clone();
                        let unpin_note_visibility = note_for_unpin.visibility.clone();
                        let note_color_for_toolbar = note.board_style.color.clone();
                        let note_font_size_pt = note.board_style.font_size_pt;
                        let font_down_note_id = note.id.clone();
                        let font_down_note_visibility = note.visibility.clone();
                        let font_up_note_id = note.id.clone();
                        let font_up_note_visibility = note.visibility.clone();
                        let (screen_x, screen_y) = world_to_screen(
                            position.world_x,
                            position.world_y,
                            vm.viewport_width.get(),
                            vm.viewport_height.get(),
                            vm.camera_x.get(),
                            vm.camera_y.get(),
                            vm.zoom.get(),
                        );

                        view! {
                            <div
                                on:mousedown=move |event: MouseEvent| {
                                    event.stop_propagation();
                                    if event.button() != 0 || !can_drag {
                                        return;
                                    }
                                    if is_editing {
                                        return;
                                    }
                                    let Some((local_x, local_y)) = viewport_local_point(
                                        &viewport_ref,
                                        event.client_x(),
                                        event.client_y(),
                                    ) else {
                                        return;
                                    };
                                    let (world_x, world_y) = super::model::screen_to_world(
                                        local_x,
                                        local_y,
                                        vm.viewport_width.get_untracked(),
                                        vm.viewport_height.get_untracked(),
                                        vm.camera_x.get_untracked(),
                                        vm.camera_y.get_untracked(),
                                        vm.zoom.get_untracked(),
                                    );
                                    board_note_drag.set(Some(BoardNoteDragState {
                                        note_id: drag_note_id.clone(),
                                        visibility: drag_note_visibility.clone(),
                                        pointer_offset_x: world_x - drag_note_position.world_x,
                                        pointer_offset_y: world_y - drag_note_position.world_y,
                                        start_note_x: drag_note_position.world_x,
                                        start_note_y: drag_note_position.world_y,
                                    }));
                                    board_note_drag_did_move.set(false);
                                }
                                style=format!(
                                    "position: absolute; left: {:.2}px; top: {:.2}px; transform: translateY(-100%); \
                                     display: flex; align-items: center; gap: 0.45rem; padding: 0.45rem 0.6rem; \
                                     border-radius: 0.75rem; background: rgba(255,255,255,0.94); border: 1px solid rgba(15,23,42,0.08); \
                                     box-shadow: 0 10px 22px rgba(15,23,42,0.18); z-index: 20; color: #111827; \
                                     cursor: {}; pointer-events: auto;",
                                    screen_x,
                                    screen_y - 12.0,
                                    if can_drag && !is_editing { "move" } else { "default" },
                                )
                            >
                                <span style="font-size: 0.72rem; font-weight: 600; white-space: nowrap; color: #475569;">
                                    {note_meta}
                                </span>

                                {BOARD_NOTE_COLORS
                                    .iter()
                                    .copied()
                                    .map(|color| {
                                        let note_for_color = note.clone();
                                        view! {
                                            <button
                                                on:mousedown=move |event: MouseEvent| {
                                                    event.prevent_default();
                                                    event.stop_propagation();
                                                    if !can_drag {
                                                        return;
                                                    }
                                                    let Some(mut updated_note) = find_note_by_ref(
                                                        &note_for_color.id,
                                                        &note_for_color.visibility,
                                                        &public_notes.get_untracked(),
                                                        &private_notes.get_untracked(),
                                                        &direct_notes.get_untracked(),
                                                    ) else {
                                                        return;
                                                    };
                                                    updated_note.board_style.color = color.to_string();
                                                    updated_note.updated_at_ms = current_time_ms();
                                                    apply_local_note_upsert(
                                                        public_notes,
                                                        private_notes,
                                                        direct_notes,
                                                        updated_note.clone(),
                                                    );
                                                    persist_note_upsert(&ws_sender, &room_id, &username, updated_note);
                                                }
                                                style=format!(
                                                    "width: 1rem; height: 1rem; border-radius: 999px; border: 1px solid {}; background: {}; cursor: {}; padding: 0;",
                                                    if note_color_for_toolbar == color { "#111827" } else { "rgba(15,23,42,0.18)" },
                                                    color,
                                                    if can_drag { "pointer" } else { "default" }
                                                )
                                            />
                                        }
                                    })
                                    .collect_view()}

                                {if can_drag {
                                    view! {
                                        <>
                                            <button
                                                on:mousedown=move |event: MouseEvent| {
                                                    event.prevent_default();
                                                    event.stop_propagation();
                                                    let Some(mut updated_note) = find_note_by_ref(
                                                        &font_down_note_id,
                                                        &font_down_note_visibility,
                                                        &public_notes.get_untracked(),
                                                        &private_notes.get_untracked(),
                                                        &direct_notes.get_untracked(),
                                                    ) else {
                                                        return;
                                                    };
                                                    updated_note.board_style.font_size_pt = (
                                                        updated_note.board_style.font_size_pt
                                                            - BOARD_NOTE_FONT_SIZE_STEP_PT
                                                    )
                                                        .clamp(
                                                            BOARD_NOTE_MIN_FONT_SIZE_PT,
                                                            BOARD_NOTE_MAX_FONT_SIZE_PT,
                                                        );
                                                    updated_note.updated_at_ms = current_time_ms();
                                                    apply_local_note_upsert(
                                                        public_notes,
                                                        private_notes,
                                                        direct_notes,
                                                        updated_note.clone(),
                                                    );
                                                    persist_note_upsert(
                                                        &ws_sender,
                                                        &room_id,
                                                        &username,
                                                        updated_note,
                                                    );
                                                }
                                                style="padding: 0.28rem 0.5rem; background: rgba(148,163,184,0.16); color: #1f2937; border: none; border-radius: 0.5rem; cursor: pointer; font-size: 0.78rem; font-weight: 700;"
                                            >
                                                {"A-"}
                                            </button>
                                            <span style="font-size: 0.72rem; font-weight: 600; white-space: nowrap; color: #475569; min-width: 2.8rem; text-align: center;">
                                                {format!("{:.0}pt", note_font_size_pt)}
                                            </span>
                                            <button
                                                on:mousedown=move |event: MouseEvent| {
                                                    event.prevent_default();
                                                    event.stop_propagation();
                                                    let Some(mut updated_note) = find_note_by_ref(
                                                        &font_up_note_id,
                                                        &font_up_note_visibility,
                                                        &public_notes.get_untracked(),
                                                        &private_notes.get_untracked(),
                                                        &direct_notes.get_untracked(),
                                                    ) else {
                                                        return;
                                                    };
                                                    updated_note.board_style.font_size_pt = (
                                                        updated_note.board_style.font_size_pt
                                                            + BOARD_NOTE_FONT_SIZE_STEP_PT
                                                    )
                                                        .clamp(
                                                            BOARD_NOTE_MIN_FONT_SIZE_PT,
                                                            BOARD_NOTE_MAX_FONT_SIZE_PT,
                                                        );
                                                    updated_note.updated_at_ms = current_time_ms();
                                                    apply_local_note_upsert(
                                                        public_notes,
                                                        private_notes,
                                                        direct_notes,
                                                        updated_note.clone(),
                                                    );
                                                    persist_note_upsert(
                                                        &ws_sender,
                                                        &room_id,
                                                        &username,
                                                        updated_note,
                                                    );
                                                }
                                                style="padding: 0.28rem 0.5rem; background: rgba(148,163,184,0.16); color: #1f2937; border: none; border-radius: 0.5rem; cursor: pointer; font-size: 0.78rem; font-weight: 700;"
                                            >
                                                {"A+"}
                                            </button>
                                            <button
                                                on:mousedown=move |event: MouseEvent| {
                                                    event.prevent_default();
                                                    event.stop_propagation();
                                                    let Some(mut updated_note) = find_note_by_ref(
                                                        &unpin_note_id,
                                                        &unpin_note_visibility,
                                                        &public_notes.get_untracked(),
                                                        &private_notes.get_untracked(),
                                                        &direct_notes.get_untracked(),
                                                    ) else {
                                                        return;
                                                    };
                                                    updated_note.board_position = None;
                                                    updated_note.updated_at_ms = current_time_ms();
                                                    apply_local_note_upsert(
                                                        public_notes,
                                                        private_notes,
                                                        direct_notes,
                                                        updated_note.clone(),
                                                    );
                                                    persist_note_upsert(&ws_sender, &room_id, &username, updated_note);
                                                    selected_board_note.set(None);
                                                    clear_board_note_editor_state(
                                                        board_note_editor,
                                                        board_note_editor_error,
                                                        board_note_focus_request,
                                                    );
                                                }
                                                style="padding: 0.35rem 0.6rem; background: rgba(148,163,184,0.16); color: #1f2937; border: none; border-radius: 0.5rem; cursor: pointer; font-size: 0.72rem;"
                                            >
                                                {t!(i18n, notes.remove_from_board_button)}
                                            </button>
                                        </>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                                {if can_delete_direct_for_recipient {
                                    let note_for_delete = note.clone();
                                    view! {
                                        <button
                                            on:mousedown=move |event: MouseEvent| {
                                                event.prevent_default();
                                                event.stop_propagation();
                                                let Some(note_to_delete) = find_note_by_ref(
                                                    &note_for_delete.id,
                                                    &note_for_delete.visibility,
                                                    &public_notes.get_untracked(),
                                                    &private_notes.get_untracked(),
                                                    &direct_notes.get_untracked(),
                                                ) else {
                                                    return;
                                                };
                                                apply_local_note_delete(
                                                    public_notes,
                                                    private_notes,
                                                    direct_notes,
                                                    &note_to_delete.id,
                                                    &note_to_delete.visibility,
                                                );
                                                persist_note_delete(
                                                    &ws_sender,
                                                    &room_id,
                                                    &username,
                                                    &note_to_delete,
                                                );
                                                selected_board_note.set(None);
                                                clear_board_note_editor_state(
                                                    board_note_editor,
                                                    board_note_editor_error,
                                                    board_note_focus_request,
                                                );
                                            }
                                            style="padding: 0.35rem 0.6rem; background: #dc2626; color: #fff; border: none; border-radius: 0.5rem; cursor: pointer; font-size: 0.72rem;"
                                        >
                                            {t!(i18n, notes.delete_button)}
                                        </button>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                            </div>
                        }.into_any()
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

                    // Ruler overlay: rendered in screen space when ruler has a start point.
                    {move || {
                        let tool = vm.active_tool.get();
                        if tool != BoardTool::Ruler {
                            return ().into_any();
                        }
                        let Some(start) = vm.ruler_start.get() else {
                            return ().into_any();
                        };
                        // Determine end: either anchored or live cursor position.
                        let (end_wx, end_wy) = vm.ruler_end.get().unwrap_or_else(|| {
                            super::model::screen_to_world(
                                vm.pointer_local_x.get(),
                                vm.pointer_local_y.get(),
                                vm.viewport_width.get(),
                                vm.viewport_height.get(),
                                vm.camera_x.get(),
                                vm.camera_y.get(),
                                vm.zoom.get(),
                            )
                        });

                        // Find active scene for cell_size_feet.
                        let cell_size_feet = scenes.get()
                            .into_iter()
                            .find(|s| active_scene_id.get().as_deref() == Some(s.id.as_str()))
                            .map(|s| s.grid.cell_size_feet)
                            .unwrap_or(5);

                        let (dcells, dfeet) = ruler_distance(
                            start.0, start.1, end_wx, end_wy, cell_size_feet,
                        );

                        let cam_x = vm.camera_x.get();
                        let cam_y = vm.camera_y.get();
                        let zoom = vm.zoom.get();
                        let vw = vm.viewport_width.get();
                        let vh = vm.viewport_height.get();
                        let (ssx, ssy) = world_to_screen(start.0, start.1, vw, vh, cam_x, cam_y, zoom);
                        let (sex, sey) = world_to_screen(end_wx, end_wy, vw, vh, cam_x, cam_y, zoom);

                        view! {
                            <RulerOverlay
                                start_screen_x=ssx
                                start_screen_y=ssy
                                end_screen_x=sex
                                end_screen_y=sey
                                distance_cells=dcells
                                distance_feet=dfeet
                            />
                        }.into_any()
                    }}

                    // Remote board pointer trails: accumulated from cursor positions.
                    {move || {
                        let cam_x = vm.camera_x.get();
                        let cam_y = vm.camera_y.get();
                        let zoom = vm.zoom.get();
                        let vw = vm.viewport_width.get();
                        let vh = vm.viewport_height.get();
                        pointer_trails.get().into_iter().map(|(user, world_pts)| {
                            let pts: Vec<(f64, f64)> = world_pts
                                .iter()
                                .map(|(wx, wy)| world_to_screen(*wx, *wy, vw, vh, cam_x, cam_y, zoom))
                                .collect();
                            view! {
                                <PointerTrailOverlay
                                    username=user
                                    points=pts
                                    active=true
                                    theme=toolbar_theme_pointer.clone()
                                />
                            }
                        }).collect_view()
                    }}

                    // Attention ping animations
                    {move || {
                        attention_pings.get().iter().map(|ping| {
                            let cam_x = vm.camera_x.get();
                            let cam_y = vm.camera_y.get();
                            let zoom = vm.zoom.get();
                            let vw = vm.viewport_width.get();
                            let vh = vm.viewport_height.get();
                            let (sx, sy) = world_to_screen(
                                ping.position.x, ping.position.y, vw, vh, cam_x, cam_y, zoom
                            );
                            view! {
                                <AttentionPingAnimation
                                    screen_x=sx
                                    screen_y=sy
                                    username=ping.username.clone()
                                    theme=toolbar_theme_ping.clone()
                                />
                            }
                        }).collect_view()
                    }}

                    // Board toolbar (above workspace hint)
                    <BoardToolbar
                        active_tool=vm.active_tool
                        theme=toolbar_theme_main.clone()
                    />

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

#[cfg(test)]
mod tests {
    use super::*;

    fn note_with_position(id: &str, world_x: f64, world_y: f64) -> NotePayload {
        NotePayload {
            id: id.to_string(),
            author: "gm".to_string(),
            visibility: NoteVisibility::Public,
            title: String::new(),
            body: "body".to_string(),
            created_at_ms: 1.0,
            updated_at_ms: 1.0,
            board_position: Some(NoteBoardPosition { world_x, world_y }),
            board_style: NoteBoardStyle {
                width_px: 100.0,
                height_px: 100.0,
                font_size_pt: 14.0,
                color: "#F8EE96".to_string(),
            },
        }
    }

    #[test]
    fn board_note_hit_prefers_topmost_note() {
        let notes = vec![
            note_with_position("bottom", 10.0, 20.0),
            note_with_position("top", 10.0, 20.0),
        ];

        let hit = board_note_hit(&notes, 50.0, 70.0).expect("note should be hit");

        assert_eq!(hit.id, "top");
    }
}
