use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{storage, types::CursorSignals};
use crate::config;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket as GlooWebSocket};
use gloo_timers::future::TimeoutFuture;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use rand::seq::IndexedRandom;
use shared::events::{
    ChatMessagePayload, ClientEvent, RoomState, SyncSnapshotRequestPayload, VotingResultPayload,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::handlers;
use super::types::SyncConflict;

pub type WsSender = futures::channel::mpsc::Sender<Message>;

pub fn connect_websocket(
    room_name: String,
    jwt_token: String,
    my_username: String,
    set_ws_sender: WriteSignal<Option<WsSender>>,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
    votings: RwSignal<HashMap<String, VotingState>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    has_statistics_notification: RwSignal<bool>,
    notification_count: RwSignal<u32>,
    has_chat_notification: RwSignal<bool>,
    chat_notification_count: RwSignal<u32>,
    config: config::Config,
) {
    // Инициализация состояния
    let local_version = Rc::new(RefCell::new(0u64));
    let room_state = Rc::new(RefCell::new(RoomState::default()));
    let last_synced_version = Rc::new(RefCell::new(0u64));
    let expected_snapshot_from: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let collected_snapshots: Rc<RefCell<Vec<(String, RoomState)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let is_collecting_snapshots: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    // Загрузка из localStorage
    if let Some(data) = storage::load_state(&room_name) {
        log!("Loaded state from LS: v{}", data.state.version);
        *local_version.borrow_mut() = data.state.version;
        *last_synced_version.borrow_mut() = data.state.version;
        *room_state.borrow_mut() = data.state.clone();
        messages_signal.set(data.state.chat_history.clone());
        voting_results.set(data.state.voting_results);
    }

    let room_name_clone = room_name.clone();
    let my_username_clone = my_username.clone();

    // Построение WebSocket URL
    let ws_url = build_ws_url(&config, &room_name, &jwt_token);

    // Подключение
    spawn_local(async move {
        match GlooWebSocket::open(&ws_url) {
            Ok(ws) => {
                let (mut write, read) = ws.split();
                let (tx, mut rx) = futures::channel::mpsc::channel::<Message>(1000);
                set_ws_sender.set(Some(tx.clone()));

                // Поток отправки сообщений
                spawn_local(async move {
                    while let Some(msg) = rx.next().await {
                        let _ = write.send(msg).await;
                    }
                });

                // Инициализация синхронизации
                let sync_candidates = init_sync(&tx);

                // Таймер выбора донора для синхронизации
                start_sync_timer(
                    sync_candidates.clone(),
                    local_version.clone(),
                    my_username_clone.clone(),
                    tx.clone(),
                );

                // Основной цикл обработки сообщений
                process_messages(
                    read,
                    tx,
                    room_state,
                    local_version,
                    last_synced_version,
                    sync_candidates,
                    my_username_clone,
                    room_name_clone,
                    set_cursors,
                    messages_signal,
                    state_events,
                    conflict_signal,
                    votings,
                    voting_results,
                    has_statistics_notification,
                    notification_count,
                    has_chat_notification,
                    chat_notification_count,
                    expected_snapshot_from,
                    collected_snapshots,
                    is_collecting_snapshots,
                )
                .await;
            }
            Err(e) => log!("WebSocket open error: {:?}", e),
        }
    });
}

fn build_ws_url(config: &config::Config, room_name: &str, jwt_token: &str) -> String {
    let ws_protocol = if config.api.back_url.starts_with("https://") {
        "wss://"
    } else {
        "ws://"
    };

    let host = config
        .api
        .back_url
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    format!(
        "{}{}{}?room_id={}&token={}",
        ws_protocol, host, config.api.ws_path, room_name, jwt_token
    )
}

fn init_sync(tx: &WsSender) -> Rc<RefCell<Vec<(String, u64)>>> {
    let sync_req = ClientEvent::SyncRequest;
    if let Ok(json) = serde_json::to_string(&sync_req) {
        let _ = tx.clone().try_send(Message::Text(json));
    }
    Rc::new(RefCell::new(Vec::new()))
}

fn start_sync_timer(
    sync_candidates: Rc<RefCell<Vec<(String, u64)>>>,
    local_version: Rc<RefCell<u64>>,
    my_username: String,
    tx: WsSender,
) {
    spawn_local(async move {
        TimeoutFuture::new(1000).await;

        let candidates = sync_candidates.borrow();
        if candidates.is_empty() {
            log!("No sync candidates found. Assuming I am alone or up-to-date.");
            return;
        }

        let my_ver = *local_version.borrow();
        let max_ver = candidates.iter().map(|(_, v)| *v).max().unwrap_or(0);

        if max_ver > my_ver {
            log!("Found newer version {}. Selecting donor...", max_ver);
            let best_candidates: Vec<&String> = candidates
                .iter()
                .filter(|(u, v)| *v == max_ver && *u != my_username)
                .map(|(u, _)| u)
                .collect();

            let mut rng = rand::rng();
            if let Some(target) = best_candidates.choose(&mut rng) {
                log!("Requesting snapshot from {}", target);
                let req = ClientEvent::SyncSnapshotRequest(SyncSnapshotRequestPayload {
                    target_username: target.to_string(),
                });
                if let Ok(json) = serde_json::to_string(&req) {
                    let _ = tx.clone().try_send(Message::Text(json));
                }
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn process_messages(
    mut read: futures::stream::SplitStream<GlooWebSocket>,
    tx: WsSender,
    room_state: Rc<RefCell<RoomState>>,
    local_version: Rc<RefCell<u64>>,
    last_synced_version: Rc<RefCell<u64>>,
    sync_candidates: Rc<RefCell<Vec<(String, u64)>>>,
    my_username: String,
    room_name: String,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
    votings: RwSignal<HashMap<String, VotingState>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    has_statistics_notification: RwSignal<bool>,
    notification_count: RwSignal<u32>,
    has_chat_notification: RwSignal<bool>,
    chat_notification_count: RwSignal<u32>,
    expected_snapshot_from: Rc<RefCell<Option<String>>>,
    collected_snapshots: Rc<RefCell<Vec<(String, RoomState)>>>,
    is_collecting_snapshots: Rc<RefCell<bool>>,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    handlers::handle_event(
                        event,
                        &tx,
                        &room_state,
                        &local_version,
                        &last_synced_version,
                        &sync_candidates,
                        &my_username,
                        &room_name,
                        set_cursors,
                        messages_signal,
                        state_events,
                        conflict_signal,
                        votings,
                        voting_results,
                        has_statistics_notification,
                        notification_count,
                        has_chat_notification,
                        chat_notification_count,
                        &expected_snapshot_from,
                        &collected_snapshots,
                        &is_collecting_snapshots,
                    );
                } else {
                    log!("Failed to parse event: {}", text);
                }
            }
            Err(e) => log!("WebSocket error: {:?}", e),
            _ => {}
        }
    }
}
