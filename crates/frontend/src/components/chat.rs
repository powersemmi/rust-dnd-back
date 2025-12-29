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
            theme=theme
        >
            <div style="flex: 1; overflow-y: auto; padding: 15px; display: flex; flex-direction: column; gap: 10px;">
                <For
                    each=move || messages.get()
                    key=|msg| (msg.username.clone(), msg.payload.clone())
                    let:msg
                >
                    {
                        let my_name = username.get_untracked();
                        let is_mine = msg.username == my_name;
                        let bg_color = if is_mine { "#2563eb" } else { "#374151" };
                        let align = if is_mine { "flex-end" } else { "flex-start" };
                        view! {
                            <div style=format!(
                                "padding: 8px 12px; background: {}; border-radius: 8px; align-self: {}; max-width: 70%; word-wrap: break-word;",
                                bg_color, align
                            )>
                                <div style="font-size: 11px; color: #9ca3af; margin-bottom: 2px;">
                                    {msg.username}
                                </div>
                                <div style="color: white;">
                                    {msg.payload}
                                </div>
                            </div>
                        }
                    }
                </For>
            </div>

            <div style="padding: 15px; border-top: 1px solid #444; display: flex; gap: 10px;">
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
                    style="flex: 1; padding: 8px 12px; background: #2a2a2a; border: 1px solid #444; border-radius: 5px; color: white; outline: none;"
                />
                <button
                    on:click=move |_| send_message()
                    style="padding: 8px 16px; background: #2563eb; color: white; border: none; border-radius: 5px; cursor: pointer;"
                >
                    {move || t!(i18n, chat.send)}
                </button>
            </div>
        </DraggableWindow>
    }
}
