use super::view_model::ChatViewModel;
use crate::components::draggable_window::DraggableWindow;
use crate::components::websocket::{
    CHAT_FILE_INPUT_ACCEPT, FileTransferStage, FileTransferState, WsSender,
};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use shared::events::{ChatMessagePayload, DirectMessagePayload, FileRef};
use web_sys::{Event, HtmlInputElement};

const CHAT_BODY_FONT_SIZE: &str = "clamp(0.9rem, 0.87rem + 0.12vw, 0.98rem)";

/// Unified display entry for the sorted chat list.
/// Regular messages are converted to this when they arrive; DMs use their
/// own `sent_at_ms` as the sort key.
#[derive(Clone)]
struct ChatDisplayEntry {
    /// Sort key — milliseconds since Unix epoch.
    ts: f64,
    /// `true` for private (DM) entries.
    is_dm: bool,
    /// Sender username (always set).
    from: String,
    /// Recipient — only set for DM entries.
    to: String,
    /// Message text.
    body: String,
    /// File attachments — only regular messages can have these.
    attachments: Vec<FileRef>,
}
const CHAT_META_FONT_SIZE: &str = "clamp(0.72rem, 0.69rem + 0.12vw, 0.8rem)";
const CHAT_BUTTON_FONT_SIZE: &str = "clamp(0.84rem, 0.81rem + 0.12vw, 0.92rem)";

fn format_file_size(size: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];

    let mut value = size as f64;
    let mut unit_index = 0usize;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size, UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

fn attachment_kind_label(file: &FileRef) -> &'static str {
    if file.mime_type.starts_with("image/") {
        "Image"
    } else if file.mime_type == "application/pdf" {
        "PDF"
    } else if file.mime_type.contains("json") {
        "JSON"
    } else if file.mime_type.contains("xml") {
        "XML"
    } else if file.mime_type.contains("zip") {
        "ZIP"
    } else {
        "File"
    }
}

#[component]
fn DraftAttachmentChip(file: FileRef, vm: ChatViewModel, theme: Theme) -> impl IntoView {
    let hash = file.hash.clone();
    let file_name = file.file_name.clone();
    let file_size = format_file_size(file.size);

    view! {
        <div style=format!(
            "display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.45rem 0.65rem; \
             background: {}; border: 0.0625rem solid {}; border-radius: 999px; max-width: 100%;",
            theme.ui_bg_secondary, theme.ui_border
        )>
            <span style=format!(
                "color: {}; font-size: {}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                theme.ui_text_primary, CHAT_META_FONT_SIZE
            )>
                {format!("{} ({})", file_name, file_size)}
            </span>
            <button
                type="button"
                on:click=move |_| vm.remove_attachment(&hash)
                style=format!(
                    "padding: 0.15rem 0.45rem; background: {}; color: {}; border: none; border-radius: 999px; cursor: pointer; font-size: {};",
                    theme.ui_button_primary, theme.ui_text_primary, CHAT_META_FONT_SIZE
                )
            >
                "x"
            </button>
        </div>
    }
}

