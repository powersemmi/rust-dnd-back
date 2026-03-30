use crate::components::draggable_window::DraggableWindow;
use crate::components::websocket::{FileTransferStage, FileTransferState};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
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

fn default_scene_position(scene_count: usize) -> (f32, f32) {
    (scene_count as f32 * DEFAULT_SCENE_SPACING_PX, 0.0)
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
    let editor_error = RwSignal::new(None::<String>);
    let background_input_ref = NodeRef::<html::Input>::new();

    let reset_editor = move || {
        selected_scene_id.set(None);
        draft_name.set(String::new());
        draft_columns.set(DEFAULT_COLUMNS.to_string());
        draft_rows.set(DEFAULT_ROWS.to_string());
        draft_cell_size_feet.set(DEFAULT_CELL_SIZE_FEET.to_string());
        draft_background.set(None);
        editor_error.set(None);
    };

    let apply_scene_to_editor = move |scene: &Scene| {
        selected_scene_id.set(Some(scene.id.clone()));
        draft_name.set(scene.name.clone());
        draft_columns.set(scene.grid.columns.to_string());
        draft_rows.set(scene.grid.rows.to_string());
        draft_cell_size_feet.set(scene.grid.cell_size_feet.to_string());
        draft_background.set(scene.background.clone());
        editor_error.set(None);
    };

    let send_event = move |event: ClientEvent| {
        if let Some(mut sender) = ws_sender.get_untracked()
            && let Ok(json) = serde_json::to_string(&event)
        {
            let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
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
        let selected_id = selected_scene_id.get();
        let current_scenes = scenes.get();

        if let Some(scene_id) = selected_id {
            if let Some(scene) = current_scenes.iter().find(|scene| scene.id == scene_id) {
                draft_name.set(scene.name.clone());
                draft_columns.set(scene.grid.columns.to_string());
                draft_rows.set(scene.grid.rows.to_string());
                draft_cell_size_feet.set(scene.grid.cell_size_feet.to_string());
                draft_background.set(scene.background.clone());
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
            background_scale: 1.0,
            background_offset_x: 0.0,
            background_offset_y: 0.0,
            background_rotation_deg: 0.0,
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
            background_scale: existing_scene
                .as_ref()
                .map(|scene| scene.background_scale)
                .unwrap_or(1.0),
            background_offset_x: existing_scene
                .as_ref()
                .map(|scene| scene.background_offset_x)
                .unwrap_or(0.0),
            background_offset_y: existing_scene
                .as_ref()
                .map(|scene| scene.background_offset_y)
                .unwrap_or(0.0),
            background_rotation_deg: existing_scene
                .as_ref()
                .map(|scene| scene.background_rotation_deg)
                .unwrap_or(0.0),
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

    view! {
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
                                                        view! { <></> }.into_any()
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
                                            _ => view! { <></> }.into_any(),
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
                                            _ => view! { <></> }.into_any(),
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
                                    on:click=move |_| draft_background.set(None)
                                    style=format!(
                                        "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;",
                                        theme.ui_bg_secondary,
                                        theme.ui_text_primary
                                    )
                                >
                                    {move || t!(i18n, scenes.background_remove_button)}
                                </button>
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
                                view! { <></> }.into_any()
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
    }
}
