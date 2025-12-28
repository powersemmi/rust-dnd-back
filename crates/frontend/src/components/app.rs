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
use super::login::LoginForm;
use super::register::RegisterForm;
use super::room_selector::RoomSelector;
use super::side_menu::SideMenu;
use super::websocket::{CursorSignals, WsSender, connect_websocket};

#[derive(Clone, Copy, PartialEq)]
enum AppState {
    Login,
    Register,
    RoomSelection,
    Connected,
}

#[component]
pub fn App() -> impl IntoView {
    // Конфигурация
    let cfg = StoredValue::new(config::Config::default());
    let theme = StoredValue::new(cfg.get_value().theme.clone());
    let back_url = cfg.get_value().api.back_url;
    let api_path = cfg.get_value().api.api_path;

    // Проверяем наличие токена в localStorage при загрузке
    let initial_state = if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if storage.get_item("jwt_token").ok().flatten().is_some() {
                AppState::RoomSelection
            } else {
                AppState::Login
            }
        } else {
            AppState::Login
        }
    } else {
        AppState::Login
    };

    // Состояние приложения
    let (app_state, set_app_state) = signal(initial_state);
    let (jwt_token, set_jwt_token) = signal(String::new());
    let (username, set_username) = signal(String::new());
    let (room_id, set_room_id) = signal(String::new());

    // Хранилище всех курсоров
    let (cursors, set_cursors) = signal(HashMap::<String, CursorSignals>::new());

    // Сообщения чата
    let messages = RwSignal::new(Vec::<ChatMessagePayload>::new());

    // WebSocket sender
    let (ws_sender, set_ws_sender) = signal::<Option<WsSender>>(None);

    // UI состояния
    let is_menu_open = RwSignal::new(false);
    let is_chat_open = RwSignal::new(false);

    // Загружаем токен и username из localStorage если есть
    if initial_state == AppState::RoomSelection {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(token)) = storage.get_item("jwt_token") {
                    set_jwt_token.set(token);
                }
                if let Ok(Some(user)) = storage.get_item("username") {
                    set_username.set(user);
                }
            }
        }
    }

    // Callbacks для навигации между экранами
    let on_login_success = move |token: String| {
        set_jwt_token.set(token);
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(user)) = storage.get_item("username") {
                    set_username.set(user);
                }
            }
        }
        set_app_state.set(AppState::RoomSelection);
    };

    let on_registered = move |_| {
        set_app_state.set(AppState::Login);
    };

    let on_switch_to_register = move |_| {
        set_app_state.set(AppState::Register);
    };

    let on_switch_to_login = move |_| {
        set_app_state.set(AppState::Login);
    };

    let on_room_selected = move |selected_room_id: String| {
        set_room_id.set(selected_room_id.clone());
        set_app_state.set(AppState::Connected);

        // Подключаемся к WebSocket
        connect_websocket(
            selected_room_id,
            jwt_token.get(),
            set_ws_sender,
            set_cursors,
            messages,
            cfg.get_value(),
        );
    };

    // Обработчик движения мыши (отправляем свои данные)
    let on_mouse_move = move |ev: leptos::web_sys::MouseEvent| {
        if app_state.get() != AppState::Connected {
            return;
        }

        let x = ev.client_x();
        let y = ev.client_y();
        let user = username.get();

        // Сразу обновляем СВОЙ курсор локально (чтобы не ждать пинга от сервера)
        set_cursors.update(|map| {
            if let Some((sig_x, sig_y)) = map.get(&user) {
                sig_x.set(x);
                sig_y.set(y);
            } else {
                map.insert(user.clone(), (RwSignal::new(x), RwSignal::new(y)));
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
                user_id: user.clone(),
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
            {move || match app_state.get() {
                AppState::Login => view! {
                    <LoginForm
                        on_login_success=Callback::new(on_login_success)
                        on_switch_to_register=Callback::new(on_switch_to_register)
                        back_url=back_url
                        api_path=api_path
                        theme=theme.get_value()
                    />
                }.into_any(),
                AppState::Register => view! {
                    <RegisterForm
                        on_registered=Callback::new(on_registered)
                        on_switch_to_login=Callback::new(on_switch_to_login)
                        back_url=back_url
                        api_path=api_path
                        theme=theme.get_value()
                    />
                }.into_any(),
                AppState::RoomSelection => view! {
                    <RoomSelector
                        on_room_selected=Callback::new(on_room_selected)
                        theme=theme.get_value()
                    />
                }.into_any(),
                AppState::Connected => view! {
                    <div style="width: 100%; height: 100%; position: relative;">
                        // Информация о пользователе и комнате
                        <h3 style="color: #aaa; position: absolute; top: 10px; right: 10px; z-index: 100;">
                            "Connected as: " {move || username.get()} " | Room: " {move || room_id.get()}
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
                            username=username
                        />

                        // Рендерим все курсоры из мапы
                        <For
                            each=move || {
                                cursors.get().into_iter().collect::<Vec<_>>()
                            }
                            key=|(name, _)| name.clone()
                            children=move |(name, (sig_x, sig_y))| {
                                let is_me = name == username.get();
                                let theme_copy = theme.get_value();
                                view! {
                                    <Cursor username=name.clone() x=sig_x y=sig_y is_me=is_me theme=theme_copy />
                                }
                            }
                        />
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
