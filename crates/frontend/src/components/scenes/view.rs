use super::model::{
    FILE_INPUT_ACCEPT, MAX_BACKGROUND_OFFSET_PX, MAX_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_SCALE,
    MAX_SCENES_PER_ROOM, MIN_BACKGROUND_OFFSET_PX, MIN_BACKGROUND_ROTATION_DEG,
    MIN_BACKGROUND_SCALE, default_scene_position, fit_preview_layout,
};

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
use super::view_model::ScenesWindowViewModel;
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
    ClientEvent, Scene, SceneActivatePayload, SceneCreatePayload, SceneDeletePayload,
    SceneUpdatePayload,
};
use uuid::Uuid;
use web_sys::{Event, HtmlInputElement};

use crate::components::websocket::WsSender;

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
        let (workspace_x, workspace_y) = default_scene_position(scenes.get_untracked().len());
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
                                    "padding: 0.45rem 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer;",
                                    theme.ui_button_primary, theme.ui_text_primary
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
                                                        } else { ().into_any() }}
                                                    </div>

                                                    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.75rem;">
                                                        <button
                                                            on:click=move |_| vm.apply_scene(&edit_scene)
                                                            style=format!(
                                                                "padding: 0.4rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer;",
                                                                theme.ui_button_primary, theme.ui_text_primary
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
                                                                theme.ui_success, theme.ui_text_primary
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
                                                                theme.ui_button_danger, theme.ui_text_primary
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
                        <h4 style=format!("margin: 0 0 0.75rem 0; color: {};", theme.ui_text_primary)>
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
                                    style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                />
                            </label>

                            // Grid dimensions
                            <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem;">
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.columns_label)}</span>
                                    <input type="number" min="1" max="200"
                                        prop:value=move || vm.draft_columns.get()
                                        on:input=move |ev| vm.draft_columns.set(event_target_value(&ev))
                                        style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                    />
                                </label>
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.rows_label)}</span>
                                    <input type="number" min="1" max="200"
                                        prop:value=move || vm.draft_rows.get()
                                        on:input=move |ev| vm.draft_rows.set(event_target_value(&ev))
                                        style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                    />
                                </label>
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.cell_size_label)}</span>
                                    <input type="number" min="1" max="100"
                                        prop:value=move || vm.draft_cell_size_feet.get()
                                        on:input=move |ev| vm.draft_cell_size_feet.set(event_target_value(&ev))
                                        style=format!("padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                    />
                                </label>
                            </div>

                            // Background section
                            <div style=format!("padding: 0.85rem; border: 0.0625rem solid {}; border-radius: 0.5rem; background: {};", theme.ui_bg_primary, theme.ui_border)>
                                <div style=format!("color: {}; font-size: 0.8125rem; margin-bottom: 0.55rem;", theme.ui_text_secondary)>
                                    {move || t!(i18n, scenes.background_label)}
                                </div>

                                {move || {
                                    let Some(file_ref) = vm.draft_background.get() else {
                                        return view! {
                                            <div style=format!("color: {}; font-size: 0.8125rem;", theme.ui_text_muted)>
                                                {t!(i18n, scenes.background_empty)}
                                            </div>
                                        }.into_any();
                                    };
                                    let preview_url = file_transfer.file_urls.get().get(&file_ref.hash).cloned();
                                    let status = file_transfer.transfer_statuses.get().get(&file_ref.hash).cloned();
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
                                                Some(s) if s.stage != FileTransferStage::Complete => view! {
                                                    <div style=format!("color: {}; font-size: 0.78rem;", theme.ui_text_secondary)>
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
                                        style=format!("display: inline-flex; align-items: center; padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;", theme.ui_button_primary, theme.ui_text_primary)
                                    >
                                        {move || t!(i18n, scenes.background_upload_button)}
                                    </button>
                                    <button type="button"
                                        on:click=move |_| {
                                            vm.draft_background.set(None);
                                            vm.close_background_fit_editor();
                                        }
                                        style=format!("padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;", theme.ui_bg_secondary, theme.ui_text_primary)
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
                                                style=format!("padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;", theme.ui_success, theme.ui_text_primary)
                                            >
                                                {move || t!(i18n, scenes.background_fit_open_button)}
                                            </button>
                                        }.into_any()
                                    }}
                                </div>
                            </div>

                            // Active scene hint
                            <div style=format!("color: {}; font-size: 0.8125rem;", theme.ui_text_muted)>
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
                                        <div style=format!("background: rgba(239,68,68,0.15); color: {}; padding: 0.625rem; border-radius: 0.375rem;", theme.ui_button_danger)>
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
                                    style=format!("padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;", theme.ui_button_primary, theme.ui_text_primary)
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
                                    style=format!("padding: 0.625rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;", theme.ui_bg_secondary, theme.ui_text_primary)
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
                <div style="position: fixed; inset: 0; background: rgba(5,10,18,0.72); backdrop-filter: blur(6px); z-index: 2200; display: flex; align-items: stretch; justify-content: stretch;">
                    <div style="flex: 1; display: flex; align-items: stretch; justify-content: stretch;"
                        on:click=move |e: MouseEvent| e.stop_propagation()
                    >
                        // Preview area
                        <div style="flex: 1; min-width: 0; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;">
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
                                        let cols = vm.draft_columns.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                                        let rows = vm.draft_rows.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                                        let feet = vm.draft_cell_size_feet.get().parse::<u16>().ok().filter(|v| (1..=100).contains(v));
                                        match (cols, rows, feet) {
                                            (Some(c), Some(r), Some(f)) => format!("{} x {} cells · {} ft/cell", c, r, f),
                                            _ => t_string!(i18n, scenes.error_invalid_grid).to_string(),
                                        }
                                    }}
                                </div>
                            </div>

                            <div style=format!("flex: 1; min-height: 0; border: 0.0625rem solid {}; border-radius: 1rem; background: radial-gradient(circle at top, rgba(255,255,255,0.07), transparent 30%), linear-gradient(180deg, rgba(255,255,255,0.03), rgba(0,0,0,0.18)), {}; box-shadow: inset 0 0 0 0.0625rem rgba(255,255,255,0.04); display: flex; align-items: center; justify-content: center; overflow: hidden; position: relative;", theme.ui_border, theme.ui_bg_primary)>
                                {move || {
                                    let cols = vm.draft_columns.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                                    let rows = vm.draft_rows.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                                    let (Some(columns), Some(rows)) = (cols, rows) else {
                                        return view! {
                                            <div style=format!("color: {}; font-size: 0.9rem;", theme.ui_text_muted)>
                                                {t!(i18n, scenes.error_invalid_grid)}
                                            </div>
                                        }.into_any();
                                    };

                                    let (vw, vh) = viewport_size();
                                    let board = fit_preview_layout(columns, rows, vw, vh);
                                    let stroke_minor = "rgba(255,255,255,0.18)";
                                    let stroke_major = "rgba(255,255,255,0.32)";
                                    let line_width = (1.0 / board.scale.max(0.35)).min(2.8);
                                    let preview_url = vm.draft_background.get().and_then(|f| {
                                        if f.mime_type.starts_with("image/") {
                                            file_transfer.file_urls.get().get(&f.hash).cloned()
                                        } else { None }
                                    });
                                    let transfer_status = vm.draft_background.get().and_then(|f| {
                                        file_transfer.transfer_statuses.get().get(&f.hash).cloned()
                                    });

                                    view! {
                                        <div style=format!("position: relative; width: {:.2}px; height: {:.2}px; transform: scale({:.4}); transform-origin: center center; flex: 0 0 auto;", board.board_width, board.board_height, board.scale)>
                                            <div style=format!("position: relative; width: {:.2}px; height: {:.2}px; border: 0.125rem solid {}; border-radius: 1rem; overflow: hidden; background: linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.22)), {}; box-shadow: 0 24px 80px rgba(0,0,0,0.32), inset 0 0 0 0.0625rem rgba(255,255,255,0.06);", board.board_width, board.board_height, theme.ui_success, theme.ui_bg_primary)>
                                                {match preview_url {
                                                    Some(url) => view! {
                                                        <>
                                                            <img src=url
                                                                alt=move || vm.draft_name.get()
                                                                style=format!("position: absolute; left: 50%; top: 50%; width: {:.2}px; max-width: none; pointer-events: none; transform: translate(-50%, -50%) translate({:.2}px, {:.2}px) scale({:.4}) rotate({:.2}deg); opacity: 0.94;",
                                                                    board.board_width,
                                                                    vm.draft_background_offset_x.get(),
                                                                    vm.draft_background_offset_y.get(),
                                                                    vm.draft_background_scale.get().max(0.05),
                                                                    vm.draft_background_rotation_deg.get()
                                                                )
                                                            />
                                                            <div
                                                                on:mousedown=move |e: MouseEvent| {
                                                                    e.prevent_default();
                                                                    e.stop_propagation();
                                                                    vm.start_background_drag(e.client_x(), e.client_y(), board.scale);
                                                                }
                                                                style=move || format!("position: absolute; inset: 0; cursor: {}; background: transparent;",
                                                                    if vm.is_dragging_background.get() { "grabbing" } else { "grab" }
                                                                )
                                                            />
                                                        </>
                                                    }.into_any(),
                                                    None => match transfer_status {
                                                        Some(s) if s.stage != FileTransferStage::Complete => view! {
                                                            <div style=format!("position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: {}; font-size: 0.95rem; background: rgba(0,0,0,0.18);", theme.ui_text_secondary)>
                                                                {format!("{} {}%", t_string!(i18n, scenes.background_status), s.progress_percent())}
                                                            </div>
                                                        }.into_any(),
                                                        _ => view! {
                                                            <div style=format!("position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: {}; font-size: 0.95rem;", theme.ui_text_muted)>
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
                                                    {(0..=columns).filter(|c| c % 5 != 0).map(|c| {
                                                        let x = f64::from(c) * board.cell_size;
                                                        view! { <line x1=format!("{x:.4}") y1="0" x2=format!("{x:.4}") y2=format!("{:.4}", board.board_height) stroke=stroke_minor stroke-width=format!("{line_width:.4}") /> }
                                                    }).collect_view()}
                                                    {(0..=rows).filter(|r| r % 5 != 0).map(|r| {
                                                        let y = f64::from(r) * board.cell_size;
                                                        view! { <line x1="0" y1=format!("{y:.4}") x2=format!("{:.4}", board.board_width) y2=format!("{y:.4}") stroke=stroke_minor stroke-width=format!("{line_width:.4}") /> }
                                                    }).collect_view()}
                                                    {(0..=columns).filter(|c| c % 5 == 0).map(|c| {
                                                        let x = f64::from(c) * board.cell_size;
                                                        view! { <line x1=format!("{x:.4}") y1="0" x2=format!("{x:.4}") y2=format!("{:.4}", board.board_height) stroke=stroke_major stroke-width=format!("{:.4}", line_width * 1.15) /> }
                                                    }).collect_view()}
                                                    {(0..=rows).filter(|r| r % 5 == 0).map(|r| {
                                                        let y = f64::from(r) * board.cell_size;
                                                        view! { <line x1="0" y1=format!("{y:.4}") x2=format!("{:.4}", board.board_width) y2=format!("{y:.4}") stroke=stroke_major stroke-width=format!("{:.4}", line_width * 1.15) /> }
                                                    }).collect_view()}
                                                </svg>
                                            </div>
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        </div>

                        // Sidebar controls
                        <div style=format!("width: min(34rem, 100vw); height: 100vh; background: linear-gradient(180deg, {}, {}); border-left: 0.0625rem solid {}; box-shadow: -24px 0 64px rgba(0,0,0,0.35); display: flex; flex-direction: column;", theme.ui_bg_primary, theme.ui_bg_secondary, theme.ui_border)>
                            <div style=format!("padding: 1.4rem 1.5rem 1rem; border-bottom: 0.0625rem solid {}; display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start;", theme.ui_border)>
                                <div style="min-width: 0;">
                                    <div style=format!("color: {}; font-size: 1.2rem; font-weight: 800;", theme.ui_text_primary)>
                                        {move || t!(i18n, scenes.background_fit_modal_title)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.84rem; margin-top: 0.45rem; line-height: 1.45;", theme.ui_text_secondary)>
                                        {move || t!(i18n, scenes.background_fit_modal_hint)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.78rem; margin-top: 0.55rem;", theme.ui_text_muted)>
                                        {move || {
                                            vm.selected_scene_id.get()
                                                .and_then(|id| scenes.get().into_iter().find(|s| s.id == id).map(|s| s.name))
                                                .unwrap_or_default()
                                        }}
                                    </div>
                                </div>
                                <button type="button"
                                    on:click=move |_| vm.close_background_fit_editor()
                                    style=format!("background: {}; border: none; color: {}; padding: 0.35rem 0.75rem; border-radius: 0.375rem; cursor: pointer; font-size: 1rem; font-weight: 700;", theme.ui_button_danger, theme.ui_text_primary)
                                >
                                    "x"
                                </button>
                            </div>

                            <div style="flex: 1; overflow-y: auto; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;">
                                <div style=format!("padding: 0.9rem 1rem; border-radius: 0.75rem; background: rgba(255,255,255,0.04); border: 0.0625rem solid {}; color: {}; font-size: 0.82rem; line-height: 1.5;", theme.ui_border, theme.ui_text_secondary)>
                                    {move || t!(i18n, scenes.background_fit_live_preview_hint)}
                                </div>

                                <div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.85rem;">
                                    <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                        <span>{move || t!(i18n, scenes.columns_label)}</span>
                                        <input type="number" min="1" max="200"
                                            prop:value=move || vm.draft_columns.get()
                                            on:input=move |ev| vm.draft_columns.set(event_target_value(&ev))
                                            style=format!("padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                        />
                                    </label>
                                    <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                        <span>{move || t!(i18n, scenes.rows_label)}</span>
                                        <input type="number" min="1" max="200"
                                            prop:value=move || vm.draft_rows.get()
                                            on:input=move |ev| vm.draft_rows.set(event_target_value(&ev))
                                            style=format!("padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                        />
                                    </label>
                                </div>

                                // Background scale
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.background_scale_label)}</span>
                                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                                        <input type="range" min=MIN_BACKGROUND_SCALE.to_string() max=MAX_BACKGROUND_SCALE.to_string() step="0.01"
                                            prop:value=move || format!("{:.2}", vm.draft_background_scale.get())
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                                    vm.draft_background_scale.set(vm.clamp_background_scale(v));
                                                }
                                            }
                                            style="flex: 1;"
                                        />
                                        <input type="number" min=MIN_BACKGROUND_SCALE.to_string() max=MAX_BACKGROUND_SCALE.to_string() step="0.01"
                                            prop:value=move || format!("{:.2}", vm.draft_background_scale.get())
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                                    vm.draft_background_scale.set(vm.clamp_background_scale(v));
                                                }
                                            }
                                            style=format!("width: 6rem; padding: 0.65rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                        />
                                    </div>
                                </label>

                                // Background position info
                                <div style=format!("padding: 0.95rem 1rem; border-radius: 0.75rem; background: rgba(255,255,255,0.03); border: 0.0625rem solid {}; display: flex; flex-direction: column; gap: 0.55rem;", theme.ui_border)>
                                    <div style=format!("color: {}; font-size: 0.9rem; font-weight: 700;", theme.ui_text_primary)>
                                        {move || t!(i18n, scenes.background_position_label)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.82rem; line-height: 1.45;", theme.ui_text_secondary)>
                                        {move || t!(i18n, scenes.background_fit_drag_hint)}
                                    </div>
                                    <div style=format!("color: {}; font-size: 0.8rem;", theme.ui_text_muted)>
                                        {move || format!("X: {:.0}px | Y: {:.0}px", vm.draft_background_offset_x.get(), vm.draft_background_offset_y.get())}
                                    </div>
                                </div>

                                // Background rotation
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, scenes.background_rotation_label)}</span>
                                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                                        <input type="range" min=MIN_BACKGROUND_ROTATION_DEG.to_string() max=MAX_BACKGROUND_ROTATION_DEG.to_string() step="1"
                                            prop:value=move || format!("{:.0}", vm.draft_background_rotation_deg.get())
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                                    vm.draft_background_rotation_deg.set(vm.clamp_background_rotation(v));
                                                }
                                            }
                                            style="flex: 1;"
                                        />
                                        <input type="number" min=MIN_BACKGROUND_ROTATION_DEG.to_string() max=MAX_BACKGROUND_ROTATION_DEG.to_string() step="1"
                                            prop:value=move || format!("{:.0}", vm.draft_background_rotation_deg.get())
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                                    vm.draft_background_rotation_deg.set(vm.clamp_background_rotation(v));
                                                }
                                            }
                                            style=format!("width: 6rem; padding: 0.65rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem;", theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border)
                                        />
                                    </div>
                                </label>

                                // Footer buttons
                                <div style=format!("margin-top: auto; display: flex; gap: 0.75rem; justify-content: flex-end; padding-top: 0.5rem; border-top: 0.0625rem solid {}; flex-wrap: wrap;", theme.ui_border)>
                                    <button type="button"
                                        on:click=move |_| vm.reset_background_fit()
                                        style=format!("padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer;", theme.ui_bg_secondary, theme.ui_text_primary)
                                    >
                                        {move || t!(i18n, scenes.background_reset_fit_button)}
                                    </button>
                                    <button type="button"
                                        on:click=move |_| vm.close_background_fit_editor()
                                        style=format!("padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer;", theme.ui_bg_secondary, theme.ui_text_primary)
                                    >
                                        {move || t!(i18n, scenes.background_fit_modal_close_button)}
                                    </button>
                                    <button type="button"
                                        on:click=move |_| {
                                            save_scene(());
                                            vm.close_background_fit_editor();
                                        }
                                        style=format!("padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-weight: 700;", theme.ui_button_primary, theme.ui_text_primary)
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
