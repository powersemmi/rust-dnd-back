use crate::config;
use leptos::prelude::*;
use shared::events::{
    ChatMessagePayload, ClientEvent, MouseClickPayload, mouse::MouseEventTypeEnum,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen_futures::spawn_local;

use super::chat::ChatWindow;
use super::cursor::Cursor;
use super::side_menu::SideMenu;
use super::websocket::{CursorSignals, WsSender, connect_websocket};

#[component]
pub fn App() -> impl IntoView {
    // 1. Состояние приложения
    let (is_connected, set_is_connected) = signal(false);
    let (my_username, set_my_username) = signal(String::new());

    // Хранилище всех курсоров
    let (cursors, set_cursors) = signal(HashMap::<String, CursorSignals>::new());

    // Сообщения чата
    let messages = RwSignal::new(Vec::<ChatMessagePayload>::new());

    // WebSocket sender
    let (ws_sender, set_ws_sender) = signal::<Option<WsSender>>(None);

    // UI состояния
    let is_menu_open = RwSignal::new(false);
    let is_chat_open = RwSignal::new(false);

    // Конфигурация
    let cfg = StoredValue::new(config::Config::default());
    let theme = StoredValue::new(cfg.get_value().theme.clone());

    // 2. Функция подключения (вызывается по кнопке)
    let on_connect = move || {
        let username = my_username.get();
        if username.is_empty() {
            return;
        }

        set_is_connected.set(true);
        connect_websocket(
            username,
            set_ws_sender,
            set_cursors,
            messages,
            cfg.get_value(),
        );
    };

    // 3. Обработчик движения мыши (отправляем свои данные)
    let on_mouse_move = move |ev: leptos::web_sys::MouseEvent| {
        if !is_connected.get() {
            return;
        }

        let x = ev.client_x();
        let y = ev.client_y();
        let username = my_username.get();

        // Сразу обновляем СВОЙ курсор локально (чтобы не ждать пинга от сервера)
        set_cursors.update(|map| {
            if let Some((sig_x, sig_y)) = map.get(&username) {
                sig_x.set(x);
                sig_y.set(y);
            } else {
                map.insert(username.clone(), (RwSignal::new(x), RwSignal::new(y)));
            }
        });

        // Отправляем в канал (а оттуда оно уйдет в сокет)
        thread_local! {
            static IS_THROTTLED: AtomicBool = AtomicBool::new(false);
        }

        let can_send = IS_THROTTLED.with(|throttled| {
            if !throttled.load(Ordering::Relaxed) {
                throttled.store(true, Ordering::Relaxed);
                true
            } else {
                false
            }
        });

        if can_send {
            let event = ClientEvent::MouseClickPayload(MouseClickPayload {
                x,
                y,
                mouse_event_type: MouseEventTypeEnum::Move,
                user_id: username.clone(),
            });

            if let Some(sender) = ws_sender.get() {
                let _ = sender.unbounded_send(event);
            }

            let throttle_ms = cfg.get_value().theme.mouse_throttle_ms;
            spawn_local(async move {
                gloo_timers::future::sleep(std::time::Duration::from_millis(throttle_ms)).await;
                IS_THROTTLED.with(|throttled| {
                    throttled.store(false, Ordering::Relaxed);
                });
            });
        }
    };

    let bg_color = theme.get_value().background_color;
    view! {
        <div
            style=format!("width: 100vw; height: 100vh; background: {}; overflow: hidden;", bg_color)
            on:mousemove=on_mouse_move
        >
            <Show
                when=move || is_connected.get()
                fallback=move || view! {
                    // Экран логина
                    <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%;">
                        <h1 style="color: white">"Enter Name"</h1>
                        <input
                            type="text"
                            on:input=move |ev| set_my_username.set(event_target_value(&ev))
                            placeholder="Username"
                        />
                        <button on:click=move |_| on_connect()>"Join"</button>
                    </div>
                }
            >
                // Экран игры
                <h3 style="color: #aaa; position: absolute; top: 10px; right: 10px;">
                    "Connected as: " {move || my_username.get()}
                </h3>

                // Боковое меню
                <SideMenu
                    is_open=is_menu_open
                    on_chat_open=Callback::new(move |_| is_chat_open.set(true))
                />

                // Окно чата
                <ChatWindow
                    is_open=is_chat_open
                    messages=messages
                    ws_sender=ws_sender
                    username=my_username
                />

                // Рендерим все курсоры из мапы
                <For
                    each=move || {
                        cursors.get().into_iter().collect::<Vec<_>>()
                    }
                    key=|(name, _)| name.clone()
                    children=move |(name, (sig_x, sig_y))| {
                        let is_me = name == my_username.get();
                        let theme_copy = theme.get_value();
                        view! {
                            <Cursor username=name.clone() x=sig_x y=sig_y is_me=is_me theme=theme_copy />
                        }
                    }
                />
            </Show>
        </div>
    }
}
