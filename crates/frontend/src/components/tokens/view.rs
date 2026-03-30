use super::model::TOKEN_IMAGE_ACCEPT;
use super::view_model::TokensWindowViewModel;
use crate::components::draggable_window::DraggableWindow;
use crate::components::websocket::{
    FileTransferState, StoredTokenLibraryItem, WsSender, delete_token_library_item,
    load_token_library, save_token_library_item,
};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev::{Event, MouseEvent};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

const TOKENS_TITLE_FONT_SIZE: &str = "clamp(1rem, 0.95rem + 0.2vw, 1.12rem)";
const TOKENS_BODY_FONT_SIZE: &str = "clamp(0.9rem, 0.87rem + 0.12vw, 0.98rem)";
const TOKENS_META_FONT_SIZE: &str = "clamp(0.74rem, 0.71rem + 0.12vw, 0.82rem)";
const TOKENS_BUTTON_FONT_SIZE: &str = "clamp(0.82rem, 0.79rem + 0.12vw, 0.9rem)";

fn sort_items(items: &mut [StoredTokenLibraryItem]) {
    items.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

#[component]
pub fn TokensWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    room_id: ReadSignal<String>,
    #[prop(into)] items: RwSignal<Vec<StoredTokenLibraryItem>>,
    file_transfer: FileTransferState,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    on_start_drag: Callback<StoredTokenLibraryItem>,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let vm = TokensWindowViewModel::new();
    let input_ref = NodeRef::<html::Input>::new();
    let file_transfer_for_load = file_transfer.clone();
    let file_transfer_for_save = file_transfer.clone();
    let file_transfer_for_import = file_transfer.clone();

    Effect::new(move |_| {
        if !is_open.get() {
            return;
        }

        let current_room = room_id.get();
        if current_room.is_empty() {
            items.set(Vec::new());
            vm.reset();
            return;
        }

        let items_signal = items;
        let file_transfer = file_transfer_for_load.clone();
        spawn_local(async move {
            match load_token_library(&current_room).await {
                Ok(loaded_items) => {
                    if room_id.get_untracked() != current_room {
                        return;
                    }
                    let files = loaded_items
                        .iter()
                        .map(|item| item.image.clone())
                        .collect::<Vec<_>>();
                    file_transfer.hydrate_local_files(&files);
                    items_signal.set(loaded_items);
                }
                Err(error) => {
                    if room_id.get_untracked() == current_room {
                        vm.error.set(Some(error));
                    }
                }
            }
        });
        vm.reset();
    });

    let save_item = move |_| {
        let current_room = room_id.get_untracked();
        if current_room.is_empty() {
            return;
        }

        let Some(item) = vm.build_item(
            &current_room,
            &t_string!(i18n, tokens.error_name_required),
            &t_string!(i18n, tokens.error_image_required),
            &t_string!(i18n, tokens.error_dimensions_invalid),
        ) else {
            return;
        };

        let items_signal = items;
        let vm = vm;
        let file_transfer = file_transfer_for_save.clone();
        spawn_local(async move {
            match save_token_library_item(&item).await {
                Ok(()) => {
                    file_transfer.hydrate_local_files(std::slice::from_ref(&item.image));
                    items_signal.update(|items| {
                        match items.iter_mut().find(|existing| existing.id == item.id) {
                            Some(existing) => *existing = item.clone(),
                            None => items.push(item.clone()),
                        }
                        sort_items(items);
                    });
                    vm.reset();
                }
                Err(error) => vm.error.set(Some(error)),
            }
        });
    };

    let on_image_selected = {
        let file_transfer = file_transfer_for_import.clone();
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
            let sender = ws_sender.get_untracked();
            let username = username.get_untracked();
            spawn_local(async move {
                match file_transfer
                    .import_browser_file(file, username, sender)
                    .await
                {
                    Ok(file_ref) if file_ref.mime_type.starts_with("image/") => {
                        vm.draft_image.set(Some(file_ref));
                        vm.error.set(None);
                    }
                    Ok(_) => vm.error.set(Some(
                        t_string!(i18n, tokens.error_image_must_be_image).to_string(),
                    )),
                    Err(error) => vm.error.set(Some(error)),
                }
            });
            input.set_value("");
        }
    };

    view! {
        <DraggableWindow
            is_open=is_open
            title=move || t_string!(i18n, tokens.title)
            initial_x=880
            initial_y=110
            initial_width=430
            initial_height=640
            min_width=360
            min_height=360
            is_active=is_active
            on_focus=on_focus.unwrap_or_else(|| Callback::new(|_| {}))
            theme=theme.clone()
        >
            <div style="display: flex; flex-direction: column; flex: 1; min-height: 0; padding: 1rem; gap: 0.9rem;">
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 0.75rem;">
                    <div>
                        <div style=format!(
                            "font-size: {}; font-weight: 700; color: {}; line-height: 1.2;",
                            TOKENS_TITLE_FONT_SIZE, theme.ui_text_primary
                        )>
                            {t!(i18n, tokens.library_title)}
                        </div>
                        <div style=format!(
                            "font-size: {}; color: {}; margin-top: 0.15rem; line-height: 1.4;",
                            TOKENS_META_FONT_SIZE, theme.ui_text_secondary
                        )>
                            {t!(i18n, tokens.library_hint)}
                        </div>
                    </div>
                    <button
                        on:click=move |_| vm.reset()
                        style=format!(
                            "padding: 0.45rem 0.7rem; border: 1px solid {}; border-radius: 0.7rem; \
                             background: transparent; color: {}; cursor: pointer; font-size: {};",
                            theme.ui_border, theme.ui_text_secondary, TOKENS_BUTTON_FONT_SIZE
                        )
                    >
                        {t!(i18n, tokens.new_button)}
                    </button>
                </div>

                <div style="display: flex; flex-direction: column; gap: 0.75rem;">
                    <input
                        type="text"
                        prop:value=move || vm.draft_name.get()
                        on:input=move |event| vm.draft_name.set(event_target_value(&event))
                        placeholder=move || t_string!(i18n, tokens.name_placeholder)
                        style=format!(
                            "padding: 0.7rem 0.8rem; border: 1px solid {}; border-radius: 0.75rem; \
                             background: {}; color: {}; outline: none; font-size: {};",
                            theme.ui_border, theme.ui_bg_secondary, theme.ui_text_primary, TOKENS_BODY_FONT_SIZE
                        )
                    />
                    <div style="display: grid; grid-template-columns: 5.5rem 5.5rem 1fr auto; gap: 0.6rem; align-items: center;">
                        <input
                            type="number"
                            min="1"
                            max="16"
                            placeholder=move || t_string!(i18n, tokens.width_placeholder)
                            prop:value=move || vm.draft_width_cells.get()
                            on:input=move |event| vm.draft_width_cells.set(event_target_value(&event))
                            style=format!(
                                "padding: 0.7rem 0.8rem; border: 1px solid {}; border-radius: 0.75rem; \
                                 background: {}; color: {}; outline: none; font-size: {};",
                                theme.ui_border, theme.ui_bg_secondary, theme.ui_text_primary, TOKENS_BODY_FONT_SIZE
                            )
                        />
                        <input
                            type="number"
                            min="1"
                            max="16"
                            placeholder=move || t_string!(i18n, tokens.height_placeholder)
                            prop:value=move || vm.draft_height_cells.get()
                            on:input=move |event| vm.draft_height_cells.set(event_target_value(&event))
                            style=format!(
                                "padding: 0.7rem 0.8rem; border: 1px solid {}; border-radius: 0.75rem; \
                                 background: {}; color: {}; outline: none; font-size: {};",
                                theme.ui_border, theme.ui_bg_secondary, theme.ui_text_primary, TOKENS_BODY_FONT_SIZE
                            )
                        />
                        <button
                            on:click=move |_| {
                                if let Some(input) = input_ref.get() {
                                    input.click();
                                }
                            }
                            style=format!(
                                "padding: 0.7rem 0.8rem; border: 1px solid {}; border-radius: 0.75rem; \
                                 background: {}; color: {}; cursor: pointer; font-size: {};",
                                theme.ui_border, theme.ui_bg_secondary, theme.ui_text_primary, TOKENS_BUTTON_FONT_SIZE
                            )
                        >
                            {move || {
                                if vm.draft_image.get().is_some() {
                                    t_string!(i18n, tokens.change_image_button)
                                } else {
                                    t_string!(i18n, tokens.select_image_button)
                                }
                            }}
                        </button>
                        <button
                            on:click=save_item
                            style=format!(
                                "padding: 0.7rem 0.9rem; border: none; border-radius: 0.75rem; \
                                 background: {}; color: {}; cursor: pointer; font-weight: 700; font-size: {};",
                                theme.ui_button_primary, theme.ui_text_primary, TOKENS_BUTTON_FONT_SIZE
                            )
                        >
                            {move || {
                                if vm.editing_id.get().is_some() {
                                    t_string!(i18n, tokens.save_button)
                                } else {
                                    t_string!(i18n, tokens.create_button)
                                }
                            }}
                        </button>
                    </div>
                    <input
                        node_ref=input_ref
                        type="file"
                        accept=TOKEN_IMAGE_ACCEPT
                        on:change=on_image_selected
                        style="display: none;"
                    />
                    <Show when=move || vm.draft_image.get().is_some()>
                        <div style=format!(
                            "padding: 0.6rem 0.75rem; border-radius: 0.75rem; background: {}; color: {}; font-size: {}; line-height: 1.4;",
                            theme.ui_bg_secondary, theme.ui_text_secondary, TOKENS_META_FONT_SIZE
                        )>
                            {move || vm.draft_image.get().map(|image| format!("{} ({})", image.file_name, image.mime_type)).unwrap_or_default()}
                        </div>
                    </Show>
                    <Show when=move || vm.error.get().is_some()>
                        <div style=format!(
                            "padding: 0.7rem 0.8rem; border-radius: 0.75rem; background: rgba(220, 38, 38, 0.18); \
                             color: #fecaca; font-size: {}; line-height: 1.4;",
                            TOKENS_BODY_FONT_SIZE
                        )>
                            {move || vm.error.get().unwrap_or_default()}
                        </div>
                    </Show>
                </div>

                <div style="display: flex; flex-direction: column; gap: 0.7rem; min-height: 0; overflow-y: auto; padding-right: 0.15rem;">
                    <For
                        each=move || items.get()
                        key=|item| item.id.clone()
                        children=move |item| {
                            let item_for_drag = item.clone();
                            let item_for_edit = item.clone();
                            let item_for_delete = item.clone();
                            let item_image_hash = item.image.hash.clone();
                            let item_image_hash_for_src = item.image.hash.clone();
                            let item_name = item.name.clone();
                            let item_name_for_alt = item.name.clone();
                            let item_width_cells = item.width_cells;
                            let item_height_cells = item.height_cells;
                            let item_file_name = item.image.file_name.clone();
                            let ui_border = theme.ui_border.to_string();
                            let text_primary = theme.ui_text_primary.to_string();
                            let text_secondary = theme.ui_text_secondary.to_string();
                            let text_secondary_for_fallback = text_secondary.clone();
                            let bg_secondary = theme.ui_bg_secondary.to_string();
                            let transfer_for_presence = file_transfer.clone();
                            let transfer_for_src = file_transfer.clone();
                            let image_src = Signal::derive(move || {
                                transfer_for_src
                                    .file_urls
                                    .get()
                                    .get(&item_image_hash_for_src)
                                    .cloned()
                                    .unwrap_or_default()
                            });
                            let image_alt = Signal::derive(move || item_name_for_alt.clone());
                            view! {
                                <div style=format!(
                                    "display: grid; grid-template-columns: 4.5rem 1fr auto; gap: 0.75rem; align-items: center; \
                                     padding: 0.7rem; border: 1px solid {}; border-radius: 0.9rem; background: {};",
                                    ui_border, bg_secondary
                                )>
                                    <div
                                        on:mousedown=move |event: MouseEvent| {
                                            event.prevent_default();
                                            on_start_drag.run(item_for_drag.clone());
                                        }
                                        style="width: 4.5rem; height: 4.5rem; border-radius: 0.75rem; overflow: hidden; cursor: grab; background: rgba(0,0,0,0.2); display: flex; align-items: center; justify-content: center;"
                                    >
                                        <Show
                                            when=move || transfer_for_presence.file_urls.get().contains_key(&item_image_hash)
                                            fallback=move || view! {
                                                <span style=format!(
                                                    "font-size: {}; color: {};",
                                                    TOKENS_META_FONT_SIZE, text_secondary_for_fallback
                                                )>
                                                    "IMG"
                                                </span>
                                            }
                                        >
                                            <img
                                                src=image_src
                                                alt=image_alt
                                                style="width: 100%; height: 100%; object-fit: cover;"
                                            />
                                        </Show>
                                    </div>
                                    <div style="min-width: 0;">
                                        <div style=format!(
                                            "font-size: {}; font-weight: 700; color: {}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                            TOKENS_BODY_FONT_SIZE, text_primary
                                        )>
                                            {item_name}
                                        </div>
                                        <div style=format!(
                                            "font-size: {}; color: {}; margin-top: 0.15rem;",
                                            TOKENS_META_FONT_SIZE, text_secondary
                                        )>
                                            {format!(
                                                "{} x {} {}",
                                                item_width_cells,
                                                item_height_cells,
                                                t_string!(i18n, tokens.cells_suffix)
                                            )}
                                        </div>
                                        <div style=format!(
                                            "font-size: {}; color: {}; margin-top: 0.2rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                            TOKENS_META_FONT_SIZE, text_secondary
                                        )>
                                            {item_file_name}
                                        </div>
                                    </div>
                                    <div style="display: flex; flex-direction: column; gap: 0.45rem;">
                                        <button
                                            on:click=move |_| vm.apply_item(&item_for_edit)
                                            style=format!(
                                                "padding: 0.42rem 0.6rem; border: 1px solid {}; border-radius: 0.65rem; background: transparent; color: {}; cursor: pointer; font-size: {};",
                                                ui_border, text_primary, TOKENS_BUTTON_FONT_SIZE
                                            )
                                        >
                                            {t!(i18n, tokens.edit_button)}
                                        </button>
                                        <button
                                            on:click=move |_| {
                                                let items_signal = items;
                                                let deleting_id = item_for_delete.id.clone();
                                                let editing_id = item_for_delete.id.clone();
                                                let current_room = room_id.get_untracked();
                                                spawn_local(async move {
                                                    match delete_token_library_item(&current_room, &deleting_id).await {
                                                        Ok(()) => {
                                                            items_signal.update(|items| items.retain(|item| item.id != deleting_id));
                                                            if vm.editing_id.get_untracked().as_deref() == Some(editing_id.as_str()) {
                                                                vm.reset();
                                                            }
                                                        }
                                                        Err(error) => vm.error.set(Some(error)),
                                                    }
                                                });
                                            }
                                            style=format!(
                                                "padding: 0.42rem 0.6rem; border: none; border-radius: 0.65rem; \
                                                 background: rgba(220, 38, 38, 0.16); color: #fecaca; cursor: pointer; font-size: {};",
                                                TOKENS_BUTTON_FONT_SIZE
                                            )
                                        >
                                            {t!(i18n, tokens.delete_button)}
                                        </button>
                                    </div>
                                </div>
                            }
                        }
                    />
                    <Show when=move || items.get().is_empty()>
                        <div style=format!(
                            "padding: 1rem; border: 1px dashed {}; border-radius: 0.9rem; color: {}; font-size: {}; \
                             text-align: center; line-height: 1.45;",
                            theme.ui_border, theme.ui_text_secondary, TOKENS_BODY_FONT_SIZE
                        )>
                            {t!(i18n, tokens.empty_state)}
                        </div>
                    </Show>
                </div>
            </div>
        </DraggableWindow>
    }
}
