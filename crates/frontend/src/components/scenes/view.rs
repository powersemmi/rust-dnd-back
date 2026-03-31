use super::background_fit_editor::BackgroundFitEditor;
use super::model::{
    FILE_INPUT_ACCEPT, MAX_BACKGROUND_OFFSET_PX, MAX_SCENES_PER_ROOM, MIN_BACKGROUND_OFFSET_PX,
    default_scene_position,
};
use super::view_model::ScenesWindowViewModel;
use crate::components::draggable_window::DraggableWindow;
use crate::components::websocket::{FileTransferStage, FileTransferState, WsSender};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev;
use leptos::ev::MouseEvent;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use shared::events::{
    ClientEvent, Scene, SceneActivatePayload, SceneCreatePayload, SceneDeletePayload,
    SceneUpdatePayload,
};
use uuid::Uuid;
use web_sys::{Event, HtmlInputElement};

const SCENES_SECTION_FONT_SIZE: &str = "clamp(0.95rem, 0.91rem + 0.16vw, 1.05rem)";
const SCENES_BODY_FONT_SIZE: &str = "clamp(0.9rem, 0.87rem + 0.12vw, 0.98rem)";
const SCENES_META_FONT_SIZE: &str = "clamp(0.74rem, 0.71rem + 0.12vw, 0.82rem)";
const SCENES_BUTTON_FONT_SIZE: &str = "clamp(0.84rem, 0.81rem + 0.12vw, 0.94rem)";

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
    let vm = ScenesWindowViewModel::new();
    let background_input_ref = NodeRef::<html::Input>::new();

    let send_event = move |event: ClientEvent| {
        if let Some(sender) = ws_sender.get_untracked() {
            let _ = sender.try_send_event(event);
        }
    };

    // Build scene from draft + create or update via WS
    let create_scene = move |_| {
        if scenes.get_untracked().len() >= MAX_SCENES_PER_ROOM {
            vm.editor_error
                .set(Some(t_string!(i18n, scenes.error_limit).to_string()));
            return;
        }
        let Some(grid) = vm.build_grid(
            t_string!(i18n, scenes.error_empty_name).to_string(),
            t_string!(i18n, scenes.error_invalid_grid).to_string(),
        ) else {
            return;
        };
        let (workspace_x, workspace_y) =
            default_scene_position(scenes.get_untracked().len(), grid.columns, grid.rows);
        let scene = Scene {
            id: Uuid::new_v4().to_string(),
            name: vm.draft_name.get_untracked().trim().to_string(),
            grid,
            workspace_x,
            workspace_y,
            background: vm.draft_background.get_untracked(),
            background_scale: vm.clamp_background_scale(vm.draft_background_scale.get_untracked()),
            background_offset_x: vm
                .draft_background_offset_x
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_offset_y: vm
                .draft_background_offset_y
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_rotation_deg: vm
                .clamp_background_rotation(vm.draft_background_rotation_deg.get_untracked()),
            tokens: Vec::new(),
        };
        send_event(ClientEvent::SceneCreate(SceneCreatePayload {
            scene,
            actor: username.get_untracked(),
        }));
        vm.reset();
    };

    let save_scene = move |_| {
        let Some(scene_id) = vm.selected_scene_id.get_untracked() else {
            create_scene(());
            return;
        };
        let Some(grid) = vm.build_grid(
            t_string!(i18n, scenes.error_empty_name).to_string(),
            t_string!(i18n, scenes.error_invalid_grid).to_string(),
        ) else {
            return;
        };
        let existing = scenes
            .get_untracked()
            .into_iter()
            .find(|s| s.id == scene_id);
        let scene = Scene {
            id: scene_id,
            name: vm.draft_name.get_untracked().trim().to_string(),
            grid,
            workspace_x: existing.as_ref().map(|s| s.workspace_x).unwrap_or(0.0),
            workspace_y: existing.as_ref().map(|s| s.workspace_y).unwrap_or(0.0),
            background: vm.draft_background.get_untracked(),
            background_scale: vm.clamp_background_scale(vm.draft_background_scale.get_untracked()),
            background_offset_x: vm
                .draft_background_offset_x
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_offset_y: vm
                .draft_background_offset_y
                .get_untracked()
                .clamp(MIN_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_OFFSET_PX),
            background_rotation_deg: vm
                .clamp_background_rotation(vm.draft_background_rotation_deg.get_untracked()),
            tokens: existing.map(|scene| scene.tokens).unwrap_or_default(),
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
                .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
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
            let uname = username.get_untracked();
            let sender = ws_sender.get_untracked();
            spawn_local(async move {
                match file_transfer.import_browser_file(file, uname, sender).await {
                    Ok(file_ref) if file_ref.mime_type.starts_with("image/") => {
                        vm.draft_background.set(Some(file_ref));
                        vm.reset_background_fit();
                        vm.editor_error.set(None);
                    }
                    Ok(_) => vm
                        .editor_error
                        .set(Some("Scene background must be an image".to_string())),
                    Err(e) => vm.editor_error.set(Some(e)),
                }
            });
            input.set_value("");
        }
    };

    // Close fit editor when window is closed
    Effect::new(move |_| {
        if !is_open.get() {
            vm.close_background_fit_editor();
        }
    });

    // Sync draft fields when selected scene changes externally
    Effect::new(move |_| {
        let selected_id = vm.selected_scene_id.get();
        let current_scenes = scenes.get();
        if let Some(scene_id) = selected_id {
            if let Some(scene) = current_scenes.iter().find(|s| s.id == scene_id) {
                vm.draft_name.set(scene.name.clone());
                vm.draft_columns.set(scene.grid.columns.to_string());
                vm.draft_rows.set(scene.grid.rows.to_string());
                vm.draft_cell_size_feet
                    .set(scene.grid.cell_size_feet.to_string());
                vm.draft_background.set(scene.background.clone());
                vm.draft_background_scale.set(scene.background_scale);
                vm.draft_background_offset_x.set(scene.background_offset_x);
                vm.draft_background_offset_y.set(scene.background_offset_y);
                vm.draft_background_rotation_deg
                    .set(scene.background_rotation_deg);
                if scene.background.is_none() {
                    vm.close_background_fit_editor();
                }
                vm.editor_error.set(None);
            } else {
                vm.reset();
            }
        }
    });

    // Global mouse listeners for background drag
    Effect::new(move |_| {
        let handle_mousemove = window_event_listener(ev::mousemove, move |event: MouseEvent| {
            vm.update_background_drag(event.client_x(), event.client_y());
        });
        let handle_mouseup = window_event_listener(ev::mouseup, move |_: MouseEvent| {
            vm.stop_background_drag();
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
                    // Left panel: scene list
                    <div style=format!(
                        "width: 48%; border-right: 0.0625rem solid {}; padding: 1rem; overflow-y: auto;",
                        theme.ui_border
                    )>
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem;">
                            <h4 style=format!("margin: 0; color: {};", theme.ui_text_primary)>
                                {move || t_string!(i18n, scenes.list_title)}
                            </h4>
                            <button
                                on:click=move |_| vm.reset()
                                style=format!(
                                    "padding: 0.45rem 0.75rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                    theme.ui_button_primary, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
                                )
                            >
                                {move || t!(i18n, scenes.new_button)}
                            </button>
                        </div>

                        <div style=format!(
                            "color: {}; font-size: {}; margin-bottom: 0.75rem;",
                            theme.ui_text_secondary, SCENES_META_FONT_SIZE
                        )>
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
                                        each=move || { scenes.get() }
                                        key=|scene| scene.id.clone()
                                        children=move |scene| {
                                            let scene_id = scene.id.clone();
                                            let activate_id = scene.id.clone();
                                            let edit_scene = scene.clone();
                                            let delete_id = scene.id.clone();
                                            let is_active_scene = Signal::derive({
                                                let id = scene_id.clone();
                                                move || active_scene_id.get() == Some(id.clone())
                                            });
                                            view! {
                                                <div style=move || format!(
                                                    "padding: 0.75rem; background: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; margin-bottom: 0.625rem;",
                                                    if is_active_scene.get() { theme.ui_bg_secondary } else { theme.ui_bg_primary },
                                                    if is_active_scene.get() { theme.ui_success } else { theme.ui_border }
                                                )>
                                                    <div style="display: flex; justify-content: space-between; gap: 0.75rem; align-items: flex-start;">
                                                        <div>
                                                            <div style=format!("color: {}; font-weight: 700;", theme.ui_text_primary)>
                                                                {scene.name.clone()}
                                                            </div>
                                                            <div style=format!(
                                                                "color: {}; font-size: {}; margin-top: 0.25rem;",
                                                                theme.ui_text_secondary, SCENES_META_FONT_SIZE
                                                            )>
                                                                {format!("{} x {} · {} ft", scene.grid.columns, scene.grid.rows, scene.grid.cell_size_feet)}
                                                            </div>
                                                        </div>
                                                        {move || if is_active_scene.get() {
                                                            view! {
                                                                <span style=format!(
                                                                    "background: {}; color: {}; padding: 0.2rem 0.5rem; border-radius: 999px; font-size: {};",
                                                                    theme.ui_success, theme.ui_text_primary, SCENES_META_FONT_SIZE
                                                                )>
                                                                    {t!(i18n, scenes.active_badge)}
                                                                </span>
                                                            }.into_any()
                                                        } else { ().into_any() }}
                                                    </div>

                                                    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.75rem;">
                                                        <button
                                                            on:click=move |_| vm.apply_scene(&edit_scene)
                                                            style=format!(
                                                                "padding: 0.4rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                                                theme.ui_button_primary, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
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
                                                                "padding: 0.4rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                                                theme.ui_success, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
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
                                                                "padding: 0.4rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                                                theme.ui_button_danger, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
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

                    // Right panel: editor form
                    <div style="flex: 1; padding: 1rem; overflow-y: auto;">
                        <h4 style=format!(
                            "margin: 0 0 0.75rem 0; color: {}; font-size: {}; line-height: 1.2;",
                            theme.ui_text_primary, SCENES_SECTION_FONT_SIZE
                        )>
                            {move || {
                                if vm.selected_scene_id.get().is_some() {
                                    t_string!(i18n, scenes.edit_title).to_string()
                                } else {
                                    t_string!(i18n, scenes.create_title).to_string()
                                }
                            }}
                        </h4>

                        <div style="display: flex; flex-direction: column; gap: 0.75rem;">
                            // Name
                            <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                <span>{move || t!(i18n, scenes.name_label)}</span>
                                <input
                                    type="text"
                                    prop:value=move || vm.draft_name.get()
                                    on:input=move |ev| vm.draft_name.set(event_target_value(&ev))
                                    style=format!(
                                        "padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                        theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, SCENES_BODY_FONT_SIZE
                                    )
                                />
                            </label>

                            // Grid dimensions
                            <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem;">
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.columns_label)}</span>
                                    <input type="number" min="1" max="200"
                                        prop:value=move || vm.draft_columns.get()
                                        on:input=move |ev| vm.draft_columns.set(event_target_value(&ev))
                                        style=format!(
                                            "padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, SCENES_BODY_FONT_SIZE
                                        )
                                    />
                                </label>
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.rows_label)}</span>
                                    <input type="number" min="1" max="200"
                                        prop:value=move || vm.draft_rows.get()
                                        on:input=move |ev| vm.draft_rows.set(event_target_value(&ev))
                                        style=format!(
                                            "padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, SCENES_BODY_FONT_SIZE
                                        )
                                    />
                                </label>
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.cell_size_label)}</span>
                                    <input type="number" min="1" max="100"
                                        prop:value=move || vm.draft_cell_size_feet.get()
                                        on:input=move |ev| vm.draft_cell_size_feet.set(event_target_value(&ev))
                                        style=format!(
                                            "padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, SCENES_BODY_FONT_SIZE
                                        )
                                    />
                                </label>
                            </div>

                            // Background section
                            <div style=format!("padding: 0.85rem; border: 0.0625rem solid {}; border-radius: 0.5rem; background: {};", theme.ui_bg_primary, theme.ui_border)>
                                <div style=format!(
                                    "color: {}; font-size: {}; margin-bottom: 0.55rem;",
                                    theme.ui_text_secondary, SCENES_META_FONT_SIZE
                                )>
                                    {move || t!(i18n, scenes.background_label)}
                                </div>

                                {move || {
                                    let Some(file_ref) = vm.draft_background.get() else {
                                        return view! {
                                            <div style=format!(
                                                "color: {}; font-size: {};",
                                                theme.ui_text_muted, SCENES_META_FONT_SIZE
                                            )>
                                                {t!(i18n, scenes.background_empty)}
                                            </div>
                                        }.into_any();
                                    };
                                    let preview_url = file_transfer.file_urls.get().get(&file_ref.hash).cloned();
                                    let status = file_transfer.transfer_statuses.get().get(&file_ref.hash).cloned();
                                    let is_image = file_ref.mime_type.starts_with("image/");
                                    view! {
                                        <div style="display: flex; flex-direction: column; gap: 0.65rem;">
                                            <div style=format!(
                                                "color: {}; font-size: {}; font-weight: 600;",
                                                theme.ui_text_primary, SCENES_BODY_FONT_SIZE
                                            )>
                                                {file_ref.file_name.clone()}
                                            </div>
                                            <div style=format!(
                                                "color: {}; font-size: {};",
                                                theme.ui_text_secondary, SCENES_META_FONT_SIZE
                                            )>
                                                {format!("{} - {} bytes", file_ref.mime_type, file_ref.size)}
                                            </div>
                                            {match status {
                                                Some(s) if s.stage != FileTransferStage::Complete => view! {
                                                    <div style=format!(
                                                        "color: {}; font-size: {};",
                                                        theme.ui_text_secondary, SCENES_META_FONT_SIZE
                                                    )>
                                                        {format!("{} {}%", t_string!(i18n, scenes.background_status), s.progress_percent())}
                                                    </div>
                                                }.into_any(),
                                                _ => ().into_any(),
                                            }}
                                            {match (is_image, preview_url) {
                                                (true, Some(url)) => view! {
                                                    <img src=url alt=file_ref.file_name.clone()
                                                        style=format!("max-width: 100%; max-height: 10rem; object-fit: contain; border: 0.0625rem solid {}; border-radius: 0.5rem; background: {};", theme.ui_border, theme.ui_bg_secondary)
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
                                    <button type="button"
                                        on:mousedown=move |e: MouseEvent| e.stop_propagation()
                                        on:click=move |e: MouseEvent| {
                                            e.prevent_default();
                                            e.stop_propagation();
                                            if let Some(input) = background_input_ref.get() {
                                                input.click();
                                            }
                                        }
                                        style=format!(
                                            "display: inline-flex; align-items: center; padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                            theme.ui_button_primary, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
                                        )
                                    >
                                        {move || t!(i18n, scenes.background_upload_button)}
                                    </button>
                                    <button type="button"
                                        on:click=move |_| {
                                            vm.draft_background.set(None);
                                            vm.close_background_fit_editor();
                                        }
                                        style=format!(
                                            "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                            theme.ui_bg_secondary, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
                                        )
                                    >
                                        {move || t!(i18n, scenes.background_remove_button)}
                                    </button>
                                    {move || {
                                        if vm.draft_background.get().is_none() || vm.selected_scene_id.get().is_none() {
                                            return ().into_any();
                                        }
                                        view! {
                                            <button type="button"
                                                on:click=move |_| vm.is_background_fit_editor_open.set(true)
                                                style=format!(
                                                    "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                                    theme.ui_success, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
                                                )
                                            >
                                                {move || t!(i18n, scenes.background_fit_open_button)}
                                            </button>
                                        }.into_any()
                                    }}
                                </div>
                            </div>

                            // Active scene hint
                            <div style=format!(
                                "color: {}; font-size: {};",
                                theme.ui_text_muted, SCENES_META_FONT_SIZE
                            )>
                                {move || {
                                    if let Some(id) = active_scene_id.get() {
                                        scenes.get().into_iter()
                                            .find(|s| s.id == id)
                                            .map(|s| format!("{}: {}", t_string!(i18n, scenes.current_active), s.name))
                                            .unwrap_or_default()
                                    } else {
                                        t_string!(i18n, scenes.no_active).to_string()
                                    }
                                }}
                            </div>

                            // Validation error
                            {move || {
                                if let Some(err) = vm.editor_error.get() {
                                    view! {
                                        <div style=format!(
                                            "background: rgba(239,68,68,0.15); color: {}; padding: 0.625rem; border-radius: 0.5rem; font-size: {}; line-height: 1.45;",
                                            theme.ui_button_danger, SCENES_BODY_FONT_SIZE
                                        )>
                                            {err}
                                        </div>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }
                            }}

                            // Action buttons
                            <div style="display: flex; gap: 0.75rem; margin-top: 0.5rem;">
                                <button
                                    on:click=move |_| {
                                        if vm.selected_scene_id.get().is_some() {
                                            save_scene(());
                                        } else {
                                            create_scene(());
                                        }
                                    }
                                    style=format!(
                                        "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                        theme.ui_button_primary, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
                                    )
                                >
                                    {move || {
                                        if vm.selected_scene_id.get().is_some() {
                                            t_string!(i18n, scenes.save_button)
                                        } else {
                                            t_string!(i18n, scenes.create_button)
                                        }
                                    }}
                                </button>
                                <button
                                    on:click=move |_| vm.reset()
                                    style=format!(
                                        "padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                        theme.ui_bg_secondary, theme.ui_text_primary, SCENES_BUTTON_FONT_SIZE
                                    )
                                >
                                    {move || t!(i18n, scenes.reset_button)}
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </DraggableWindow>

            // Background fit editor modal
            <Show when=move || is_open.get() && vm.is_background_fit_editor_open.get()>
                <BackgroundFitEditor
                    vm=vm
                    scenes=scenes
                    file_transfer=file_transfer.clone()
                    on_save=Callback::new(move |_| save_scene(()))
                    theme=theme.clone()
                />
            </Show>
        </>
    }
}
