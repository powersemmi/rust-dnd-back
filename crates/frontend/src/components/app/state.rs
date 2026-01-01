use super::super::chat::ChatWindow;
use super::super::conflict_resolver::ConflictResolver;
use super::super::cursor::Cursor;
use super::super::language_selector::LanguageSelector;
use super::super::login::LoginForm;
use super::super::register::RegisterForm;
use super::super::room_selector::RoomSelector;
use super::super::settings::Settings;
use super::super::side_menu::SideMenu;
use super::super::statistics::{StateEvent, StatisticsWindow};
use super::super::websocket::{CursorSignals, SyncConflict, WsSender};
use super::navigation::create_room_selected_callback;
use super::{
    AppState, create_login_success_callback, create_mouse_move_handler, create_navigation_callbacks,
};
use crate::components::voting::VotingWindow;
use crate::config;
use crate::i18n::i18n::{I18nContextProvider, Locale};
use crate::utils::{auth, token_refresh};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use shared::events::ChatMessagePayload;
use std::collections::HashMap;

#[component]
pub fn App() -> impl IntoView {
    // Загружаем сохранённую локаль из localStorage или используем дефолтную
    let initial_locale = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("locale").ok().flatten())
        .map(|locale_str| {
            if locale_str == "ru" {
                Locale::ru
            } else {
                Locale::en
            }
        })
        .unwrap_or(Locale::en);

    // Конфигурация
    let cfg = StoredValue::new(config::Config::default());
    let theme = StoredValue::new(cfg.get_value().theme.clone());
    let back_url = cfg.get_value().api.back_url;
    let api_path = cfg.get_value().api.api_path;

    // Проверяем наличие токена в localStorage при загрузке
    let initial_state = if auth::load_and_validate_token().is_some() {
        AppState::RoomSelection
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

    // События статистики
    let state_events = RwSignal::new(Vec::<StateEvent>::new());

    // Результаты голосований
    let voting_results =
        RwSignal::new(HashMap::<String, shared::events::voting::VotingResultPayload>::new());

    // Конфликты синхронизации
    let conflict_signal = RwSignal::new(Option::<SyncConflict>::None);

    // Голосования (множественные)
    let votings = RwSignal::new(HashMap::<String, super::super::voting::VotingState>::new());

    // Голосования, в которых пользователь проголосовал
    let voted_in = RwSignal::new(std::collections::HashSet::<String>::new());

    // Выбранные опции для каждого голосования
    let selected_options_map = RwSignal::new(std::collections::HashMap::<
        String,
        std::collections::HashSet<String>,
    >::new());

    // WebSocket sender
    let (ws_sender, set_ws_sender) = signal::<Option<WsSender>>(None);

    // UI состояния
    let is_menu_open = RwSignal::new(false);
    let is_chat_open = RwSignal::new(false);
    let is_settings_open = RwSignal::new(false);
    let is_statistics_open = RwSignal::new(false);
    let is_voting_open = RwSignal::new(false);
    let has_statistics_notification = RwSignal::new(false);
    let notification_count = RwSignal::new(0u32);
    let has_chat_notification = RwSignal::new(false);
    let chat_notification_count = RwSignal::new(0u32);

    // Отслеживание активного окна (последнее открытое)
    #[derive(Clone, Copy, PartialEq)]
    enum ActiveWindow {
        None,
        Chat,
        Settings,
        Statistics,
        Voting,
    }
    let active_window = RwSignal::new(ActiveWindow::None);

    // Загружаем токен и username из localStorage если есть
    if initial_state == AppState::RoomSelection {
        if let Some(token) = auth::load_and_validate_token() {
            set_jwt_token.set(token);
            // Запускаем автоматическое обновление токена
            token_refresh::start_token_refresh(back_url, api_path);
            if let Some(user) = auth::load_username() {
                set_username.set(user);
            }
        } else {
            set_app_state.set(AppState::Login);
        }
    }

    // Callbacks для навигации между экранами - сохраняем в StoredValue для использования в view!
    let on_login_success = StoredValue::new(create_login_success_callback(
        set_jwt_token,
        set_username,
        set_app_state,
        back_url,
        api_path,
    ));

    let (on_registered, on_switch_to_register, on_switch_to_login) =
        create_navigation_callbacks(set_app_state);
    let on_registered = StoredValue::new(on_registered);
    let on_switch_to_register = StoredValue::new(on_switch_to_register);
    let on_switch_to_login = StoredValue::new(on_switch_to_login);

    let on_room_selected = StoredValue::new(create_room_selected_callback(
        set_room_id,
        set_app_state,
        jwt_token,
        username,
        set_ws_sender,
        set_cursors,
        messages,
        state_events,
        conflict_signal,
        votings,
        voting_results,
        has_statistics_notification,
        notification_count,
        has_chat_notification,
        chat_notification_count,
        cfg,
    ));

    // Обработчик движения мыши
    let on_mouse_move = create_mouse_move_handler(app_state, username, set_cursors, ws_sender, cfg);

    // Обработчик клавиатуры для хоткеев (использует code для независимости от раскладки)
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if app_state.get() != AppState::Connected {
            return;
        }

        // Проверяем, что фокус не на input/textarea элементах
        if let Some(target) = ev.target() {
            if let Some(element) = target.dyn_ref::<web_sys::HtmlElement>() {
                let tag_name = element.tag_name().to_lowercase();
                if tag_name == "input" || tag_name == "textarea" {
                    return;
                }
            }
        }

        let code = ev.code();

        match code.as_str() {
            "Escape" => {
                // Закрываем активное окно
                match active_window.get() {
                    ActiveWindow::Chat => is_chat_open.set(false),
                    ActiveWindow::Settings => is_settings_open.set(false),
                    ActiveWindow::Voting => is_voting_open.set(false),
                    ActiveWindow::Statistics => is_statistics_open.set(false),
                    ActiveWindow::None => {}
                }
            }
            "KeyC" => {
                is_chat_open.set(true);
                active_window.set(ActiveWindow::Chat);
                has_chat_notification.set(false);
                chat_notification_count.set(0);
            }
            "KeyS" => {
                is_settings_open.set(true);
                active_window.set(ActiveWindow::Settings);
            }
            "KeyV" => {
                is_voting_open.set(true);
                active_window.set(ActiveWindow::Voting);
                has_statistics_notification.set(false);
                notification_count.set(0);
            }
            _ => {}
        }
    };

    let bg_color = theme.get_value().background_color;

    // Effect для автофокуса на главный div при подключении
    Effect::new(move || {
        if app_state.get() == AppState::Connected {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(element) = document.get_element_by_id("main-app-container") {
                        let _ = element
                            .dyn_ref::<web_sys::HtmlElement>()
                            .map(|el| el.focus());
                    }
                }
            }
        }
    });

    view! {
        <I18nContextProvider>
            <div
                id="main-app-container"
                tabindex="0"
                style=format!("width: 100vw; height: 100vh; background: {}; overflow: hidden; outline: none;", bg_color)
                on:mousemove=on_mouse_move
                on:keydown=on_keydown
            >
                <Show when=move || app_state.get() != AppState::Connected>
                    <LanguageSelector
                        initial_locale=initial_locale
                        theme=theme.get_value()
                    />
                </Show>

            {move || match app_state.get() {
                AppState::Login => view! {
                    <LoginForm
                        on_login_success=Callback::new(move |token| on_login_success.get_value()(token))
                        on_switch_to_register=Callback::new(move |_| on_switch_to_register.get_value()(()))
                        back_url=back_url
                        api_path=api_path
                        theme=theme.get_value()
                    />
                }.into_any(),
                AppState::Register => view! {
                    <RegisterForm
                        on_registered=Callback::new(move |_| on_registered.get_value()(()))
                        on_switch_to_login=Callback::new(move |_| on_switch_to_login.get_value()(()))
                        back_url=back_url
                        api_path=api_path
                        theme=theme.get_value()
                    />
                }.into_any(),
                AppState::RoomSelection => view! {
                    <RoomSelector
                        on_room_selected=Callback::new(move |room| on_room_selected.get_value()(room))
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
                            on_chat_open=Callback::new(move |_| {
                                is_chat_open.set(true);
                                active_window.set(ActiveWindow::Chat);
                                has_chat_notification.set(false);
                                chat_notification_count.set(0);
                            })
                            on_settings_open=Callback::new(move |_| {
                                is_settings_open.set(true);
                                active_window.set(ActiveWindow::Settings);
                            })
                            on_statistics_open=Callback::new(move |_| {
                                is_statistics_open.set(true);
                                active_window.set(ActiveWindow::Statistics);
                            })
                            on_voting_open=Callback::new(move |_| {
                                is_voting_open.set(true);
                                active_window.set(ActiveWindow::Voting);
                                has_statistics_notification.set(false);
                                notification_count.set(0);
                            })
                            has_statistics_notification=has_statistics_notification.read_only()
                            notification_count=notification_count.read_only()
                            has_chat_notification=has_chat_notification.read_only()
                            chat_notification_count=chat_notification_count.read_only()
                            theme=theme.get_value()
                        />

                        // Окно чата
                        <ChatWindow
                            is_open=is_chat_open
                            messages=messages
                            ws_sender=ws_sender
                            username=username
                            is_active=Signal::derive(move || active_window.get() == ActiveWindow::Chat)
                            on_focus=Callback::new(move |_| active_window.set(ActiveWindow::Chat))
                            theme=theme.get_value()
                        />

                        // Окно настроек
                        <Settings
                            is_open=is_settings_open
                            theme=theme.get_value()
                        />

                        // Окно статистики
                        <StatisticsWindow
                            is_open=is_statistics_open
                            events=state_events
                            voting_results=voting_results
                            is_active=Signal::derive(move || active_window.get() == ActiveWindow::Statistics)
                            on_focus=Callback::new(move |_| active_window.set(ActiveWindow::Statistics))
                            theme=theme.get_value()
                        />

                        // Окно разрешения конфликтов
                        <ConflictResolver
                            conflict=conflict_signal
                            username=username
                            on_create_voting=move |mut payload| {
                                payload.creator = username.get();
                                if let Some(mut sender) = ws_sender.get() {
                                    // Отправляем presence request чтобы собрать участников
                                    let request_id = format!("voting_{}", payload.voting_id);
                                    let presence_req = shared::events::PresenceRequestPayload {
                                        request_id,
                                        requester: username.get(),
                                    };
                                    let event = shared::events::ClientEvent::PresenceRequest(presence_req);
                                    if let Ok(json) = serde_json::to_string(&event) {
                                        let _ = sender.try_send(gloo_net::websocket::Message::Text(json.clone()));
                                    }

                                    // Отправляем событие начала голосования
                                    let event = shared::events::ClientEvent::VotingStart(payload);
                                    if let Ok(json) = serde_json::to_string(&event) {
                                        let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
                                    }
                                }
                            }
                            on_submit_vote={
                                let submit_vote_fn = move |voting_id: String, selected_option_ids: Vec<String>| {
                                    let voting_id_clone = voting_id.clone();
                                    let payload = shared::events::VotingCastPayload {
                                        voting_id,
                                        user: username.get(),
                                        selected_option_ids,
                                    };

                                    if let Some(mut sender) = ws_sender.get() {
                                        if let Ok(json) = serde_json::to_string(&shared::events::ClientEvent::VotingCast(payload)) {
                                            let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
                                        }
                                    }

                                    voted_in.update(|set| {
                                        set.insert(voting_id_clone);
                                    });
                                };
                                submit_vote_fn
                            }
                            on_change_room=move |new_room: String| {
                                on_room_selected.get_value()(new_room)
                            }
                            current_room=room_id
                            votings=votings
                            ws_sender=ws_sender
                            voted_in=voted_in
                            selected_options_map=selected_options_map
                            theme=theme.get_value()
                        />

                        // Окно голосований
                        <VotingWindow
                            show_voting_window=is_voting_open
                            votings=votings
                            voted_in=voted_in
                            username=username
                            ws_sender=ws_sender
                            is_active=Signal::derive(move || active_window.get() == ActiveWindow::Voting)
                            on_focus=Callback::new(move |_| active_window.set(ActiveWindow::Voting))
                            on_create_voting=move |mut payload| {
                                payload.creator = username.get();
                                if let Some(mut sender) = ws_sender.get() {
                                    // Отправляем presence request чтобы собрать участников
                                    let request_id = format!("voting_{}", payload.voting_id);
                                    let presence_req = shared::events::PresenceRequestPayload {
                                        request_id,
                                        requester: username.get(),
                                    };
                                    let event = shared::events::ClientEvent::PresenceRequest(presence_req);
                                    if let Ok(json) = serde_json::to_string(&event) {
                                        let _ = sender.try_send(gloo_net::websocket::Message::Text(json.clone()));
                                    }

                                    // Отправляем событие начала голосования
                                    let event = shared::events::ClientEvent::VotingStart(payload);
                                    if let Ok(json) = serde_json::to_string(&event) {
                                        let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
                                    }
                                }
                            }
                            theme=theme.get_value()
                        />

                        // Рендерим все курсоры из мапы
                        <For
                            each=move || {
                                cursors.get().into_iter().collect::<Vec<_>>()
                            }
                            key=|(name, _)| name.clone()
                            children=move |(name, cursor_sig)| {
                                let is_me = name == username.get();
                                let theme_copy = theme.get_value();
                                view! {
                                    <Cursor
                                        username=name.clone()
                                        x=cursor_sig.x
                                        y=cursor_sig.y
                                        is_me=is_me
                                        theme=theme_copy
                                    />
                                }
                            }
                        />
                    </div>
                }.into_any(),
            }}
            </div>
        </I18nContextProvider>
    }
}
