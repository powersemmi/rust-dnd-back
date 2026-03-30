use super::view_model::ChatViewModel;
use crate::components::draggable_window::DraggableWindow;
use crate::components::websocket::WsSender;
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use shared::events::ChatMessagePayload;

#[component]
pub fn ChatWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] messages: RwSignal<Vec<ChatMessagePayload>>,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let vm = ChatViewModel::new();

    let input_ref = NodeRef::<leptos::html::Input>::new();
    let messages_container_ref = NodeRef::<leptos::html::Div>::new();

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

    // Auto-scroll on new messages
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

    let do_send = move || {
        let sent = vm.send_message(&username.get_untracked(), ws_sender);
        if sent {
            if let Some(input_el) = input_ref.get() {
                let _ = input_el.focus();
            }
            set_timeout(
                move || scroll_to_bottom(),
                std::time::Duration::from_millis(10),
            );
        }
    };

    view! {
        <DraggableWindow
            is_open=is_open
            title=move || t_string!(i18n, chat.title)
            initial_x=100
            initial_y=100
            initial_width=400
            initial_height=500
            min_width=300
            min_height=200
            is_active=is_active
            on_focus=on_focus.unwrap_or_else(|| Callback::new(|_| {}))
            theme=theme.clone()
        >
            // Message list
            <div
                node_ref=messages_container_ref
                style="flex: 1; overflow-y: auto; padding: 0.9375rem; display: flex; flex-direction: column; gap: 0.625rem;"
            >
                {move || {
                    messages.get().into_iter().map(|msg| {
                        let my_name = username.get_untracked();
                        let is_mine = msg.username == my_name;
                        let bg_color = if is_mine { theme.ui_button_primary } else { theme.ui_bg_secondary };
                        let align = if is_mine { "flex-end" } else { "flex-start" };
                        view! {
                            <div style=format!(
                                "padding: 0.5rem 0.75rem; background: {}; border-radius: 0.5rem; \
                                 align-self: {}; max-width: 70%; word-wrap: break-word;",
                                bg_color, align
                            )>
                                <div style=format!("font-size: 0.6875rem; color: {}; margin-bottom: 0.125rem;", theme.ui_text_secondary)>
                                    {msg.username.clone()}
                                </div>
                                <div style=format!("color: {};", theme.ui_text_primary)>
                                    {msg.payload.clone()}
                                </div>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>

            // Input area
            <div style=format!(
                "padding: 0.9375rem; border-top: 0.0625rem solid {}; display: flex; gap: 0.625rem;",
                theme.ui_border
            )>
                <input
                    node_ref=input_ref
                    type="text"
                    prop:value=move || vm.input_text.get()
                    on:input=move |ev| vm.input_text.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            do_send();
                        }
                    }
                    placeholder=move || t_string!(i18n, chat.placeholder)
                    style=format!(
                        "flex: 1; padding: 0.5rem 0.75rem; background: {}; border: 0.0625rem solid {}; \
                         border-radius: 0.3125rem; color: {}; outline: none;",
                        theme.ui_bg_primary, theme.ui_border, theme.ui_text_primary
                    )
                />
                <button
                    on:click=move |_| do_send()
                    style=format!(
                        "padding: 0.5rem 1rem; background: {}; color: {}; border: none; \
                         border-radius: 0.3125rem; cursor: pointer;",
                        theme.ui_button_primary, theme.ui_text_primary
                    )
                >
                    {move || t!(i18n, chat.send)}
                </button>
            </div>
        </DraggableWindow>
    }
}
