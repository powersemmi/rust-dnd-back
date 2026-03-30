use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{
    ConflictResolutionHandle, ConnectWebSocketArgs, CursorSignals, FileTransferState, SyncConflict,
    WsSender, connect_websocket,
};
use crate::config;
use crate::utils::{auth, token_refresh};
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, NotePayload, Scene, voting::VotingResultPayload};
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

pub struct RoomSelectedCallbackArgs {
    pub set_room_id: WriteSignal<String>,
    pub set_app_state: WriteSignal<AppState>,
    pub jwt_token: ReadSignal<String>,
    pub username: ReadSignal<String>,
    pub file_transfer: FileTransferState,
    pub set_ws_sender: WriteSignal<Option<WsSender>>,
    pub set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    pub messages: RwSignal<Vec<ChatMessagePayload>>,
    pub public_notes: RwSignal<Vec<NotePayload>>,
    pub direct_notes: RwSignal<Vec<NotePayload>>,
    pub direct_note_recipients: RwSignal<Vec<String>>,
    pub direct_note_recipients_cache_updated_at_ms: RwSignal<Option<f64>>,
    pub direct_note_recipients_request_id: RwSignal<Option<String>>,
    pub state_events: RwSignal<Vec<StateEvent>>,
    pub scenes: RwSignal<Vec<Scene>>,
    pub active_scene_id: RwSignal<Option<String>>,
    pub conflict_signal: RwSignal<Option<SyncConflict>>,
    pub votings: RwSignal<HashMap<String, VotingState>>,
    pub voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    pub has_statistics_notification: RwSignal<bool>,
    pub notification_count: RwSignal<u32>,
    pub has_chat_notification: RwSignal<bool>,
    pub chat_notification_count: RwSignal<u32>,
    pub cfg: StoredValue<config::Config>,
    pub conflict_resolution_handle: ConflictResolutionHandle,
}

/// Создает callback для выбора комнаты и подключения к WebSocket
pub fn create_room_selected_callback(args: RoomSelectedCallbackArgs) -> impl Fn(String) + Clone {
    let RoomSelectedCallbackArgs {
        set_room_id,
        set_app_state,
        jwt_token,
        username,
        file_transfer,
        set_ws_sender,
        set_cursors,
        messages,
        public_notes,
        direct_notes,
        direct_note_recipients,
        direct_note_recipients_cache_updated_at_ms,
        direct_note_recipients_request_id,
        state_events,
        scenes,
        active_scene_id,
        conflict_signal,
        votings,
        voting_results,
        has_statistics_notification,
        notification_count,
        has_chat_notification,
        chat_notification_count,
        cfg,
        conflict_resolution_handle,
    } = args;
    let handle_clone = conflict_resolution_handle.clone();
    move |selected_room_id: String| {
        file_transfer.reset();
        set_room_id.set(selected_room_id.clone());
        set_app_state.set(AppState::Connected);

        // Подключаемся к WebSocket
        connect_websocket(ConnectWebSocketArgs {
            room_name: selected_room_id,
            jwt_token: jwt_token.get(),
            my_username: username.get_untracked(),
            file_transfer: file_transfer.clone(),
            set_ws_sender,
            set_cursors,
            messages_signal: messages,
            public_notes_signal: public_notes,
            direct_notes_signal: direct_notes,
            direct_note_recipients_signal: direct_note_recipients,
            direct_note_recipients_cache_updated_at_ms_signal:
                direct_note_recipients_cache_updated_at_ms,
            direct_note_recipients_request_id_signal: direct_note_recipients_request_id,
            state_events,
            scenes_signal: scenes,
            active_scene_id_signal: active_scene_id,
            conflict_signal,
            votings,
            voting_results,
            has_statistics_notification,
            notification_count,
            has_chat_notification,
            chat_notification_count,
            config: cfg.get_value(),
            conflict_resolution_handle: handle_clone.clone(),
        });
    }
}
