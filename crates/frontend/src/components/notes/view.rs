use super::model::{
    BOARD_NOTE_DRAG_MIME, NotesTab, can_delete_note, can_edit_note, note_heading_and_body,
    recipients_cache_is_stale, render_note_html, sort_notes,
};
use super::view_model::NotesViewModel;
use crate::components::draggable_window::DraggableWindow;
use crate::components::tab_bar::{TabBar, TabItem};
use crate::components::websocket::{StoredNoteBucket, WsSender, delete_note, save_note};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::task::spawn_local_scoped;
use shared::events::PresenceRequestPayload;
use shared::events::{ClientEvent, NoteDeletePayload, NotePayload, NoteVisibility};
use web_sys::DragEvent;

const NOTE_DRAG_LABEL_FONT_SIZE: &str = "0.74rem";
const NOTE_META_FONT_SIZE: &str = "0.78rem";
const NOTE_BODY_FONT_SIZE: &str = "0.92rem";
const RECIPIENT_REQUEST_TIMEOUT_MS: u32 = 1500;

fn current_time_ms() -> f64 {
    js_sys::Date::now()
}

fn upsert_note(notes: &mut Vec<NotePayload>, note: NotePayload) {
    match notes.iter_mut().find(|existing| existing.id == note.id) {
        Some(existing) => *existing = note,
        None => notes.push(note),
    }
    sort_notes(notes);
}

fn remove_note(notes: &mut Vec<NotePayload>, note_id: &str) {
    notes.retain(|note| note.id != note_id);
}

fn notes_for_tab(
    tab: NotesTab,
    public_notes: Vec<NotePayload>,
    private_notes: Vec<NotePayload>,
    direct_notes: Vec<NotePayload>,
) -> Vec<NotePayload> {
    match tab {
        NotesTab::Public => public_notes,
        NotesTab::Private => private_notes,
        NotesTab::Direct => direct_notes,
    }
}

fn note_scope_label(note: &NotePayload, current_username: &str) -> String {
    match &note.visibility {
        NoteVisibility::Public => format!("@{}", note.author),
        NoteVisibility::Private => format!("@{} | private", note.author),
        NoteVisibility::Direct(recipient) if note.author == current_username => {
            format!("@{} -> @{}", note.author, recipient)
        }
        NoteVisibility::Direct(_) => format!("@{} -> you", note.author),
    }
}

fn recipient_options(
    current_recipient: &str,
    recipients: &[String],
    current_username: &str,
) -> Vec<String> {
    let mut options = recipients
        .iter()
        .filter(|username| username.as_str() != current_username)
        .cloned()
        .collect::<Vec<_>>();
    options.sort();
    options.dedup();

    if !current_recipient.is_empty() && !options.iter().any(|option| option == current_recipient) {
        options.insert(0, current_recipient.to_string());
    }

    options
}