#[component]
fn ChatAttachmentCard(
    file: FileRef,
    file_transfer: FileTransferState,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let status_hash_progress = file.hash.clone();
    let status_hash_action = file.hash.clone();
    let preview_hash_image = file.hash.clone();
    let preview_hash_action = file.hash.clone();
    let kind_label = attachment_kind_label(&file).to_string();
    let is_image = file.mime_type.starts_with("image/");
    let is_pdf = file.mime_type == "application/pdf";
    let metadata = format!("{} - {}", kind_label, format_file_size(file.size));
    let file_name = file.file_name.clone();
    let preview_alt = file.file_name.clone();
    let download_name = file.file_name.clone();

    let request_file = Callback::new({
        let action_file = file.clone();
        let file_transfer = file_transfer.clone();
        move |()| {
            file_transfer.request_file(
                action_file.clone(),
                username.get_untracked(),
                ws_sender.get_untracked(),
            );
        }
    });

    view! {
        <div style=format!(
            "display: flex; flex-direction: column; gap: 0.55rem; padding: 0.7rem; background: {}; \
             border: 0.0625rem solid {}; border-radius: 0.6rem;",
            theme.ui_bg_primary, theme.ui_border
        )>
            <div style="display: flex; flex-direction: column; gap: 0.2rem;">
                <div style=format!(
                    "color: {}; font-size: {}; font-weight: 600; word-break: break-word;",
                    theme.ui_text_primary, CHAT_BODY_FONT_SIZE
                )>
                    {file_name}
                </div>
                <div style=format!(
                    "color: {}; font-size: {};",
                    theme.ui_text_secondary, CHAT_META_FONT_SIZE
                )>
                    {metadata}
                </div>
            </div>

            {move || {
                let status = file_transfer
                    .transfer_statuses
                    .get()
                    .get(&status_hash_progress)
                    .cloned();
                match status {
                    Some(status) if status.stage != FileTransferStage::Complete => {
                        let label = match status.stage {
                            FileTransferStage::Requested => t_string!(i18n, chat.loading).to_string(),
                            FileTransferStage::Receiving => {
                                format!(
                                    "{} {}%",
                                    t_string!(i18n, chat.loading),
                                    status.progress_percent()
                                )
                            }
                            FileTransferStage::Failed => status
                                .detail
                                .clone()
                                .unwrap_or_else(|| t_string!(i18n, chat.transfer_failed).to_string()),
                            FileTransferStage::Complete => String::new(),
                        };
                        let progress = if matches!(status.stage, FileTransferStage::Failed) {
                            100
                        } else {
                            status.progress_percent()
                        };
                        let bar_color = if matches!(status.stage, FileTransferStage::Failed) {
                            theme.ui_button_danger
                        } else {
                            theme.ui_button_primary
                        };
                        view! {
                            <div style="display: flex; flex-direction: column; gap: 0.35rem;">
                                <div style=format!(
                                    "color: {}; font-size: {};",
                                    theme.ui_text_secondary, CHAT_META_FONT_SIZE
                                )>
                                    {label}
                                </div>
                                <div style=format!(
                                    "width: 100%; height: 0.35rem; background: {}; border-radius: 999px; overflow: hidden;",
                                    theme.ui_bg_secondary
                                )>
                                    <div style=format!(
                                        "width: {}%; height: 100%; background: {}; transition: width 120ms ease;",
                                        progress, bar_color
                                    )></div>
                                </div>
                            </div>
                        }
                            .into_any()
                    }
                    _ => ().into_any(),
                }
            }}

            {move || {
                let preview_url = file_transfer
                    .file_urls
                    .get()
                    .get(&preview_hash_image)
                    .cloned();
                match (is_image, preview_url) {
                    (true, Some(url)) => {
                        let image_url = url.clone();
                        view! {
                        <a href=url target="_blank" rel="noopener noreferrer">
                            <img
                                src=image_url
                                alt=preview_alt.clone()
                                style=format!(
                                    "display: block; width: 100%; max-height: 14rem; object-fit: contain; border: 0.0625rem solid {}; \
                                     border-radius: 0.5rem; background: {};",
                                    theme.ui_border, theme.ui_bg_secondary
                                )
                            />
                        </a>
                    }
                        .into_any()
                    }
                    _ => ().into_any(),
                }
            }}

            {move || {
                let preview_url = file_transfer
                    .file_urls
                    .get()
                    .get(&preview_hash_action)
                    .cloned();
                let status = file_transfer
                    .transfer_statuses
                    .get()
                    .get(&status_hash_action)
                    .cloned();
                let is_loading = status.as_ref().is_some_and(|state| {
                    matches!(
                        state.stage,
                        FileTransferStage::Requested | FileTransferStage::Receiving
                    )
                });
                let is_failed = status
                    .as_ref()
                    .is_some_and(|state| matches!(state.stage, FileTransferStage::Failed));

                if is_image {
                    if preview_url.is_some() {
                        return ().into_any();
                    }

                    let label = if is_loading {
                        t_string!(i18n, chat.loading).to_string()
                    } else if is_failed {
                        t_string!(i18n, chat.retry).to_string()
                    } else {
                        t_string!(i18n, chat.load_preview).to_string()
                    };
                    let request_preview = request_file;

                    return view! {
                        <button
                            type="button"
                            on:click=move |_| request_preview.run(())
                            disabled=is_loading
                            style=format!(
                                "align-self: flex-start; padding: 0.45rem 0.8rem; background: {}; color: {}; border: none; \
                                 border-radius: 0.4rem; cursor: pointer; opacity: {}; font-size: {};",
                                theme.ui_button_primary,
                                theme.ui_text_primary,
                                if is_loading { 0.65 } else { 1.0 },
                                CHAT_BUTTON_FONT_SIZE
                            )
                        >
                            {label}
                        </button>
                    }
                        .into_any();
                }

                if let Some(url) = preview_url {
                    if is_pdf {
                        return view! {
                            <a
                                href=url
                                target="_blank"
                                rel="noopener noreferrer"
                                style=format!(
                                "align-self: flex-start; padding: 0.45rem 0.8rem; background: {}; color: {}; \
                                 border-radius: 0.4rem; text-decoration: none; font-size: {};",
                                    theme.ui_button_primary, theme.ui_text_primary, CHAT_BUTTON_FONT_SIZE
                                )
                            >
                                {t!(i18n, chat.open)}
                            </a>
                        }
                            .into_any();
                    }

                    return view! {
                        <a
                            href=url
                            download=download_name.clone()
                            style=format!(
                                "align-self: flex-start; padding: 0.45rem 0.8rem; background: {}; color: {}; \
                                 border-radius: 0.4rem; text-decoration: none; font-size: {};",
                                theme.ui_button_primary, theme.ui_text_primary, CHAT_BUTTON_FONT_SIZE
                            )
                        >
                            {t!(i18n, chat.download)}
                        </a>
                    }
                        .into_any();
                }

                let label = if is_loading {
                    t_string!(i18n, chat.loading).to_string()
                } else if is_failed {
                    t_string!(i18n, chat.retry).to_string()
                } else {
                    t_string!(i18n, chat.download).to_string()
                };
                let request_download = request_file;

                view! {
                    <button
                        type="button"
                        on:click=move |_| request_download.run(())
                        disabled=is_loading
                        style=format!(
                            "align-self: flex-start; padding: 0.45rem 0.8rem; background: {}; color: {}; border: none; \
                             border-radius: 0.4rem; cursor: pointer; opacity: {}; font-size: {};",
                            theme.ui_button_primary,
                            theme.ui_text_primary,
                            if is_loading { 0.65 } else { 1.0 },
                            CHAT_BUTTON_FONT_SIZE
                        )
                    >
                        {label}
                    </button>
                }
                    .into_any()
            }}
        </div>
    }
}

