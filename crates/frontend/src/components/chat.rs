use leptos::prelude::*;
use leptos::web_sys::MouseEvent;
use shared::events::{ChatMessagePayload, ClientEvent};

use super::websocket::WsSender;

#[component]
pub fn ChatWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] messages: RwSignal<Vec<ChatMessagePayload>>,
    ws_sender: ReadSignal<Option<WsSender>>,
    username: ReadSignal<String>,
) -> impl IntoView {
    let (input_text, set_input_text) = signal(String::new());

    // Позиция и размер окна
    let (pos_x, set_pos_x) = signal(100);
    let (pos_y, set_pos_y) = signal(100);
    let (width, set_width) = signal(400);
    let (height, set_height) = signal(500);

    // Состояние перетаскивания
    let (is_dragging, set_is_dragging) = signal(false);
    let (drag_start_x, set_drag_start_x) = signal(0);
    let (drag_start_y, set_drag_start_y) = signal(0);
    let (window_start_x, set_window_start_x) = signal(0);
    let (window_start_y, set_window_start_y) = signal(0);

    // Состояние изменения размера
    let (is_resizing, set_is_resizing) = signal(false);
    let (resize_start_x, set_resize_start_x) = signal(0);
    let (resize_start_y, set_resize_start_y) = signal(0);
    let (size_start_w, set_size_start_w) = signal(0);
    let (size_start_h, set_size_start_h) = signal(0);

    let send_message = move || {
        let text = input_text.get();
        if text.is_empty() {
            return;
        }

        let msg = ChatMessagePayload {
            payload: text.clone(),
            user_id: username.get(),
        };

        // Отправляем сообщение через WebSocket
        if let Some(sender) = ws_sender.get() {
            let _ = sender.unbounded_send(ClientEvent::ChatMessage(msg));
        }

        set_input_text.set(String::new());
    };

    let on_header_mouse_down = move |ev: MouseEvent| {
        ev.prevent_default();
        set_is_dragging.set(true);
        set_drag_start_x.set(ev.client_x());
        set_drag_start_y.set(ev.client_y());
        set_window_start_x.set(pos_x.get());
        set_window_start_y.set(pos_y.get());
    };

    let on_resize_mouse_down = move |ev: MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        set_is_resizing.set(true);
        set_resize_start_x.set(ev.client_x());
        set_resize_start_y.set(ev.client_y());
        set_size_start_w.set(width.get());
        set_size_start_h.set(height.get());
    };

    // Глобальные обработчики для перетаскивания и изменения размера
    let on_global_mouse_move = move |ev: MouseEvent| {
        if is_dragging.get() {
            let dx = ev.client_x() - drag_start_x.get();
            let dy = ev.client_y() - drag_start_y.get();
            set_pos_x.set(window_start_x.get() + dx);
            set_pos_y.set(window_start_y.get() + dy);
        } else if is_resizing.get() {
            let dx = ev.client_x() - resize_start_x.get();
            let dy = ev.client_y() - resize_start_y.get();
            set_width.set((size_start_w.get() + dx).max(300));
            set_height.set((size_start_h.get() + dy).max(200));
        }
    };

    let on_global_mouse_up = move |_: MouseEvent| {
        set_is_dragging.set(false);
        set_is_resizing.set(false);
    };

    view! {
        <Show when=move || is_open.get()>
            <div
                on:mousemove=on_global_mouse_move
                on:mouseup=on_global_mouse_up
                style=move || format!(
                    "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; background: #1e1e1e; border: 1px solid #444; border-radius: 8px; box-shadow: 0 4px 20px rgba(0,0,0,0.5); z-index: 1001; display: flex; flex-direction: column; overflow: hidden;",
                    pos_x.get(),
                    pos_y.get(),
                    width.get(),
                    height.get()
                )
            >
                <div
                    on:mousedown=on_header_mouse_down
                    style="padding: 10px 15px; background: #2a2a2a; color: white; cursor: move; user-select: none; display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #444;"
                >
                    <span>"💬 Чат"</span>
                    <button
                        on:click=move |_| is_open.set(false)
                        style="background: none; border: none; color: white; cursor: pointer; font-size: 18px; padding: 0 5px;"
                    >
                        "×"
                    </button>
                </div>

                <div style="flex: 1; overflow-y: auto; padding: 15px; display: flex; flex-direction: column; gap: 10px;">
                    {move || {
                        messages.get().into_iter().enumerate().map(|(_, msg)| {
                            let is_mine = msg.user_id == username.get();
                            let bg_color = if is_mine { "#2563eb" } else { "#374151" };
                            let align = if is_mine { "flex-end" } else { "flex-start" };
                            view! {
                                <div style=format!(
                                    "padding: 8px 12px; background: {}; border-radius: 8px; align-self: {}; max-width: 70%; word-wrap: break-word;",
                                    bg_color, align
                                )>
                                    <div style="font-size: 11px; color: #9ca3af; margin-bottom: 2px;">
                                        {msg.user_id.clone()}
                                    </div>
                                    <div style="color: white;">
                                        {msg.payload.clone()}
                                    </div>
                                </div>
                            }
                        }).collect_view()
                    }}
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
                        placeholder="Введите сообщение..."
                        style="flex: 1; padding: 8px 12px; background: #2a2a2a; border: 1px solid #444; border-radius: 5px; color: white; outline: none;"
                    />
                    <button
                        on:click=move |_| send_message()
                        style="padding: 8px 16px; background: #2563eb; color: white; border: none; border-radius: 5px; cursor: pointer;"
                    >
                        "Отправить"
                    </button>
                </div>

                <div
                    on:mousedown=on_resize_mouse_down
                    style="position: absolute; bottom: 0; right: 0; width: 20px; height: 20px; cursor: nwse-resize; background: linear-gradient(135deg, transparent 50%, #666 50%);"
                />
            </div>
        </Show>
    }
}