#[component]
pub fn NotesWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    room_id: ReadSignal<String>,
    username: ReadSignal<String>,
    ws_sender: ReadSignal<Option<WsSender>>,
    #[prop(into)] public_notes: RwSignal<Vec<NotePayload>>,
    #[prop(into)] private_notes: RwSignal<Vec<NotePayload>>,
    #[prop(into)] direct_notes: RwSignal<Vec<NotePayload>>,
    #[prop(into)] direct_note_recipients: RwSignal<Vec<String>>,
    #[prop(into)] direct_note_recipients_cache_updated_at_ms: RwSignal<Option<f64>>,
    #[prop(into)] direct_note_recipients_request_id: RwSignal<Option<String>>,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let vm = NotesViewModel::new();

    let tabs = move || {
        vec![
            TabItem::new(NotesTab::Public, t_string!(i18n, notes.tab_public)),
            TabItem::new(NotesTab::Private, t_string!(i18n, notes.tab_private)),
            TabItem::new(NotesTab::Direct, t_string!(i18n, notes.tab_direct)),
        ]
    };

    let refresh_recipients = {
        move || {
            let Some(sender) = ws_sender.get_untracked() else {
                vm.error_message
                    .set(Some("WebSocket connection is not available".to_string()));
                return;
            };
            let current_user = username.get_untracked();
            let requested_at_ms = current_time_ms();
            let request_id = format!(
                "notes_recipients:{}:{}",
                current_user, requested_at_ms as u64
            );

            direct_note_recipients_request_id.set(Some(request_id.clone()));
            vm.is_loading_recipients.set(true);
            vm.error_message.set(None);

            if let Err(error) =
                sender.try_send_event(ClientEvent::PresenceRequest(PresenceRequestPayload {
                    request_id: request_id.clone(),
                    requester: current_user,
                }))
            {
                direct_note_recipients_request_id.set(None);
                vm.is_loading_recipients.set(false);
                vm.error_message.set(Some(error));
                return;
            }

            direct_note_recipients.set(Vec::new());
            direct_note_recipients_cache_updated_at_ms.set(Some(requested_at_ms));

            spawn_local_scoped(async move {
                TimeoutFuture::new(RECIPIENT_REQUEST_TIMEOUT_MS).await;
                if direct_note_recipients_request_id.get_untracked().as_deref()
                    == Some(request_id.as_str())
                {
                    vm.is_loading_recipients.set(false);
                }
            });
        }
    };
    let refresh_recipients = StoredValue::new(refresh_recipients);

    Effect::new(move |_| {
        if !is_open.get() || vm.active_tab.get() != NotesTab::Direct {
            return;
        }

        let cache_updated_at_ms = direct_note_recipients_cache_updated_at_ms.get();
        if recipients_cache_is_stale(cache_updated_at_ms, current_time_ms()) {
            refresh_recipients.with_value(|refresh| refresh());
        }
    });

    Effect::new(move |_| {
        let recipients = direct_note_recipients.get();
        if vm.active_tab.get() != NotesTab::Direct {
            return;
        }

        let options = recipient_options(
            &vm.recipient.get_untracked(),
            &recipients,
            &username.get_untracked(),
        );
        if vm.recipient.get_untracked().is_empty()
            && let Some(first) = options.first()
        {
            vm.recipient.set(first.clone());
        }

        if !vm.is_loading_recipients.get_untracked() {
            return;
        }
        if !recipients.is_empty() {
            vm.is_loading_recipients.set(false);
        }
    });

    let submit_note = move || {
        let current_user = username.get_untracked();
        let current_room = room_id.get_untracked();
        let existing_note = vm.editing_note_id.get_untracked().and_then(|editing_id| {
            notes_for_tab(
                vm.active_tab.get_untracked(),
                public_notes.get_untracked(),
                private_notes.get_untracked(),
                direct_notes.get_untracked(),
            )
            .into_iter()
            .find(|note| note.id == editing_id)
        });
        let board_position = existing_note
            .as_ref()
            .and_then(|note| note.board_position.clone());
        let board_style = existing_note
            .as_ref()
            .map(|note| note.board_style.clone())
            .unwrap_or_default();
        let note = match vm.build_note(&current_user, board_position, board_style) {
            Ok(note) => note,
            Err(error) => {
                vm.error_message.set(Some(error));
                return;
            }
        };

        match &note.visibility {
            NoteVisibility::Public | NoteVisibility::Direct(_) => {
                let Some(sender) = ws_sender.get_untracked() else {
                    vm.error_message
                        .set(Some("WebSocket connection is not available".to_string()));
                    return;
                };
                if let Err(error) = sender.try_send_event(ClientEvent::NoteUpsert(note)) {
                    vm.error_message.set(Some(error));
                    return;
                }
            }
            NoteVisibility::Private => {
                private_notes.update(|notes| upsert_note(notes, note.clone()));
                spawn_local(async move {
                    let _ = save_note(
                        &current_room,
                        &current_user,
                        StoredNoteBucket::Private,
                        &note,
                    )
                    .await;
                });
            }
        }

        vm.reset_form();
    };

    let delete_note_action = move |note: NotePayload| {
        let current_room = room_id.get_untracked();
        let current_user = username.get_untracked();
        let note_id = note.id.clone();
        match note.visibility.clone() {
            NoteVisibility::Public => {
                if let Some(sender) = ws_sender.get_untracked() {
                    let _ = sender.try_send_event(ClientEvent::NoteDelete(NoteDeletePayload {
                        id: note_id.clone(),
                        author: note.author,
                        visibility: note.visibility,
                    }));
                } else {
                    vm.error_message
                        .set(Some("WebSocket connection is not available".to_string()));
                }
            }
            NoteVisibility::Direct(_) => {
                if note.author == current_user {
                    if let Some(sender) = ws_sender.get_untracked() {
                        let _ = sender.try_send_event(ClientEvent::NoteDelete(NoteDeletePayload {
                            id: note_id.clone(),
                            author: note.author,
                            visibility: note.visibility,
                        }));
                    } else {
                        vm.error_message
                            .set(Some("WebSocket connection is not available".to_string()));
                    }
                } else {
                    direct_notes.update(|notes| remove_note(notes, &note_id));
                    let note_id_for_task = note_id.clone();
                    spawn_local(async move {
                        let _ = delete_note(
                            &current_room,
                            &current_user,
                            StoredNoteBucket::Direct,
                            &note_id_for_task,
                        )
                        .await;
                    });
                }
            }
            NoteVisibility::Private => {
                private_notes.update(|notes| remove_note(notes, &note_id));
                let note_id_for_task = note_id.clone();
                spawn_local(async move {
                    let _ = delete_note(
                        &current_room,
                        &current_user,
                        StoredNoteBucket::Private,
                        &note_id_for_task,
                    )
                    .await;
                });
            }
        }

        if vm.editing_note_id.get_untracked().as_deref() == Some(note_id.as_str()) {
            vm.reset_form();
        }
    };
    let delete_note_action = StoredValue::new(delete_note_action);

    let unpin_note_action = move |note: NotePayload| {
        let mut updated = note.clone();
        updated.board_position = None;
        updated.updated_at_ms = js_sys::Date::now();
        let current_room = room_id.get_untracked();
        let current_user = username.get_untracked();
        match updated.visibility.clone() {
            NoteVisibility::Public | NoteVisibility::Direct(_) => {
                if let Some(sender) = ws_sender.get_untracked() {
                    let _ = sender.try_send_event(ClientEvent::NoteUpsert(updated));
                } else {
                    vm.error_message
                        .set(Some("WebSocket connection is not available".to_string()));
                }
            }
            NoteVisibility::Private => {
                private_notes.update(|notes| upsert_note(notes, updated.clone()));
                spawn_local(async move {
                    let _ = save_note(
                        &current_room,
                        &current_user,
                        StoredNoteBucket::Private,
                        &updated,
                    )
                    .await;
                });
            }
        }
    };
    let unpin_note_action = StoredValue::new(unpin_note_action);

    view! {
        <DraggableWindow
            is_open=is_open
            title=move || t_string!(i18n, notes.title)
            initial_x=210
            initial_y=120
            initial_width=520
            initial_height=680
            min_width=380
            min_height=320
            is_active=is_active
            on_focus=on_focus.unwrap_or_else(|| Callback::new(|_| {}))
            theme=theme.clone()
        >
            <div style="display: flex; flex-direction: column; height: 100%;">
                <TabBar
                    tabs=tabs
                    active_tab=vm.active_tab
                    theme=theme.clone()
                />

                <div style="display: flex; flex-direction: column; gap: 0.9rem; padding: 0.5rem 1rem 0.75rem 1rem; border-bottom: 0.0625rem solid rgba(255,255,255,0.08);">
                    {move || if vm.active_tab.get() == NotesTab::Direct {
                        view! {
                            <div style="display: flex; flex-direction: column; gap: 0.45rem;">
                                <div style="display: flex; gap: 0.6rem; align-items: center;">
                                    <select
                                        prop:value=move || vm.recipient.get()
                                        on:change=move |ev| vm.recipient.set(event_target_value(&ev))
                                        style=format!(
                                            "flex: 1; min-width: 0; padding: 0.7rem 0.8rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; box-sizing: border-box;",
                                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border
                                        )
                                    >
                                        <option value="">{move || t!(i18n, notes.recipient_placeholder)}</option>
                                        <For
                                            each=move || recipient_options(
                                                &vm.recipient.get(),
                                                &direct_note_recipients.get(),
                                                &username.get(),
                                            )
                                            key=|recipient| recipient.clone()
                                            children=move |recipient| {
                                                let label = recipient.clone();
                                                view! {
                                                    <option value=recipient>{label}</option>
                                                }
                                            }
                                        />
                                    </select>
                                    <button
                                        on:click=move |_| refresh_recipients.with_value(|refresh| refresh())
                                        style=format!(
                                            "padding: 0.7rem 0.9rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; white-space: nowrap;",
                                            theme.ui_bg_secondary, theme.ui_text_primary
                                        )
                                    >
                                        {t!(i18n, notes.refresh_recipients_button)}
                                    </button>
                                </div>

                                <div style=format!("display: flex; justify-content: space-between; gap: 0.75rem; color: {}; font-size: 0.78rem;", theme.ui_text_secondary)>
                                    <span>
                                        {move || if vm.is_loading_recipients.get() {
                                            t_string!(i18n, notes.recipients_loading).to_string()
                                        } else if recipient_options(
                                            &vm.recipient.get(),
                                            &direct_note_recipients.get(),
                                            &username.get(),
                                        ).is_empty() {
                                            t_string!(i18n, notes.no_active_recipients).to_string()
                                        } else {
                                            format!(
                                                "{} {}",
                                                recipient_options(
                                                    &vm.recipient.get(),
                                                    &direct_note_recipients.get(),
                                                    &username.get(),
                                                )
                                                .len(),
                                                t_string!(i18n, notes.active_recipients_count)
                                            )
                                        }}
                                    </span>
                                    <span>
                                        {move || {
                                            direct_note_recipients_cache_updated_at_ms.get().map(|updated_at| {
                                                let age_seconds =
                                                    ((current_time_ms() - updated_at) / 1000.0).max(0.0).round() as u64;
                                                format!(
                                                    "{} {}s",
                                                    t_string!(i18n, notes.recipients_updated_prefix),
                                                    age_seconds
                                                )
                                            }).unwrap_or_else(|| t_string!(i18n, notes.recipients_not_loaded).to_string())
                                        }}
                                    </span>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}

                    <textarea
                        prop:value=move || vm.body.get()
                        on:input=move |ev| vm.body.set(event_target_value(&ev))
                        placeholder=move || t_string!(i18n, notes.body_placeholder)
                        style=format!(
                            "width: 100%; min-height: 8rem; resize: vertical; padding: 0.8rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; box-sizing: border-box; font-family: inherit;",
                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border
                        )
                    ></textarea>

                    {move || vm.error_message.get().map(|error| view! {
                        <div style=format!("color: {}; font-size: 0.84rem;", theme.ui_button_danger)>{error}</div>
                    })}

                    <div style="display: flex; gap: 0.75rem; justify-content: flex-end;">
                        <button
                            on:click=move |_| vm.reset_form()
                            style=format!(
                                "padding: 0.7rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer;",
                                theme.ui_bg_secondary, theme.ui_text_primary
                            )
                        >
                            {move || if vm.editing_note_id.get().is_some() {
                                t_string!(i18n, notes.cancel_button).to_string()
                            } else {
                                t_string!(i18n, notes.clear_button).to_string()
                            }}
                        </button>
                        <button
                            on:click=move |_| submit_note()
                            style=format!(
                                "padding: 0.7rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-weight: 600;",
                                theme.ui_button_primary, theme.ui_text_primary
                            )
                        >
                            {move || if vm.editing_note_id.get().is_some() {
                                t_string!(i18n, notes.save_button).to_string()
                            } else {
                                t_string!(i18n, notes.create_button).to_string()
                            }}
                        </button>
                    </div>
                </div>

                <div style="flex: 1; overflow-y: auto; padding: 0.9rem 1rem 3rem 1rem; display: flex; flex-direction: column; gap: 0.9rem;">
                    {move || {
                        let tab_notes = notes_for_tab(
                            vm.active_tab.get(),
                            public_notes.get(),
                            private_notes.get(),
                            direct_notes.get(),
                        );
                        let current_username = username.get();
                        if tab_notes.is_empty() {
                            return view! {
                                <div style=format!(
                                    "padding: 2rem 1rem; text-align: center; color: {}; font-style: italic;",
                                    theme.ui_text_muted
                                )>
                                    {t!(i18n, notes.empty_state)}
                                </div>
                            }.into_any();
                        }

                        view! {
                            <For
                                each=move || notes_for_tab(
                                    vm.active_tab.get(),
                                    public_notes.get(),
                                    private_notes.get(),
                                    direct_notes.get(),
                                )
                                key=|note| format!("{:?}:{}", note.visibility, note.id)
                                children=move |note| {
                                    let current_username = current_username.clone();
                                    let can_edit = can_edit_note(&note, &current_username);
                                    let can_delete = can_delete_note(&note, &current_username);
                                    let drag_note = note.clone();
                                    let (display_title, display_body) = note_heading_and_body(&note.body);
                                    let rendered_html = render_note_html(&display_body);
                                    let note_for_edit = note.clone();
                                    let note_for_delete = note.clone();
                                    let note_for_unpin = note.clone();
                                    let meta = note_scope_label(&note, &current_username);
                                    view! {
                                        <article
                                            style=format!(
                                                "padding: 0.95rem; background: linear-gradient(180deg, rgba(255,255,255,0.05), rgba(0,0,0,0.08)), {}; border: 0.0625rem solid {}; border-radius: 0.8rem; box-shadow: 0 0.9rem 2rem rgba(0,0,0,0.16);",
                                                theme.ui_bg_primary, theme.ui_border
                                            )
                                        >
                                            <div style="display: flex; justify-content: space-between; align-items: flex-start; gap: 0.75rem; margin-bottom: 0.65rem;">
                                                <div style="min-width: 0;">
                                                    <div style=format!("color: {}; font-weight: 700; font-size: 1rem; word-break: break-word;", theme.ui_text_primary)>
                                                        {if display_title.trim().is_empty() {
                                                            t_string!(i18n, notes.untitled_note).to_string()
                                                        } else {
                                                            display_title.clone()
                                                        }}
                                                    </div>
                                                    <div style=format!("color: {}; font-size: {}; margin-top: 0.2rem;", theme.ui_text_secondary, NOTE_META_FONT_SIZE)>
                                                        {meta}
                                                    </div>
                                                </div>
                                                <div style="display: flex; align-items: center; gap: 0.4rem; flex-shrink: 0;">
                                                    {if can_edit {
                                                        view! {
                                                            <button
                                                                draggable="true"
                                                                on:mousedown=move |ev| ev.stop_propagation()
                                                                on:dragstart=move |event: DragEvent| {
                                                                    if let Some(data_transfer) = event.data_transfer()
                                                                        && let Ok(payload) = serde_json::to_string(&drag_note)
                                                                    {
                                                                        let _ = data_transfer.set_data(BOARD_NOTE_DRAG_MIME, &payload);
                                                                        data_transfer.set_drop_effect("move");
                                                                    }
                                                                }
                                                                style=format!(
                                                                    "padding: 0.45rem 0.65rem; background: {}; color: {}; border: none; border-radius: 999px; cursor: grab; font-size: {};",
                                                                    theme.ui_button_primary, theme.ui_text_primary, NOTE_DRAG_LABEL_FONT_SIZE
                                                                )
                                                            >
                                                                {t!(i18n, notes.drag_to_board)}
                                                            </button>
                                                        }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                </div>
                                            </div>

                                            {if display_body.is_empty() {
                                                ().into_any()
                                            } else {
                                                view! {
                                                    <div
                                                        inner_html=rendered_html
                                                        style=format!(
                                                            "color: {}; font-size: {}; line-height: 1.5; word-break: break-word;",
                                                            theme.ui_text_primary, NOTE_BODY_FONT_SIZE
                                                        )
                                                    ></div>
                                                }.into_any()
                                            }}

                                            <div style=format!("display: flex; gap: 0.55rem; justify-content: flex-end; margin-top: 0.85rem; color: {}; font-size: {};", theme.ui_text_secondary, NOTE_META_FONT_SIZE)>
                                                {move || if note.board_position.is_some() {
                                                    view! {
                                                        <span>{t!(i18n, notes.on_board_badge)}</span>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}

                                                {if can_edit || can_delete {
                                                    view! {
                                                        {if can_edit {
                                                            view! {
                                                                <button
                                                                    on:click=move |_| vm.start_edit(&note_for_edit)
                                                                    style=format!(
                                                                        "padding: 0.45rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.45rem; cursor: pointer;",
                                                                        theme.ui_bg_secondary, theme.ui_text_primary
                                                                    )
                                                                >
                                                                    {t!(i18n, notes.edit_button)}
                                                                </button>
                                                            }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                        {if can_edit && note_for_unpin.board_position.is_some() {
                                                            view! {
                                                                <button
                                                                    on:click=move |_| unpin_note_action.with_value(|action| action(note_for_unpin.clone()))
                                                                    style=format!(
                                                                        "padding: 0.45rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.45rem; cursor: pointer;",
                                                                        theme.ui_bg_secondary, theme.ui_text_primary
                                                                    )
                                                                >
                                                                    {t!(i18n, notes.remove_from_board_button)}
                                                                </button>
                                                            }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                        {if can_delete {
                                                            view! {
                                                                <button
                                                                    on:click=move |_| delete_note_action.with_value(|action| action(note_for_delete.clone()))
                                                                    style=format!(
                                                                        "padding: 0.45rem 0.7rem; background: {}; color: {}; border: none; border-radius: 0.45rem; cursor: pointer;",
                                                                        theme.ui_button_danger, theme.ui_text_primary
                                                                    )
                                                                >
                                                                    {t!(i18n, notes.delete_button)}
                                                                </button>
                                                            }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                            </div>
                                        </article>
                                    }
                                }
                            />
                        }.into_any()
                    }}
                </div>
            </div>
        </DraggableWindow>
    }
}