#[component]
pub fn ChatWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] messages: RwSignal<Vec<ChatMessagePayload>>,
    #[prop(into)] direct_messages: RwSignal<Vec<shared::events::DirectMessagePayload>>,
    file_transfer: FileTransferState,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let vm = ChatViewModel::new();

    let input_ref = NodeRef::<html::Input>::new();
    let attachment_input_ref = NodeRef::<html::Input>::new();
    let messages_container_ref = NodeRef::<html::Div>::new();

    // Sorted unified list of all display entries (regular + DM), keyed by ts.
    let chat_entries: RwSignal<Vec<ChatDisplayEntry>> = RwSignal::new(Vec::new());
    // Track how many regular messages we have already added to chat_entries so
    // we can detect incremental additions vs full snapshot replacements.
    let prev_msg_len: RwSignal<usize> = RwSignal::new(0);

    // Effect: merge regular messages into chat_entries with local timestamps.
    {
        Effect::new(move |_| {
            let msgs = messages.get();
            let prev = prev_msg_len.get_untracked();
            let now = js_sys::Date::now();

            chat_entries.update(|entries| {
                if msgs.len() > prev {
                    // Incremental append: assign current time to new messages.
                    for msg in &msgs[prev..] {
                        entries.push(ChatDisplayEntry {
                            ts: now,
                            is_dm: false,
                            from: msg.username.clone(),
                            to: String::new(),
                            body: msg.payload.clone(),
                            attachments: msg.attachments.clone(),
                        });
                    }
                } else {
                    // Snapshot replacement: rebuild all regular entries with
                    // virtual timestamps spaced 100 ms apart in the past.
                    entries.retain(|e| e.is_dm);
                    let base = now - (msgs.len() as f64) * 100.0;
                    for (idx, msg) in msgs.iter().enumerate() {
                        entries.push(ChatDisplayEntry {
                            ts: base + (idx as f64) * 100.0,
                            is_dm: false,
                            from: msg.username.clone(),
                            to: String::new(),
                            body: msg.payload.clone(),
                            attachments: msg.attachments.clone(),
                        });
                    }
                    entries.sort_by(|a, b| {
                        a.ts.partial_cmp(&b.ts).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            });
            prev_msg_len.set(msgs.len());
        });
    }

    // Effect: append new DMs to chat_entries using their own sent_at_ms.
    {
        let prev_dm_len: RwSignal<usize> = RwSignal::new(0);
        Effect::new(move |_| {
            let dms = direct_messages.get();
            let prev = prev_dm_len.get_untracked();
            if dms.len() > prev {
                chat_entries.update(|entries| {
                    for dm in &dms[prev..] {
                        entries.push(ChatDisplayEntry {
                            ts: dm.sent_at_ms,
                            is_dm: true,
                            from: dm.from.clone(),
                            to: dm.to.clone(),
                            body: dm.body.clone(),
                            attachments: vec![],
                        });
                        entries.sort_by(|a, b| {
                            a.ts.partial_cmp(&b.ts).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                });
            }
            prev_dm_len.set(dms.len());
        });
    }

    {
        let file_transfer = file_transfer.clone();
        Effect::new(move || {
            let current_messages = messages.get();
            file_transfer.reconcile_chat_attachments(
                &current_messages,
                username.get_untracked(),
                ws_sender.get_untracked(),
            );
        });
    }

    let scroll_to_bottom = move || {
        if let Some(container) = messages_container_ref.get() {
            container.set_scroll_top(container.scroll_height());
        }
    };

    let is_scrolled_to_bottom = move || -> bool {
        if let Some(container) = messages_container_ref.get() {
            let scroll_top = container.scroll_top();
            let scroll_height = container.scroll_height();
            let client_height = container.client_height();
            (scroll_height - scroll_top - client_height) < 100
        } else {
            true
        }
    };

    Effect::new(move || {
        let _msgs = messages.get();
        set_timeout(
            move || {
                if is_scrolled_to_bottom() {
                    scroll_to_bottom();
                }
            },
            std::time::Duration::from_millis(10),
        );
    });

    let on_attachments_selected = {
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

            for index in 0..files.length() {
                let Some(file) = files.get(index) else {
                    continue;
                };
                let file_transfer = file_transfer.clone();
                let username = username.get_untracked();
                let sender = ws_sender.get_untracked();
                spawn_local(async move {
                    match file_transfer
                        .import_browser_file_with_announce(file, username, sender, false)
                        .await
                    {
                        Ok(file_ref) => vm.add_attachment(file_ref),
                        Err(error) => vm.error_message.set(Some(error)),
                    }
                });
            }

            input.set_value("");
        }
    };

    let do_send = {
        let file_transfer = file_transfer.clone();
        Callback::new(move |()| {
            let attachments = vm.pending_attachments.get_untracked();
            let current_username = username.get_untracked();
            // Capture draft text BEFORE send_message clears it so we can show
            // a local-only DM echo in the chat if the message was a DM.
            let draft_text = vm.input_text.get_untracked();
            let sent = vm.send_message(&current_username, ws_sender);
            if sent {
                // If the sent message was a DM, insert a local display entry so
                // the sender gets immediate visual feedback.  DMs are not stored
                // in room state and will vanish on the next snapshot; that is an
                // acceptable trade-off for a session-scoped feature.
                let trimmed_draft = draft_text.trim().to_string();
                if let Some((recipient, body)) =
                    ChatViewModel::parse_direct_message(&trimmed_draft)
                {
                    // Echo the sent DM into the direct_messages signal so it
                    // appears in the DM section and survives snapshot overwrites.
                    let now = js_sys::Date::now();
                    direct_messages.update(|msgs| {
                        msgs.push(DirectMessagePayload {
                            from: current_username.clone(),
                            to: recipient.to_string(),
                            body: body.to_string(),
                            sent_at_ms: now,
                        });
                    });
                }

                file_transfer.announce_local_files(
                    &attachments,
                    current_username,
                    ws_sender.get_untracked(),
                );
                if let Some(input_el) = input_ref.get() {
                    let _ = input_el.focus();
                }
                set_timeout(scroll_to_bottom, std::time::Duration::from_millis(10));
            }
        })
    };

    let messages_theme = theme.clone();
    let dm_theme = theme.clone();
    let error_theme = theme.clone();
    let draft_theme = theme.clone();
    let do_send_on_enter = do_send;
    let do_send_on_click = do_send;

    view! {
        <DraggableWindow
            is_open=is_open
            title=move || t_string!(i18n, chat.title)
            initial_x=100
            initial_y=100
            initial_width=460
            initial_height=560
            min_width=340
            min_height=240
            is_active=is_active
            on_focus=on_focus.unwrap_or_else(|| Callback::new(|_| {}))
            theme=theme.clone()
        >
            <div
                node_ref=messages_container_ref
                style="flex: 1; overflow-y: auto; padding: 0.9375rem; display: flex; flex-direction: column; gap: 0.625rem;"
            >
                // Unified sorted message list (regular + DM).
                {move || {
                    let my_name = username.get_untracked();
                    chat_entries.get().into_iter().map(|entry| {
                        let is_mine = entry.from == my_name;
                        let align = if is_mine { "flex-end" } else { "flex-start" };

                        if entry.is_dm {
                            let header = format!("{} → @{}", entry.from, entry.to);
                            let body = entry.body.clone();
                            view! {
                                <div style=format!(
                                    "padding: 0.5rem 0.75rem; background: #2d1b4e; border-radius: 0.5rem; \
                                     align-self: {}; max-width: 80%; word-wrap: break-word; display: flex; \
                                     flex-direction: column; gap: 0.55rem; \
                                     border: 1px solid rgba(139,92,246,0.35);",
                                    align
                                )>
                                    <div style=format!(
                                        "font-size: {}; color: #c4b5fd; margin-bottom: 0.125rem;",
                                        CHAT_META_FONT_SIZE
                                    )>{header}</div>
                                    <div style=format!(
                                        "color: {}; white-space: pre-wrap; font-size: {}; line-height: 1.45;",
                                        dm_theme.ui_text_primary, CHAT_BODY_FONT_SIZE
                                    )>{body}</div>
                                </div>
                            }.into_any()
                        } else {
                            let bg_color = if is_mine {
                                messages_theme.ui_button_primary
                            } else {
                                messages_theme.ui_bg_secondary
                            };
                            let has_text = !entry.body.is_empty();
                            let attachments = entry.attachments.clone();
                            view! {
                                <div style=format!(
                                    "padding: 0.5rem 0.75rem; background: {}; border-radius: 0.5rem; \
                                     align-self: {}; max-width: 80%; word-wrap: break-word; display: flex; flex-direction: column; gap: 0.55rem;",
                                    bg_color, align
                                )>
                                    <div style=format!(
                                        "font-size: {}; color: {}; margin-bottom: 0.125rem;",
                                        CHAT_META_FONT_SIZE, messages_theme.ui_text_secondary
                                    )>{entry.from.clone()}</div>
                                    {has_text.then(|| view! {
                                        <div style=format!(
                                            "color: {}; white-space: pre-wrap; font-size: {}; line-height: 1.45;",
                                            messages_theme.ui_text_primary, CHAT_BODY_FONT_SIZE
                                        )>{entry.body.clone()}</div>
                                    })}
                                    {(!attachments.is_empty()).then(|| view! {
                                        <div style="display: flex; flex-direction: column; gap: 0.55rem;">
                                            {attachments.into_iter().map(|file| view! {
                                                <ChatAttachmentCard
                                                    file=file
                                                    file_transfer=file_transfer.clone()
                                                    ws_sender=ws_sender
                                                    username=username
                                                    theme=messages_theme.clone()
                                                />
                                            }).collect_view()}
                                        </div>
                                    })}
                                </div>
                            }.into_any()
                        }
                    }).collect_view()
                }}
            </div>

            <div style=format!(
                "padding: 0.9375rem; border-top: 0.0625rem solid {}; display: flex; flex-direction: column; gap: 0.65rem;",
                theme.ui_border
            )>
                {move || {
                    vm.error_message
                        .get()
                        .map(|error| {
                            view! {
                                <div style=format!(
                                    "color: {}; background: {}; padding: 0.55rem 0.7rem; border-radius: 0.45rem; font-size: {}; line-height: 1.45;",
                                    error_theme.ui_text_primary, error_theme.ui_button_danger, CHAT_BODY_FONT_SIZE
                                )>
                                    {error}
                                </div>
                            }
                        })
                }}

                {move || {
                    let attachments = vm.pending_attachments.get();
                    (!attachments.is_empty())
                        .then(|| {
                            view! {
                                <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                                    {attachments
                                        .into_iter()
                                        .map(|file| {
                                            view! {
                                                <DraftAttachmentChip file=file vm=vm theme=draft_theme.clone() />
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                        })
                }}

                <div style="display: flex; gap: 0.625rem; align-items: stretch;">
                    <input
                        node_ref=attachment_input_ref
                        type="file"
                        accept=CHAT_FILE_INPUT_ACCEPT
                        multiple
                        on:change=on_attachments_selected
                        style="display: none;"
                    />
                    <button
                        type="button"
                        on:click=move |_| {
                            if let Some(input) = attachment_input_ref.get() {
                                input.click();
                            }
                        }
                        style=format!(
                            "padding: 0.5rem 0.85rem; background: {}; color: {}; border: none; \
                             border-radius: 0.3125rem; cursor: pointer; font-size: {};",
                            theme.ui_bg_secondary, theme.ui_text_primary, CHAT_BUTTON_FONT_SIZE
                        )
                    >
                        {move || t!(i18n, chat.attach)}
                    </button>
                    <input
                        node_ref=input_ref
                        type="text"
                        prop:value=move || vm.input_text.get()
                        on:input=move |ev| vm.input_text.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                do_send_on_enter.run(());
                            }
                        }
                        placeholder=move || t_string!(i18n, chat.placeholder)
                        style=format!(
                            "flex: 1; padding: 0.5rem 0.75rem; background: {}; border: 0.0625rem solid {}; \
                             border-radius: 0.3125rem; color: {}; outline: none; font-size: {};",
                            theme.ui_bg_primary, theme.ui_border, theme.ui_text_primary, CHAT_BODY_FONT_SIZE
                        )
                    />
                    <button
                        on:click=move |_| do_send_on_click.run(())
                        style=format!(
                            "padding: 0.5rem 1rem; background: {}; color: {}; border: none; \
                             border-radius: 0.3125rem; cursor: pointer; font-size: {};",
                            theme.ui_button_primary, theme.ui_text_primary, CHAT_BUTTON_FONT_SIZE
                        )
                    >
                        {move || t!(i18n, chat.send)}
                    </button>
                </div>
            </div>
        </DraggableWindow>
    }
}
