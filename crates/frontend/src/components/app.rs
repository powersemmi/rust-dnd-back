use crate::config;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use leptos::prelude::*;
use leptos::*;
use shared::events::{ClientEvent, MouseMovePayload};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen_futures::spawn_local;

use super::cursor::Cursor;

#[component]
pub fn App() -> impl IntoView {
    // 1. Состояние приложения
    let (is_connected, set_is_connected) = signal(false);
    let (my_username, set_my_username) = signal(String::new());

    // Хранилище всех курсоров: Map<Username, (signal_x, signal_y)>
    type CursorSignals = (RwSignal<i32>, RwSignal<i32>);
    let (cursors, set_cursors) = signal(HashMap::<String, CursorSignals>::new());

    // WebSocket sender (store type for sharing)
    type WsSender = futures::channel::mpsc::UnboundedSender<ClientEvent>;
    let (ws_sender, set_ws_sender) = signal::<Option<WsSender>>(None);

    // 2. Функция подключения (вызывается по кнопке)
    let on_connect = move || {
        let username = my_username.get();
        if username.is_empty() {
            return;
        }

        set_is_connected.set(true);

        spawn_local(async move {
            let ws_url = format!(
                "{}{}?room_id={}",
                config::ws_url(),
                config::WS_ROOM_PATH,
                config::DEFAULT_ROOM_ID
            );
            let ws = WebSocket::open(&ws_url).unwrap();
            let (mut write, mut read) = ws.split();

            // Создаем канал для отправки событий
            let (tx, mut rx) = futures::channel::mpsc::unbounded::<ClientEvent>();
            set_ws_sender.set(Some(tx));

            // Задача: Чтение из сокета -> Обновление UI
            spawn_local(async move {
                while let Some(msg) = read.next().await {
                    if let Ok(Message::Text(text)) = msg {
                        if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                            match event {
                                ClientEvent::MouseMovePayload(p) => {
                                    // Обновляем мапу курсоров
                                    set_cursors.update(|map| {
                                        if let Some((sig_x, sig_y)) = map.get(&p.user_id) {
                                            sig_x.set(p.x);
                                            sig_y.set(p.y);
                                        } else {
                                            map.insert(
                                                p.user_id.clone(),
                                                (RwSignal::new(p.x), RwSignal::new(p.y)),
                                            );
                                        }
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            // Задача: UI (канал) -> Сокет
            spawn_local(async move {
                while let Some(event) = rx.next().await {
                    let json = serde_json::to_string(&event).unwrap();
                    let _ = write.send(Message::Text(json)).await;
                }
            });
        });
    };

    // 3. Обработчик движения мыши (отправляем свои данные)
    let on_mouse_move = move |ev: web_sys::MouseEvent| {
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
            let event = ClientEvent::MouseMovePayload(MouseMovePayload {
                x,
                y,
                user_id: username.clone(),
            });

            if let Some(sender) = ws_sender.get() {
                let _ = sender.unbounded_send(event);
            }

            spawn_local(async move {
                gloo_timers::future::sleep(std::time::Duration::from_millis(
                    config::MOUSE_THROTTLE_MS,
                ))
                .await;
                IS_THROTTLED.with(|throttled| {
                    throttled.store(false, Ordering::Relaxed);
                });
            });
        }
    };

    view! {
        <div
            style=format!("width: 100vw; height: 100vh; background: {}; overflow: hidden;", config::BACKGROUND_COLOR)
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
                <h3 style="color: #aaa; position: absolute; top: 10px; left: 10px;">
                    "Connected as: " {move || my_username.get()}
                </h3>

                // Рендерим все курсоры из мапы
                <For
                    each=move || {
                        cursors.get().into_iter().collect::<Vec<_>>()
                    }
                    key=|(name, _)| name.clone()
                    children=move |(name, (sig_x, sig_y))| {
                        let is_me = name == my_username.get();
                        view! {
                            <Cursor username=name.clone() x=sig_x y=sig_y is_me=is_me />
                        }
                    }
                />
            </Show>
        </div>
    }
}
