use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{
    ConflictResolutionHandle, CursorSignals, SyncConflict, WsSender, connect_websocket,
};
use crate::config;
use crate::utils::{auth, token_refresh};
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, voting::VotingResultPayload};
use std::collections::HashMap;

use super::AppState;

/// Создает callback для успешного логина
pub fn create_login_success_callback(
    set_jwt_token: WriteSignal<String>,
    set_username: WriteSignal<String>,
    set_app_state: WriteSignal<AppState>,
    back_url: &'static str,
    api_path: &'static str,
) -> impl Fn(String) + Clone {
    move |token: String| {
        set_jwt_token.set(token);
        // Запускаем автоматическое обновление токена после успешного входа
        token_refresh::start_token_refresh(back_url, api_path);
        if let Some(user) = auth::load_username() {
            set_username.set(user);
        }
        set_app_state.set(AppState::RoomSelection);
    }
}

/// Создает все навигационные callbacks
pub fn create_navigation_callbacks(
    set_app_state: WriteSignal<AppState>,
) -> (
    impl Fn(()) + Clone,
    impl Fn(()) + Clone,
    impl Fn(()) + Clone,
) {
    let on_registered = move |_| {
        set_app_state.set(AppState::Login);
    };

    let on_switch_to_register = move |_| {
        set_app_state.set(AppState::Register);
    };

    let on_switch_to_login = move |_| {
        set_app_state.set(AppState::Login);
    };

    (on_registered, on_switch_to_register, on_switch_to_login)
}

/// Создает callback для выбора комнаты и подключения к WebSocket
pub fn create_room_selected_callback(
    set_room_id: WriteSignal<String>,
    set_app_state: WriteSignal<AppState>,
    jwt_token: ReadSignal<String>,
    username: ReadSignal<String>,
    set_ws_sender: WriteSignal<Option<WsSender>>,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    messages: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
    votings: RwSignal<HashMap<String, VotingState>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    has_statistics_notification: RwSignal<bool>,
    notification_count: RwSignal<u32>,
    has_chat_notification: RwSignal<bool>,
    chat_notification_count: RwSignal<u32>,
    cfg: StoredValue<config::Config>,
    conflict_resolution_handle: ConflictResolutionHandle,
) -> impl Fn(String) + Clone {
    let handle_clone = conflict_resolution_handle.clone();
    move |selected_room_id: String| {
        set_room_id.set(selected_room_id.clone());
        set_app_state.set(AppState::Connected);

        // Подключаемся к WebSocket
        connect_websocket(
            selected_room_id,
            jwt_token.get(),
            username.get_untracked(),
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
            cfg.get_value(),
            handle_clone.clone(),
        );
    }
}
