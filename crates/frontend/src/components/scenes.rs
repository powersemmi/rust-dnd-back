use crate::components::draggable_window::DraggableWindow;
use crate::components::websocket::{FileTransferStage, FileTransferState};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev;
use leptos::ev::MouseEvent;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use shared::events::{
    ClientEvent, Scene, SceneActivatePayload, SceneCreatePayload, SceneDeletePayload, SceneGrid,
    SceneUpdatePayload,
};
use uuid::Uuid;
use web_sys::{Event, HtmlInputElement};

use super::websocket::WsSender;

const MAX_SCENES_PER_ROOM: usize = 50;
const DEFAULT_COLUMNS: &str = "24";
const DEFAULT_ROWS: &str = "16";
const DEFAULT_CELL_SIZE_FEET: &str = "5";
const DEFAULT_SCENE_SPACING_PX: f32 = 720.0;
const FILE_INPUT_ACCEPT: &str = "image/png,image/jpeg,image/webp,image/gif";
const DEFAULT_BACKGROUND_SCALE: f32 = 1.0;
const DEFAULT_BACKGROUND_OFFSET_X: f32 = 0.0;
const DEFAULT_BACKGROUND_OFFSET_Y: f32 = 0.0;
const DEFAULT_BACKGROUND_ROTATION_DEG: f32 = 0.0;
const MIN_BACKGROUND_SCALE: f32 = 0.1;
const MAX_BACKGROUND_SCALE: f32 = 4.0;
const MIN_BACKGROUND_OFFSET_PX: f32 = -2000.0;
const MAX_BACKGROUND_OFFSET_PX: f32 = 2000.0;
const MIN_BACKGROUND_ROTATION_DEG: f32 = -180.0;
const MAX_BACKGROUND_ROTATION_DEG: f32 = 180.0;
const BOARD_SIDE_PADDING_PX: f64 = 220.0;
const BOARD_TOP_PADDING_PX: f64 = 180.0;
const BOARD_BOTTOM_PADDING_PX: f64 = 140.0;
const MAX_CELL_SIZE_PX: f64 = 72.0;
const MIN_CELL_SIZE_PX: f64 = 18.0;
const FIT_PREVIEW_SIDEBAR_WIDTH_PX: f64 = 560.0;
const FIT_PREVIEW_HORIZONTAL_CHROME_PX: f64 = 120.0;
const FIT_PREVIEW_VERTICAL_CHROME_PX: f64 = 140.0;

#[derive(Clone, Copy)]
struct FitPreviewLayout {
    cell_size: f64,
    board_width: f64,
    board_height: f64,
    scale: f64,
}

