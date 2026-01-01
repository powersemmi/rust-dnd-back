use crate::components::draggable_window::DraggableWindow;
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, ClientEvent};

use super::websocket::WsSender;

#[component]
pub fn ChatWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] messages: RwSignal<Vec<ChatMessagePayload>>,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let (input_text, set_input_text) = signal(String::new());

    let send_message = move || {
        let text = input_text.get();
        if text.is_empty() {
            return;
        }

        let msg = ChatMessagePayload {
            payload: text.clone(),
            username: username.get_untracked(),
        };

        if let Some(mut sender) = ws_sender.get_untracked() {
            let event = ClientEvent::ChatMessage(msg);
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
            }
        }
        set_input_text.set(String::new());
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
            theme=theme.clone()
        >
            <div style="flex: 1; overflow-y: auto; padding: 0.9375rem; display: flex; flex-direction: column; gap: 0.625rem;">
                <For
                    each=move || messages.get()
                    key=|msg| (msg.username.clone(), msg.payload.clone())
                    let:msg
                >
                    {
                        let my_name = username.get_untracked();
                        let is_mine = msg.username == my_name;
                        let bg_color = if is_mine { theme.ui_button_primary } else { theme.ui_bg_secondary };
                        let align = if is_mine { "flex-end" } else { "flex-start" };
                        view! {
                            <div style=format!(
                                "padding: 0.5rem 0.75rem; background: {}; border-radius: 0.5rem; align-self: {}; max-width: 70%; word-wrap: break-word;",
                                bg_color, align
                            )>
                                <div style=format!("font-size: 0.6875rem; color: {}; margin-bottom: 0.125rem;", theme.ui_text_secondary)>
                                    {msg.username}
                                </div>
                                <div style=format!("color: {};", theme.ui_text_primary)>
                                    {msg.payload}
                                </div>
                            </div>
                        }
                    }
                </For>
            </div>

            <div style=format!("padding: 0.9375rem; border-top: 0.0625rem solid {}; display: flex; gap: 0.625rem;", theme.ui_border)>
                <input
                    type="text"
                    prop:value=move || input_text.get()
                    on:input=move |ev| set_input_text.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            send_message();
                        }
                    }
                    placeholder=move || t_string!(i18n, chat.placeholder)
                    style=format!("flex: 1; padding: 0.5rem 0.75rem; background: {}; border: 0.0625rem solid {}; border-radius: 0.3125rem; color: {}; outline: none;", theme.ui_bg_primary, theme.ui_border, theme.ui_text_primary)
                />
                <button
                    on:click=move |_| send_message()
                    style=format!("padding: 0.5rem 1rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer;", theme.ui_button_primary, theme.ui_text_primary)
                >
                    {move || t!(i18n, chat.send)}
                </button>
            </div>
        </DraggableWindow>
    }
}