fn default_scene_position(scene_count: usize) -> (f32, f32) {
    (scene_count as f32 * DEFAULT_SCENE_SPACING_PX, 0.0)
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

fn board_metrics(
    columns: u16,
    rows: u16,
    viewport_width: f64,
    viewport_height: f64,
) -> (f64, f64, f64) {
    let usable_width = (viewport_width - BOARD_SIDE_PADDING_PX).max(320.0);
    let usable_height =
        (viewport_height - BOARD_TOP_PADDING_PX - BOARD_BOTTOM_PADDING_PX).max(240.0);

    let columns = f64::from(columns.max(1));
    let rows = f64::from(rows.max(1));

    let cell_size = (usable_width / columns)
        .min(usable_height / rows)
        .clamp(MIN_CELL_SIZE_PX, MAX_CELL_SIZE_PX);

    let board_width = columns * cell_size;
    let board_height = rows * cell_size;

    (cell_size, board_width, board_height)
}

fn fit_preview_layout(columns: u16, rows: u16) -> FitPreviewLayout {
    let (viewport_width, viewport_height) = viewport_size();
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

#[component]
pub fn ScenesWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] scenes: RwSignal<Vec<Scene>>,
    #[prop(into)] active_scene_id: RwSignal<Option<String>>,
    file_transfer: FileTransferState,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    let selected_scene_id = RwSignal::new(None::<String>);
    let draft_name = RwSignal::new(String::new());
    let draft_columns = RwSignal::new(DEFAULT_COLUMNS.to_string());
    let draft_rows = RwSignal::new(DEFAULT_ROWS.to_string());
    let draft_cell_size_feet = RwSignal::new(DEFAULT_CELL_SIZE_FEET.to_string());
    let draft_background = RwSignal::new(None::<shared::events::FileRef>);
    let draft_background_scale = RwSignal::new(DEFAULT_BACKGROUND_SCALE);
    let draft_background_offset_x = RwSignal::new(DEFAULT_BACKGROUND_OFFSET_X);
    let draft_background_offset_y = RwSignal::new(DEFAULT_BACKGROUND_OFFSET_Y);
    let draft_background_rotation_deg = RwSignal::new(DEFAULT_BACKGROUND_ROTATION_DEG);
    let is_background_fit_editor_open = RwSignal::new(false);
    let is_dragging_background = RwSignal::new(false);
    let background_drag_start_client_x = RwSignal::new(0i32);
    let background_drag_start_client_y = RwSignal::new(0i32);
    let background_drag_origin_offset_x = RwSignal::new(DEFAULT_BACKGROUND_OFFSET_X);
    let background_drag_origin_offset_y = RwSignal::new(DEFAULT_BACKGROUND_OFFSET_Y);
    let background_drag_preview_scale = RwSignal::new(1.0f64);
    let editor_error = RwSignal::new(None::<String>);
    let background_input_ref = NodeRef::<html::Input>::new();

    let reset_background_fit = move || {
        draft_background_scale.set(DEFAULT_BACKGROUND_SCALE);
        draft_background_offset_x.set(DEFAULT_BACKGROUND_OFFSET_X);
        draft_background_offset_y.set(DEFAULT_BACKGROUND_OFFSET_Y);
        draft_background_rotation_deg.set(DEFAULT_BACKGROUND_ROTATION_DEG);
    };

    let close_background_fit_editor = move || {
        is_background_fit_editor_open.set(false);
        is_dragging_background.set(false);
    };

    let reset_editor = move || {
        selected_scene_id.set(None);
        draft_name.set(String::new());
        draft_columns.set(DEFAULT_COLUMNS.to_string());
        draft_rows.set(DEFAULT_ROWS.to_string());
        draft_cell_size_feet.set(DEFAULT_CELL_SIZE_FEET.to_string());
        draft_background.set(None);
        reset_background_fit();
        close_background_fit_editor();
        editor_error.set(None);
    };

    let apply_scene_to_editor = move |scene: &Scene| {
        selected_scene_id.set(Some(scene.id.clone()));
        draft_name.set(scene.name.clone());
        draft_columns.set(scene.grid.columns.to_string());
        draft_rows.set(scene.grid.rows.to_string());
        draft_cell_size_feet.set(scene.grid.cell_size_feet.to_string());
        draft_background.set(scene.background.clone());
        draft_background_scale.set(scene.background_scale);
        draft_background_offset_x.set(scene.background_offset_x);
        draft_background_offset_y.set(scene.background_offset_y);
        draft_background_rotation_deg.set(scene.background_rotation_deg);
        close_background_fit_editor();
        editor_error.set(None);
    };

    let send_event = move |event: ClientEvent| {
        if let Some(sender) = ws_sender.get_untracked() {
            let _ = sender.try_send_event(event);
        }
    };

    let build_grid = move || -> Option<SceneGrid> {
        let name = draft_name.get_untracked().trim().to_string();
        if name.is_empty() {
            editor_error.set(Some(t_string!(i18n, scenes.error_empty_name).to_string()));
            return None;
        }

        let parse_field = |value: String| value.parse::<u16>().ok();
        let columns = parse_field(draft_columns.get_untracked());
        let rows = parse_field(draft_rows.get_untracked());
        let cell_size_feet = parse_field(draft_cell_size_feet.get_untracked());

        let Some(columns) = columns else {
            editor_error.set(Some(t_string!(i18n, scenes.error_invalid_grid).to_string()));
            return None;
        };
        let Some(rows) = rows else {
            editor_error.set(Some(t_string!(i18n, scenes.error_invalid_grid).to_string()));
            return None;
        };
        let Some(cell_size_feet) = cell_size_feet else {
            editor_error.set(Some(t_string!(i18n, scenes.error_invalid_grid).to_string()));
            return None;
        };

        if !(1..=200).contains(&columns)
            || !(1..=200).contains(&rows)
            || !(1..=100).contains(&cell_size_feet)
        {
            editor_error.set(Some(t_string!(i18n, scenes.error_invalid_grid).to_string()));
            return None;
        }

        editor_error.set(None);

        Some(SceneGrid {
            columns,
            rows,
            cell_size_feet,
        })
    };

    Effect::new(move |_| {
        if !is_open.get() {
            close_background_fit_editor();
        }
    });

    Effect::new(move |_| {
        let selected_id = selected_scene_id.get();
        let current_scenes = scenes.get();

        if let Some(scene_id) = selected_id {
            if let Some(scene) = current_scenes.iter().find(|scene| scene.id == scene_id) {
                draft_name.set(scene.name.clone());
                draft_columns.set(scene.grid.columns.to_string());
                draft_rows.set(scene.grid.rows.to_string());
                draft_cell_size_feet.set(scene.grid.cell_size_feet.to_string());
                draft_background.set(scene.background.clone());
                draft_background_scale.set(scene.background_scale);
                draft_background_offset_x.set(scene.background_offset_x);
                draft_background_offset_y.set(scene.background_offset_y);
                draft_background_rotation_deg.set(scene.background_rotation_deg);
                if scene.background.is_none() {
                    close_background_fit_editor();
                }
                editor_error.set(None);
            } else {
                reset_editor();
            }
        }
    });

    let create_scene = move |_| {
        if scenes.get_untracked().len() >= MAX_SCENES_PER_ROOM {
            editor_error.set(Some(t_string!(i18n, scenes.error_limit).to_string()));
            return;
        }

        let Some(grid) = build_grid() else {
            return;
        };

        let (workspace_x, workspace_y) = default_scene_position(scenes.get_untracked().len());

        let scene = Scene {
            id: Uuid::new_v4().to_string(),
            name: draft_name.get_untracked().trim().to_string(),
            grid,
            workspace_x,
            workspace_y,
            background: draft_background.get_untracked(),
            background_scale: draft_background_scale
                .get_untracked()
                .clamp(MIN_BACKGROUND_SCALE, MAX_BACKGROUND_SCALE),
            background_offset_x: draft_background_offset_x
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_offset_y: draft_background_offset_y
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_rotation_deg: draft_background_rotation_deg
                .get_untracked()
                .clamp(MIN_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_ROTATION_DEG),
        };

        send_event(ClientEvent::SceneCreate(SceneCreatePayload {
            scene,
            actor: username.get_untracked(),
        }));
        reset_editor();
    };

    let save_scene = move |_| {
        let Some(scene_id) = selected_scene_id.get_untracked() else {
            create_scene(());
            return;
        };

        let Some(grid) = build_grid() else {
            return;
        };

        let existing_scene = scenes
            .get_untracked()
            .into_iter()
            .find(|scene| scene.id == scene_id);

        let scene = Scene {
            id: scene_id,
            name: draft_name.get_untracked().trim().to_string(),
            grid,
            workspace_x: existing_scene
                .as_ref()
                .map(|scene| scene.workspace_x)
                .unwrap_or(0.0),
            workspace_y: existing_scene
                .as_ref()
                .map(|scene| scene.workspace_y)
                .unwrap_or(0.0),
            background: draft_background.get_untracked(),
            background_scale: draft_background_scale
                .get_untracked()
                .clamp(MIN_BACKGROUND_SCALE, MAX_BACKGROUND_SCALE),
            background_offset_x: draft_background_offset_x
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_offset_y: draft_background_offset_y
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_rotation_deg: draft_background_rotation_deg
                .get_untracked()
                .clamp(MIN_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_ROTATION_DEG),
        };

        send_event(ClientEvent::SceneUpdate(SceneUpdatePayload {
            scene,
            actor: username.get_untracked(),
        }));
    };

    let on_background_selected = {
        let file_transfer = file_transfer.clone();
        move |event: Event| {
            let Some(input) = event
                .target()
                .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(files) = input.files() else {
                return;
            };
            let Some(file) = files.get(0) else {
                return;
            };

            let file_transfer = file_transfer.clone();
            let username = username.get_untracked();
            let ws_sender = ws_sender.get_untracked();
            spawn_local(async move {
                match file_transfer
                    .import_browser_file(file, username, ws_sender)
                    .await
                {
                    Ok(file_ref) if file_ref.mime_type.starts_with("image/") => {
                        draft_background.set(Some(file_ref));
                        reset_background_fit();
                        editor_error.set(None);
                    }
                    Ok(_) => {
                        editor_error.set(Some("Scene background must be an image".to_string()))
                    }
                    Err(error) => editor_error.set(Some(error)),
                }
            });

            input.set_value("");
        }
    };

    let open_background_picker = move |event: MouseEvent| {
        event.prevent_default();
        event.stop_propagation();

        if let Some(input) = background_input_ref.get() {
            input.click();
        }
    };

    let stop_mouse_down = move |event: MouseEvent| {
        event.stop_propagation();
    };

    let start_background_drag = move |event: MouseEvent, preview_scale: f64| {
        event.prevent_default();
        event.stop_propagation();

        is_dragging_background.set(true);
        background_drag_start_client_x.set(event.client_x());
        background_drag_start_client_y.set(event.client_y());
        background_drag_origin_offset_x.set(draft_background_offset_x.get_untracked());
        background_drag_origin_offset_y.set(draft_background_offset_y.get_untracked());
        background_drag_preview_scale.set(preview_scale.max(0.01));
    };

    Effect::new(move |_| {
        let handle_mousemove = window_event_listener(ev::mousemove, move |event: MouseEvent| {
            if !is_dragging_background.get() {
                return;
            }

            let scale = background_drag_preview_scale.get().max(0.01) as f32;
            let delta_x = (event.client_x() - background_drag_start_client_x.get()) as f32 / scale;
            let delta_y = (event.client_y() - background_drag_start_client_y.get()) as f32 / scale;

            draft_background_offset_x.set(
                (background_drag_origin_offset_x.get() + delta_x)
                    .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            );
            draft_background_offset_y.set(
                (background_drag_origin_offset_y.get() + delta_y)
                    .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            );
        });

        let handle_mouseup = window_event_listener(ev::mouseup, move |_event: MouseEvent| {
            is_dragging_background.set(false);
        });

        on_cleanup(move || {
            drop(handle_mousemove);
            drop(handle_mouseup);
        });
    });

    view! {
        <>
            <DraggableWindow
                is_open=is_open
                title=move || t_string!(i18n, scenes.title)
                initial_x=220
                initial_y=120
                initial_width=620
                initial_height=560
                min_width=420
                min_height=320
                is_active=is_active
                on_focus=on_focus.unwrap_or_else(|| Callback::new(|_| {}))
                theme=theme.clone()
            >
                <div style="display: flex; flex: 1; min-height: 0;">
                <div
                    style=format!(
                        "width: 48%; border-right: 0.0625rem solid {}; padding: 1rem; overflow-y: auto;",
                        theme.ui_border
                    )
                >
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem;">
                        <h4 style=format!("margin: 0; color: {};", theme.ui_text_primary)>
                            {move || t_string!(i18n, scenes.list_title)}
                        </h4>
                        <button
                            on:click=move |_| reset_editor()
                            style=format!(
                                "padding: 0.45rem 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer;",
                                theme.ui_button_primary,
                                theme.ui_text_primary
                            )
                        >
                            {move || t!(i18n, scenes.new_button)}
                        </button>
                    </div>

                    <div style=format!("color: {}; font-size: 0.8125rem; margin-bottom: 0.75rem;", theme.ui_text_secondary)>
                        {move || format!("{}/{}", scenes.get().len(), MAX_SCENES_PER_ROOM)}
                    </div>

                    {move || {
                        let current_scenes = scenes.get();
                        if current_scenes.is_empty() {
                            view! {
                                <div style=format!("color: {}; font-style: italic;", theme.ui_text_muted)>
                                    {t!(i18n, scenes.empty)}
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <For
                                    each=move || scenes.get()
                                    key=|scene| scene.id.clone()
                                    children=move |scene| {
                                        let scene_id = scene.id.clone();
                                        let activate_id = scene.id.clone();
                                        let edit_scene = scene.clone();
                                        let delete_id = scene.id.clone();
                                        let is_active_scene = Signal::derive({
                                            let active_scene_id = active_scene_id;
                                            let scene_id = scene_id.clone();
                                            move || active_scene_id.get() == Some(scene_id.clone())
                                        });

                                        view! {
                                            <div
                                                style=move || format!(
                                                    "padding: 0.75rem; background: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; margin-bottom: 0.625rem;",
                                                    if is_active_scene.get() { theme.ui_bg_secondary } else { theme.ui_bg_primary },
                                                    if is_active_scene.get() { theme.ui_success } else { theme.ui_border }
                                                )
                                            >
                                                <div style="display: flex; justify-content: space-between; gap: 0.75rem; align-items: flex-start;">
                                                    <div>
                                                        <div style=format!("color: {}; font-weight: 700;", theme.ui_text_primary)>
                                                            {scene.name.clone()}
                                                        </div>
                                                        <div style=format!("color: {}; font-size: 0.8125rem; margin-top: 0.25rem;", theme.ui_text_secondary)>
                                                            {format!("{} x {} · {} ft", scene.grid.columns, scene.grid.rows, scene.grid.cell_size_feet)}
                                                        </div>
                                                    </div>
                                                    {move || if is_active_scene.get() {
                                                        view! {
                                                            <span style=format!("background: {}; color: {}; padding: 0.2rem 0.5rem; border-radius: 999px; font-size: 0.75rem;", theme.ui_success, theme.ui_text_primary)>
                                                                {t!(i18n, scenes.active_badge)}
                                                            </span>
                                                        }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                </div>

                                                <div style="display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.75rem;">
                                                    <button
                                                        on:click=move |_| apply_scene_to_editor(&edit_scene)
                                                        style=format!(
                                                            "padding: 0.4rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer;",
                                                            theme.ui_button_primary,
                                                            theme.ui_text_primary
                                                        )
                                                    >
                                                        {move || t!(i18n, scenes.edit_button)}
                                                    </button>
                                                    <button
                                                        on:click=move |_| {
                                                            send_event(ClientEvent::SceneActivate(SceneActivatePayload {
                                                                scene_id: activate_id.clone(),
                                                                actor: username.get_untracked(),
                                                            }));
                                                        }
                                                        style=format!(
                                                            "padding: 0.4rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer;",
                                                            theme.ui_success,
                                                            theme.ui_text_primary
                                                        )
                                                    >
                                                        {move || t!(i18n, scenes.activate_button)}
                                                    </button>
                                                    <button
                                                        on:click=move |_| {
                                                            send_event(ClientEvent::SceneDelete(SceneDeletePayload {
                                                                scene_id: delete_id.clone(),
                                                                actor: username.get_untracked(),
                                                            }));
                                                        }
                                                        style=format!(
                                                            "padding: 0.4rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer;",
                                                            theme.ui_button_danger,
                                                            theme.ui_text_primary
                                                        )
                                                    >
                                                        {move || t!(i18n, scenes.delete_button)}
                                                    </button>
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            }.into_any()
                        }
                    }}
                </div>

                <div style="flex: 1; padding: 1rem; overflow-y: auto;">
                    <h4 style=format!("margin: 0 0 0.75rem 0; color: {};", theme.ui_text_primary)>
                        {move || {
                            if selected_scene_id.get().is_some() {
                                t_string!(i18n, scenes.edit_title).to_string()
                            } else {
                                t_string!(i18n, scenes.create_title).to_string()
                            }
                        }}
                    </h4>

                    <div style="display: flex; flex-direction: column; gap: 0.75rem;">
                        <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                            <span>{move || t!(i18n, scenes.name_label)}</span>
                            <input
                                type="text"
                                prop:value=move || draft_name.get()
                                on:input=move |ev| draft_name.set(event_target_value(&ev))
                                style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                            />
                        </label>

                        <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem;">
                            <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                <span>{move || t!(i18n, scenes.columns_label)}</span>
                                <input
                                    type="number"
                                    min="1"
                                    max="200"
                                    prop:value=move || draft_columns.get()
                                    on:input=move |ev| draft_columns.set(event_target_value(&ev))
                                    style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                />
                            </label>

                            <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                <span>{move || t!(i18n, scenes.rows_label)}</span>
                                <input
                                    type="number"
                                    min="1"
                                    max="200"
                                    prop:value=move || draft_rows.get()
                                    on:input=move |ev| draft_rows.set(event_target_value(&ev))
                                    style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                />
                            </label>

                            <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                <span>{move || t!(i18n, scenes.cell_size_label)}</span>
                                <input
                                    type="number"
                                    min="1"
                                    max="100"
                                    prop:value=move || draft_cell_size_feet.get()
                                    on:input=move |ev| draft_cell_size_feet.set(event_target_value(&ev))
                                    style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                />
                            </label>
                        </div>

                        <div style=format!("padding: 0.85rem; border: 0.0625rem solid {}; border-radius: 0.5rem; background: {};", theme.ui_bg_primary, theme.ui_border)>
                            <div style=format!("color: {}; font-size: 0.8125rem; margin-bottom: 0.55rem;", theme.ui_text_secondary)>
                                {move || t!(i18n, scenes.background_label)}
                            </div>

                            {move || {
                                let Some(file_ref) = draft_background.get() else {
                                    return view! {
                                        <div style=format!("color: {}; font-size: 0.8125rem;", theme.ui_text_muted)>
                                            {t!(i18n, scenes.background_empty)}
                                        </div>
                                    }.into_any();
                                };

                                let preview_url = file_transfer
                                    .file_urls
                                    .get()
                                    .get(&file_ref.hash)
                                    .cloned();
                                let status = file_transfer
                                    .transfer_statuses
                                    .get()
                                    .get(&file_ref.hash)
                                    .cloned();
                                let is_image = file_ref.mime_type.starts_with("image/");

                                view! {
                                    <div style="display: flex; flex-direction: column; gap: 0.65rem;">
                                        <div style=format!("color: {}; font-size: 0.875rem; font-weight: 600;", theme.ui_text_primary)>
                                            {file_ref.file_name.clone()}
                                        </div>
                                        <div style=format!("color: {}; font-size: 0.78rem;", theme.ui_text_secondary)>
                                            {format!("{} - {} bytes", file_ref.mime_type, file_ref.size)}
                                        </div>
                                        {match status {
                                            Some(status) if status.stage != FileTransferStage::Complete => view! {
                                                <div style=format!("color: {}; font-size: 0.78rem;", theme.ui_text_secondary)>
                                                    {format!(
                                                        "{} {}%",
                                                        t_string!(i18n, scenes.background_status),
                                                        status.progress_percent()
                                                    )}
                                                </div>
                                            }.into_any(),
                                            _ => ().into_any(),
                                        }}
                                        {match (is_image, preview_url) {
                                            (true, Some(url)) => view! {
                                                <img
                                                    src=url
                                                    alt=file_ref.file_name.clone()
                                                    style=format!(
                                                        "max-width: 100%; max-height: 10rem; object-fit: contain; border: 0.0625rem solid {}; border-radius: 0.5rem; background: {};",
                                                        theme.ui_border,
                                                        theme.ui_bg_secondary
                                                    )
                                                />
                                            }.into_any(),
                                            _ => ().into_any(),
                                        }}
                                    </div>
                                }.into_any()
                            }}

                            <div style="display: flex; gap: 0.75rem; flex-wrap: wrap; margin-top: 0.85rem;">
                                <input
                                    node_ref=background_input_ref
                                    type="file"
                                    accept=FILE_INPUT_ACCEPT
                                    on:change=on_background_selected
                                    style="display: none;"
                                />

                                <button
                                    type="button"
                                    on:mousedown=stop_mouse_down
                                    on:click=open_background_picker
                                    style=format!(
                                        "display: inline-flex; align-items: center; padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;",
                                        theme.ui_button_primary,
                                        theme.ui_text_primary
                                    )
                                >
                                    {move || t!(i18n, scenes.background_upload_button)}
                                </button>

                                <button
                                    type="button"
                                    on:click=move |_| {
                                        draft_background.set(None);
                                        close_background_fit_editor();
                                    }
                                    style=format!(
                                        "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;",
                                        theme.ui_bg_secondary,
                                        theme.ui_text_primary
                                    )
                                >
                                    {move || t!(i18n, scenes.background_remove_button)}
                                </button>

                                {move || {
                                    if draft_background.get().is_none() || selected_scene_id.get().is_none() {
                                        return ().into_any();
                                    }

                                    view! {
                                        <button
                                            type="button"
                                            on:click=move |_| is_background_fit_editor_open.set(true)
                                            style=format!(
                                                "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;",
                                                theme.ui_success,
                                                theme.ui_text_primary
                                            )
                                        >
                                            {move || t!(i18n, scenes.background_fit_open_button)}
                                        </button>
                                    }.into_any()
                                }}
                            </div>
                        </div>

                        <div style=format!("color: {}; font-size: 0.8125rem;", theme.ui_text_muted)>
                            {move || {
                                if let Some(active_id) = active_scene_id.get() {
                                    scenes
                                        .get()
                                        .into_iter()
                                        .find(|scene| scene.id == active_id)
                                        .map(|scene| format!("{}: {}", t_string!(i18n, scenes.current_active), scene.name))
                                        .unwrap_or_default()
                                } else {
                                    t_string!(i18n, scenes.no_active).to_string()
                                }
                            }}
                        </div>

                        {move || {
                            if let Some(error) = editor_error.get() {
                                view! {
                                    <div style=format!("background: rgba(239,68,68,0.15); color: {}; padding: 0.625rem; border-radius: 0.375rem;", theme.ui_button_danger)>
                                        {error}
                                    </div>
                                }.into_any()
                            } else {
                                ().into_any()
                            }
                        }}

                        <div style="display: flex; gap: 0.75rem; margin-top: 0.5rem;">
                            <button
                                on:click=move |_| {
                                    if selected_scene_id.get().is_some() {
                                        save_scene(());
                                    } else {
                                        create_scene(());
                                    }
                                }
                                style=format!(
                                    "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;",
                                    theme.ui_button_primary,
                                    theme.ui_text_primary
                                )
                            >
                                {move || {
                                    if selected_scene_id.get().is_some() {
                                        t_string!(i18n, scenes.save_button)
                                    } else {
                                        t_string!(i18n, scenes.create_button)
                                    }
                                }}
                            </button>

                            <button
                                on:click=move |_| reset_editor()
                                style=format!(
                                    "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;",
                                    theme.ui_bg_secondary,
                                    theme.ui_text_primary
                                )
                            >
                                {move || t!(i18n, scenes.reset_button)}
                            </button>
                        </div>
                    </div>
                </div>
                </div>
            </DraggableWindow>

            <Show when=move || is_open.get() && is_background_fit_editor_open.get()>
                <div
                    style="
                        position: fixed;
                        inset: 0;
                        background: rgba(5, 10, 18, 0.72);
                        backdrop-filter: blur(6px);
                        z-index: 2200;
                        display: flex;
                        align-items: stretch;
                        justify-content: stretch;
                    "
                >
                    <div
                        style="flex: 1; display: flex; align-items: stretch; justify-content: stretch;"
                        on:click=move |ev: MouseEvent| ev.stop_propagation()
                    >
                        <div
                            style="flex: 1; min-width: 0; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;"
                        >
                            <div style="display: flex; justify-content: space-between; gap: 1rem; align-items: flex-end; flex-wrap: wrap;">
                                <div style="min-width: 0;">
                                    <div style=format!("color: {}; font-size: 1.15rem; font-weight: 800;", theme.ui_text_primary)>
                                        {move || t!(i18n, scenes.background_fit_title)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.84rem; margin-top: 0.35rem; line-height: 1.45;", theme.ui_text_secondary)>
                                        {move || t!(i18n, scenes.background_fit_live_preview_hint)}
                                    </div>
                                </div>

                                <div style=format!("color: {}; font-size: 0.8rem;", theme.ui_text_muted)>
                                    {move || {
                                        let columns = draft_columns
                                            .get()
                                            .parse::<u16>()
                                            .ok()
                                            .filter(|value| (1..=200).contains(value));
                                        let rows = draft_rows
                                            .get()
                                            .parse::<u16>()
                                            .ok()
                                            .filter(|value| (1..=200).contains(value));
                                        let feet = draft_cell_size_feet
                                            .get()
                                            .parse::<u16>()
                                            .ok()
                                            .filter(|value| (1..=100).contains(value));

                                        match (columns, rows, feet) {
                                            (Some(columns), Some(rows), Some(feet)) => {
                                                format!(
                                                    "{} x {} cells · {} ft/cell",
                                                    columns, rows, feet
                                                )
                                            }
                                            _ => t_string!(i18n, scenes.error_invalid_grid)
                                                .to_string(),
                                        }
                                    }}
                                </div>
                            </div>

                            <div
                                style=format!(
                                    "flex: 1; min-height: 0; border: 0.0625rem solid {}; border-radius: 1rem; background: radial-gradient(circle at top, rgba(255,255,255,0.07), transparent 30%), linear-gradient(180deg, rgba(255,255,255,0.03), rgba(0,0,0,0.18)), {}; box-shadow: inset 0 0 0 0.0625rem rgba(255,255,255,0.04); display: flex; align-items: center; justify-content: center; overflow: hidden; position: relative;",
                                    theme.ui_border,
                                    theme.ui_bg_primary
                                )
                            >
                                {move || {
                                    let columns = draft_columns
                                        .get()
                                        .parse::<u16>()
                                        .ok()
                                        .filter(|value| (1..=200).contains(value));
                                    let rows = draft_rows
                                        .get()
                                        .parse::<u16>()
                                        .ok()
                                        .filter(|value| (1..=200).contains(value));
                                    let Some(columns) = columns else {
                                        return view! {
                                            <div style=format!("color: {}; font-size: 0.9rem;", theme.ui_text_muted)>
                                                {t!(i18n, scenes.error_invalid_grid)}
                                            </div>
                                        }.into_any();
                                    };
                                    let Some(rows) = rows else {
                                        return view! {
                                            <div style=format!("color: {}; font-size: 0.9rem;", theme.ui_text_muted)>
                                                {t!(i18n, scenes.error_invalid_grid)}
                                            </div>
                                        }.into_any();
                                    };

                                    let board = fit_preview_layout(columns, rows);
                                    let stroke_minor = "rgba(255,255,255,0.18)";
                                    let stroke_major = "rgba(255,255,255,0.32)";
                                    let line_width = (1.0 / board.scale.max(0.35)).min(2.8);
                                    let preview_url = draft_background
                                        .get()
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
                                    let transfer_status = draft_background
                                        .get()
                                        .and_then(|file| {
                                            file_transfer
                                                .transfer_statuses
                                                .get()
                                                .get(&file.hash)
                                                .cloned()
                                        });

                                    view! {
                                        <div
                                            style=format!(
                                                "position: relative; width: {:.2}px; height: {:.2}px; transform: scale({:.4}); transform-origin: center center; flex: 0 0 auto;",
                                                board.board_width,
                                                board.board_height,
                                                board.scale
                                            )
                                        >
                                            <div
                                                style=format!(
                                                    "position: relative; width: {:.2}px; height: {:.2}px; border: 0.125rem solid {}; border-radius: 1rem; overflow: hidden; background: linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.22)), {}; box-shadow: 0 24px 80px rgba(0,0,0,0.32), inset 0 0 0 0.0625rem rgba(255,255,255,0.06);",
                                                    board.board_width,
                                                    board.board_height,
                                                    theme.ui_success,
                                                    theme.ui_bg_primary
                                                )
                                            >
                                                {match preview_url {
                                                    Some(url) => view! {
                                                        <>
                                                            <img
                                                                src=url
                                                                alt=move || draft_name.get()
                                                                style=format!(
                                                                    "position: absolute; left: 50%; top: 50%; width: {:.2}px; max-width: none; pointer-events: none; transform: translate(-50%, -50%) translate({:.2}px, {:.2}px) scale({:.4}) rotate({:.2}deg); opacity: 0.94;",
                                                                    board.board_width,
                                                                    draft_background_offset_x.get(),
                                                                    draft_background_offset_y.get(),
                                                                    draft_background_scale.get().max(0.05),
                                                                    draft_background_rotation_deg.get()
                                                                )
                                                            />
                                                            <div
                                                                on:mousedown=move |event| {
                                                                    start_background_drag(event, board.scale);
                                                                }
                                                                style=move || format!(
                                                                    "position: absolute; inset: 0; cursor: {}; background: transparent;",
                                                                    if is_dragging_background.get() {
                                                                        "grabbing"
                                                                    } else {
                                                                        "grab"
                                                                    }
                                                                )
                                                            />
                                                        </>
                                                    }.into_any(),
                                                    None => match transfer_status {
                                                        Some(status) if status.stage != FileTransferStage::Complete => view! {
                                                            <div
                                                                style=format!(
                                                                    "position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: {}; font-size: 0.95rem; background: rgba(0,0,0,0.18);",
                                                                    theme.ui_text_secondary
                                                                )
                                                            >
                                                                {format!(
                                                                    "{} {}%",
                                                                    t_string!(i18n, scenes.background_status),
                                                                    status.progress_percent()
                                                                )}
                                                            </div>
                                                        }.into_any(),
                                                        _ => view! {
                                                            <div
                                                                style=format!(
                                                                    "position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: {}; font-size: 0.95rem;",
                                                                    theme.ui_text_muted
                                                                )
                                                            >
                                                                {t!(i18n, scenes.background_empty)}
                                                            </div>
                                                        }.into_any(),
                                                    }
                                                }}

                                                <svg
                                                    viewBox=format!("0 0 {:.4} {:.4}", board.board_width, board.board_height)
                                                    preserveAspectRatio="none"
                                                    style="position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; shape-rendering: geometricPrecision;"
                                                >
                                                    {(0..=columns)
                                                        .filter(|column| column % 5 != 0)
                                                        .map(|column| {
                                                            let x = f64::from(column) * board.cell_size;
                                                            view! {
                                                                <line
                                                                    x1=format!("{x:.4}")
                                                                    y1="0"
                                                                    x2=format!("{x:.4}")
                                                                    y2=format!("{:.4}", board.board_height)
                                                                    stroke=stroke_minor
                                                                    stroke-width=format!("{line_width:.4}")
                                                                />
                                                            }
                                                        })
                                                        .collect_view()}
                                                    {(0..=rows)
                                                        .filter(|row| row % 5 != 0)
                                                        .map(|row| {
                                                            let y = f64::from(row) * board.cell_size;
                                                            view! {
                                                                <line
                                                                    x1="0"
                                                                    y1=format!("{y:.4}")
                                                                    x2=format!("{:.4}", board.board_width)
                                                                    y2=format!("{y:.4}")
                                                                    stroke=stroke_minor
                                                                    stroke-width=format!("{line_width:.4}")
                                                                />
                                                            }
                                                        })
                                                        .collect_view()}
                                                    {(0..=columns)
                                                        .filter(|column| column % 5 == 0)
                                                        .map(|column| {
                                                            let x = f64::from(column) * board.cell_size;
                                                            view! {
                                                                <line
                                                                    x1=format!("{x:.4}")
                                                                    y1="0"
                                                                    x2=format!("{x:.4}")
                                                                    y2=format!("{:.4}", board.board_height)
                                                                    stroke=stroke_major
                                                                    stroke-width=format!("{:.4}", line_width * 1.15)
                                                                />
                                                            }
                                                        })
                                                        .collect_view()}
                                                    {(0..=rows)
                                                        .filter(|row| row % 5 == 0)
                                                        .map(|row| {
                                                            let y = f64::from(row) * board.cell_size;
                                                            view! {
                                                                <line
                                                                    x1="0"
                                                                    y1=format!("{y:.4}")
                                                                    x2=format!("{:.4}", board.board_width)
                                                                    y2=format!("{y:.4}")
                                                                    stroke=stroke_major
                                                                    stroke-width=format!("{:.4}", line_width * 1.15)
                                                                />
                                                            }
                                                        })
                                                        .collect_view()}
                                                </svg>
                                            </div>
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        </div>

                        <div
                            style=format!(
                                "width: min(34rem, 100vw); height: 100vh; background: linear-gradient(180deg, {}, {}); border-left: 0.0625rem solid {}; box-shadow: -24px 0 64px rgba(0,0,0,0.35); display: flex; flex-direction: column;",
                                theme.ui_bg_primary,
                                theme.ui_bg_secondary,
                                theme.ui_border
                            )
                        >
                            <div
                                style=format!(
                                    "padding: 1.4rem 1.5rem 1rem; border-bottom: 0.0625rem solid {}; display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start;",
                                    theme.ui_border
                                )
                            >
                                <div style="min-width: 0;">
                                    <div style=format!("color: {}; font-size: 1.2rem; font-weight: 800;", theme.ui_text_primary)>
                                        {move || t!(i18n, scenes.background_fit_modal_title)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.84rem; margin-top: 0.45rem; line-height: 1.45;", theme.ui_text_secondary)>
                                        {move || t!(i18n, scenes.background_fit_modal_hint)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.78rem; margin-top: 0.55rem;", theme.ui_text_muted)>
                                        {move || {
                                            selected_scene_id
                                                .get()
                                                .and_then(|scene_id| {
                                                    scenes
                                                        .get()
                                                        .into_iter()
                                                        .find(|scene| scene.id == scene_id)
                                                        .map(|scene| scene.name)
                                                })
                                                .unwrap_or_default()
                                        }}
                                    </div>
                                </div>

                                <button
                                    type="button"
                                    on:click=move |_| close_background_fit_editor()
                                    style=format!(
                                        "background: {}; border: none; color: {}; padding: 0.35rem 0.75rem; border-radius: 0.375rem; cursor: pointer; font-size: 1rem; font-weight: 700;",
                                        theme.ui_button_danger,
                                        theme.ui_text_primary
                                    )
                                >
                                    "×"
                                </button>
                            </div>

                            <div style="flex: 1; overflow-y: auto; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;">
                                <div
                                    style=format!(
                                        "padding: 0.9rem 1rem; border-radius: 0.75rem; background: rgba(255,255,255,0.04); border: 0.0625rem solid {}; color: {}; font-size: 0.82rem; line-height: 1.5;",
                                        theme.ui_border,
                                        theme.ui_text_secondary
                                    )
                                >
                                    {move || t!(i18n, scenes.background_fit_live_preview_hint)}
                                </div>

                                <div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.85rem;">
                                    <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                        <span>{move || t!(i18n, scenes.columns_label)}</span>
                                        <input
                                            type="number"
                                            min="1"
                                            max="200"
                                            prop:value=move || draft_columns.get()
                                            on:input=move |ev| draft_columns.set(event_target_value(&ev))
                                            style=format!(
                                                "padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;",
                                                theme.ui_bg_secondary,
                                                theme.ui_text_primary,
                                                theme.ui_border
                                            )
                                        />
                                    </label>

                                    <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                        <span>{move || t!(i18n, scenes.rows_label)}</span>
                                        <input
                                            type="number"
                                            min="1"
                                            max="200"
                                            prop:value=move || draft_rows.get()
                                            on:input=move |ev| draft_rows.set(event_target_value(&ev))
                                            style=format!(
                                                "padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;",
                                                theme.ui_bg_secondary,
                                                theme.ui_text_primary,
                                                theme.ui_border
                                            )
                                        />
                                    </label>
                                </div>

                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.background_scale_label)}</span>
                                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                                        <input
                                            type="range"
                                            min=MIN_BACKGROUND_SCALE.to_string()
                                            max=MAX_BACKGROUND_SCALE.to_string()
                                            step="0.01"
                                            prop:value=move || format!("{:.2}", draft_background_scale.get())
                                            on:input=move |ev| {
                                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                                    draft_background_scale.set(value.clamp(MIN_BACKGROUND_SCALE, MAX_BACKGROUND_SCALE));
                                                }
                                            }
                                            style="flex: 1;"
                                        />
                                        <input
                                            type="number"
                                            min=MIN_BACKGROUND_SCALE.to_string()
                                            max=MAX_BACKGROUND_SCALE.to_string()
                                            step="0.01"
                                            prop:value=move || format!("{:.2}", draft_background_scale.get())
                                            on:input=move |ev| {
                                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                                    draft_background_scale.set(value.clamp(MIN_BACKGROUND_SCALE, MAX_BACKGROUND_SCALE));
                                                }
                                            }
                                            style=format!(
                                                "width: 6rem; padding: 0.65rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;",
                                                theme.ui_bg_secondary,
                                                theme.ui_text_primary,
                                                theme.ui_border
                                            )
                                        />
                                    </div>
                                </label>

                                <div
                                    style=format!(
                                        "padding: 0.95rem 1rem; border-radius: 0.75rem; background: rgba(255,255,255,0.03); border: 0.0625rem solid {}; display: flex; flex-direction: column; gap: 0.55rem;",
                                        theme.ui_border
                                    )
                                >
                                    <div style=format!("color: {}; font-size: 0.9rem; font-weight: 700;", theme.ui_text_primary)>
                                        {move || t!(i18n, scenes.background_position_label)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.82rem; line-height: 1.45;", theme.ui_text_secondary)>
                                        {move || t!(i18n, scenes.background_fit_drag_hint)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.8rem;", theme.ui_text_muted)>
                                        {move || {
                                            format!(
                                                "X: {:.0}px | Y: {:.0}px",
                                                draft_background_offset_x.get(),
                                                draft_background_offset_y.get()
                                            )
                                        }}
                                    </div>
                                </div>

                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.background_rotation_label)}</span>
                                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                                        <input
                                            type="range"
                                            min=MIN_BACKGROUND_ROTATION_DEG.to_string()
                                            max=MAX_BACKGROUND_ROTATION_DEG.to_string()
                                            step="1"
                                            prop:value=move || format!("{:.0}", draft_background_rotation_deg.get())
                                            on:input=move |ev| {
                                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                                    draft_background_rotation_deg.set(value.clamp(MIN_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_ROTATION_DEG));
                                                }
                                            }
                                            style="flex: 1;"
                                        />
                                        <input
                                            type="number"
                                            min=MIN_BACKGROUND_ROTATION_DEG.to_string()
                                            max=MAX_BACKGROUND_ROTATION_DEG.to_string()
                                            step="1"
                                            prop:value=move || format!("{:.0}", draft_background_rotation_deg.get())
                                            on:input=move |ev| {
                                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                                    draft_background_rotation_deg.set(value.clamp(MIN_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_ROTATION_DEG));
                                                }
                                            }
                                            style=format!(
                                                "width: 6rem; padding: 0.65rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;",
                                                theme.ui_bg_secondary,
                                                theme.ui_text_primary,
                                                theme.ui_border
                                            )
                                        />
                                    </div>
                                </label>

                                <div
                                    style=format!(
                                        "margin-top: auto; display: flex; gap: 0.75rem; justify-content: flex-end; padding-top: 0.5rem; border-top: 0.0625rem solid {}; flex-wrap: wrap;",
                                        theme.ui_border
                                    )
                                >
                                    <button
                                        type="button"
                                        on:click=move |_| reset_background_fit()
                                        style=format!(
                                            "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer;",
                                            theme.ui_bg_secondary,
                                            theme.ui_text_primary
                                        )
                                    >
                                        {move || t!(i18n, scenes.background_reset_fit_button)}
                                    </button>

                                    <button
                                        type="button"
                                        on:click=move |_| close_background_fit_editor()
                                        style=format!(
                                            "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer;",
                                            theme.ui_bg_secondary,
                                            theme.ui_text_primary
                                        )
                                    >
                                        {move || t!(i18n, scenes.background_fit_modal_close_button)}
                                    </button>

                                    <button
                                        type="button"
                                        on:click=move |_| {
                                            save_scene(());
                                            close_background_fit_editor();
                                        }
                                        style=format!(
                                            "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-weight: 700;",
                                            theme.ui_button_primary,
                                            theme.ui_text_primary
                                        )
                                    >
                                        {move || t!(i18n, scenes.background_fit_modal_save_button)}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>
        </>
    }
}
