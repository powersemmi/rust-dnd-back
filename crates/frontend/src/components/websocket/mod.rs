mod storage;
mod sync;
mod types;

pub use types::{ConflictType, CursorSignals, SyncConflict};

use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::config;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket as GlooWebSocket};
use gloo_timers::future::TimeoutFuture;
use js_sys;
use leptos::prelude::*;
use leptos::task::spawn_local;
use log::{debug, error};
use rand::seq::IndexedRandom;
use shared::events::{
    ChatMessagePayload, ClientEvent, PresenceAnnouncePayload, PresenceRequestPayload,
    PresenceResponsePayload, RoomState, SyncSnapshotPayload, SyncSnapshotRequestPayload,
    SyncVersionPayload, VotingCastPayload, VotingEndPayload,
    voting::{VotingResultPayload, VotingStartPayload},
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

    // Загрузка из localStorage
    if let Some(data) = storage::load_state(&room_name) {
        debug!("Loaded state from LS: v{}", data.state.version);
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
                )
                .await;
            }
            Err(e) => error!("WebSocket open error: {:?}", e),
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
            debug!("No sync candidates found. Assuming I am alone or up-to-date.");
            return;
        }

        let my_ver = *local_version.borrow();
        let max_ver = candidates.iter().map(|(_, v)| *v).max().unwrap_or(0);

        if max_ver > my_ver {
            debug!("Found newer version {}. Selecting donor...", max_ver);
            let best_candidates: Vec<&String> = candidates
                .iter()
                .filter(|(u, v)| *v == max_ver && *u != my_username)
                .map(|(u, _)| u)
                .collect();

            let mut rng = rand::rng();
            if let Some(target) = best_candidates.choose(&mut rng) {
                debug!("Requesting snapshot from {}", target);
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
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("WS Received: {}", text);
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    handle_event(
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
                    );
                } else {
                    error!("Failed to parse event: {}", text);
                }
            }
            Err(e) => error!("WebSocket error: {:?}", e),
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_event(
    event: ClientEvent,
    tx: &WsSender,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    sync_candidates: &Rc<RefCell<Vec<(String, u64)>>>,
    my_username: &str,
    room_name: &str,
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
) {
    match event {
        ClientEvent::ChatMessage(msg) => handle_chat_message(
            msg,
            room_state,
            local_version,
            last_synced_version,
            my_username,
            room_name,
            messages_signal,
            state_events,
            has_chat_notification,
            chat_notification_count,
        ),
        ClientEvent::MouseClickPayload(mouse_event) => {
            handle_mouse_event(mouse_event, my_username, set_cursors)
        }
        ClientEvent::SyncRequest => handle_sync_request(tx, local_version, my_username),
        ClientEvent::SyncVersionAnnounce(payload) => {
            handle_sync_announce(payload, sync_candidates, local_version, state_events)
        }
        ClientEvent::SyncSnapshotRequest(payload) => handle_snapshot_request(
            payload,
            tx,
            room_state,
            local_version,
            my_username,
            state_events,
        ),
        ClientEvent::SyncSnapshot(payload) => handle_snapshot(
            payload,
            room_state,
            local_version,
            last_synced_version,
            room_name,
            messages_signal,
            state_events,
            conflict_signal,
            voting_results,
        ),
        ClientEvent::VotingStart(payload) => handle_voting_start(
            payload,
            votings,
            tx,
            my_username,
            local_version,
            state_events,
            has_statistics_notification,
            notification_count,
        ),
        ClientEvent::VotingCast(payload) => {
            handle_voting_cast(payload, votings, local_version, state_events)
        }
        ClientEvent::VotingResult(payload) => handle_voting_result(
            payload,
            votings,
            voting_results,
            room_state,
            local_version,
            last_synced_version,
            room_name,
            state_events,
        ),
        ClientEvent::VotingEnd(payload) => {
            handle_voting_end(payload, votings, local_version, state_events)
        }
        ClientEvent::PresenceRequest(payload) => handle_presence_request(payload, tx, my_username),
        ClientEvent::PresenceResponse(payload) => {
            handle_presence_response(payload, votings, local_version, state_events)
        }
        ClientEvent::PresenceAnnounce(payload) => {
            handle_presence_announce(payload, votings, local_version, state_events)
        }
        _ => {}
    }
}

fn handle_chat_message(
    msg: ChatMessagePayload,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    my_username: &str,
    room_name: &str,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    has_chat_notification: RwSignal<bool>,
    chat_notification_count: RwSignal<u32>,
) {
    debug!("Processing ChatMessage from {}", msg.username);

    let is_from_me = msg.username == my_username;

    // Если сообщение не от текущего пользователя, увеличиваем счётчик уведомлений
    if !is_from_me {
        has_chat_notification.set(true);
        chat_notification_count.update(|count| *count += 1);
    }

    // Обновляем state и получаем новую версию
    let current_ver = {
        let mut state = room_state.borrow_mut();
        state.chat_history.push(msg.clone());
        state.commit_changes();
        state.version
    };

    *local_version.borrow_mut() = current_ver;

    // Обновляем last_synced_version только если сообщение пришло из сети
    if !is_from_me {
        *last_synced_version.borrow_mut() = current_ver;
    }

    storage::save_state(room_name, &room_state.borrow());

    log_event(
        state_events,
        current_ver,
        "CHAT_MESSAGE",
        &format!("{}: {}", msg.username, msg.payload),
    );

    messages_signal.update(|msgs| msgs.push(msg));
}

fn handle_mouse_event(
    mouse_event: shared::events::MouseClickPayload,
    my_username: &str,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
) {
    if mouse_event.user_id == my_username {
        return;
    }

    set_cursors.update(|cursors| {
        if let Some(cursor_signals) = cursors.get(&mouse_event.user_id) {
            cursor_signals.set_x.set(mouse_event.x);
            cursor_signals.set_y.set(mouse_event.y);
        } else {
            let (x, set_x) = signal(mouse_event.x);
            let (y, set_y) = signal(mouse_event.y);
            cursors.insert(
                mouse_event.user_id.clone(),
                CursorSignals { x, set_x, y, set_y },
            );
        }
    });
}

fn handle_sync_request(tx: &WsSender, local_version: &Rc<RefCell<u64>>, my_username: &str) {
    let current_ver = *local_version.borrow();
    let announce = ClientEvent::SyncVersionAnnounce(SyncVersionPayload {
        username: my_username.to_string(),
        version: current_ver,
    });
    if let Ok(json) = serde_json::to_string(&announce) {
        let _ = tx.clone().try_send(Message::Text(json));
    }
}

fn handle_sync_announce(
    payload: SyncVersionPayload,
    sync_candidates: &Rc<RefCell<Vec<(String, u64)>>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    sync_candidates
        .borrow_mut()
        .push((payload.username.clone(), payload.version));

    log_event(
        state_events,
        *local_version.borrow(),
        "SYNC_VERSION_ANNOUNCE",
        &format!("{} announced version {}", payload.username, payload.version),
    );
}

fn handle_snapshot_request(
    payload: SyncSnapshotRequestPayload,
    tx: &WsSender,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    my_username: &str,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    if payload.target_username == my_username {
        debug!("Sending snapshot to requester");
        let state = room_state.borrow().clone();
        let snapshot = ClientEvent::SyncSnapshot(SyncSnapshotPayload {
            version: state.version,
            state: state.clone(),
        });
        if let Ok(json) = serde_json::to_string(&snapshot) {
            let _ = tx.clone().try_send(Message::Text(json));
        }

        log_event(
            state_events,
            *local_version.borrow(),
            "SYNC_SNAPSHOT_SENT",
            &format!("Sent snapshot v{} to requester", state.version),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_snapshot(
    payload: SyncSnapshotPayload,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    room_name: &str,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
) {
    let local_state = room_state.borrow();
    let local_ver = local_state.version;
    let local_hash = local_state.current_hash.clone();
    let local_synced = *last_synced_version.borrow();
    drop(local_state);

    let remote_ver = payload.state.version;
    let remote_hash = &payload.state.current_hash;

    debug!(
        "Validating snapshot: local v{} (hash: {}), remote v{} (hash: {})",
        local_ver, local_hash, remote_ver, remote_hash
    );

    // Валидация через SyncValidator
    match sync::SyncValidator::validate_remote_state(
        local_ver,
        &local_hash,
        local_synced,
        remote_ver,
        remote_hash,
        &payload.state,
    ) {
        Err(conflict) => {
            conflict_signal.set(Some(conflict.clone()));
            log_event(
                state_events,
                local_ver,
                "SYNC_CONFLICT",
                &format!(
                    "Conflict detected: local v{} vs remote v{}",
                    local_ver, remote_ver
                ),
            );
        }
        Ok(()) if remote_ver > local_ver => {
            // Валидация пройдена, применяем стейт
            debug!("Applying snapshot v{}", remote_ver);

            *local_version.borrow_mut() = remote_ver;
            *last_synced_version.borrow_mut() = remote_ver;
            *room_state.borrow_mut() = payload.state.clone();

            messages_signal.set(payload.state.chat_history.clone());
            voting_results.set(payload.state.voting_results.clone());
            storage::save_state(room_name, &payload.state);

            log_event(
                state_events,
                remote_ver,
                "SYNC_SNAPSHOT_RECEIVED",
                &format!(
                    "Applied snapshot v{} ({} messages)",
                    remote_ver,
                    payload.state.chat_history.len()
                ),
            );
        }
        _ => {}
    }
}

fn log_event(
    state_events: RwSignal<Vec<StateEvent>>,
    version: u64,
    event_type: &str,
    description: &str,
) {
    let timestamp = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    state_events.update(|events| {
        events.push(StateEvent {
            version,
            event_type: event_type.to_string(),
            description: description.to_string(),
            timestamp,
        });
    });
}

// Voting event handlers

fn handle_voting_start(
    payload: VotingStartPayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    tx: &WsSender,
    my_username: &str,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
    has_statistics_notification: RwSignal<bool>,
    notification_count: RwSignal<u32>,
) {
    debug!("Voting started: {}", payload.question);
    let voting_id = payload.voting_id.clone();
    let timer_seconds = payload.timer_seconds;

    // Устанавливаем уведомление о новом голосовании и увеличиваем счётчик
    has_statistics_notification.set(true);
    notification_count.update(|count| *count += 1);

    // Отправляем presence response
    let request_id = format!("voting_{}", voting_id);
    let response = ClientEvent::PresenceResponse(PresenceResponsePayload {
        request_id,
        user: my_username.to_string(),
    });
    if let Ok(json) = serde_json::to_string(&response) {
        let _ = tx.clone().try_send(Message::Text(json));
    }

    votings.update(|map| {
        map.insert(
            voting_id.clone(),
            VotingState::Active {
                voting: payload.clone(),
                participants: vec![],
                votes: HashMap::new(),
                remaining_seconds: timer_seconds,
            },
        );
    });

    log_event(
        state_events,
        *local_version.borrow(),
        "VOTING_START",
        &format!(
            "Voting started: {} (by {})",
            payload.question, payload.creator
        ),
    );

    // Запускаем таймер если есть
    if let Some(seconds) = timer_seconds {
        let voting_id_timer = voting_id.clone();
        spawn_local(async move {
            let mut remaining = seconds;
            while remaining > 0 {
                TimeoutFuture::new(1000).await;
                remaining -= 1;
                votings.update(|map| {
                    if let Some(VotingState::Active {
                        remaining_seconds, ..
                    }) = map.get_mut(&voting_id_timer)
                    {
                        *remaining_seconds = Some(remaining);
                    }
                });
            }
        });
    }
}

fn handle_voting_cast(
    payload: VotingCastPayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    debug!(
        "Vote cast by {}: {:?}",
        payload.user, payload.selected_option_ids
    );
    votings.update(|map| {
        if let Some(VotingState::Active { votes, .. }) = map.get_mut(&payload.voting_id) {
            votes.insert(payload.user.clone(), payload.selected_option_ids.clone());
        }
    });

    log_event(
        state_events,
        *local_version.borrow(),
        "VOTING_CAST",
        &format!("{} voted in {}", payload.user, payload.voting_id),
    );
}

fn handle_voting_result(
    payload: VotingResultPayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    room_name: &str,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    debug!("Voting results received for: {}", payload.voting_id);

    // Обновляем текущее состояние голосования
    votings.update(|map| {
        if let Some(state) = map.get(&payload.voting_id) {
            let voting_clone = match state {
                VotingState::Active { voting, .. } => voting.clone(),
                _ => return,
            };
            map.insert(
                payload.voting_id.clone(),
                VotingState::Results {
                    voting: voting_clone,
                    results: payload.results.clone(),
                    total_participants: payload.total_participants,
                    total_voted: payload.total_voted,
                },
            );
        }
    });

    // Сохраняем результаты в статистику
    voting_results.update(|results| {
        results.insert(payload.voting_id.clone(), payload.clone());
    });

    // Сохраняем результаты в RoomState
    let current_ver = {
        let mut state = room_state.borrow_mut();
        state
            .voting_results
            .insert(payload.voting_id.clone(), payload.clone());
        state.commit_changes();
        state.version
    };

    *local_version.borrow_mut() = current_ver;
    *last_synced_version.borrow_mut() = current_ver;

    storage::save_state(room_name, &room_state.borrow());

    log_event(
        state_events,
        current_ver,
        "VOTING_RESULT",
        &format!("Voting {} completed", payload.voting_id),
    );
}

fn handle_voting_end(
    payload: VotingEndPayload,
    _votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    debug!("Voting ended: {}", payload.voting_id);
    // Не удаляем голосование, оно уже должно быть в состоянии Results после VotingResult
    // Просто логируем событие
    log_event(
        state_events,
        *local_version.borrow(),
        "VOTING_END",
        &format!("Voting {} ended", payload.voting_id),
    );
}

fn handle_presence_request(payload: PresenceRequestPayload, tx: &WsSender, my_username: &str) {
    debug!("Presence request from: {}", payload.requester);
    let response = ClientEvent::PresenceResponse(PresenceResponsePayload {
        request_id: payload.request_id,
        user: my_username.to_string(),
    });
    if let Ok(json) = serde_json::to_string(&response) {
        let _ = tx.clone().try_send(Message::Text(json));
    }
}

fn handle_presence_response(
    payload: PresenceResponsePayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    debug!("Presence response from: {}", payload.user);

    // Извлекаем voting_id из request_id (формат: "voting_{voting_id}")
    if let Some(voting_id) = payload.request_id.strip_prefix("voting_") {
        votings.update(|map| {
            if let Some(VotingState::Active { participants, .. }) = map.get_mut(voting_id) {
                if !participants.contains(&payload.user) {
                    participants.push(payload.user.clone());
                }
            }
        });

        log_event(
            state_events,
            *local_version.borrow(),
            "PRESENCE_RESPONSE",
            &format!("{} joined voting {}", payload.user, voting_id),
        );
    }
}

fn handle_presence_announce(
    payload: PresenceAnnouncePayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    debug!("Presence announce: {:?}", payload.online_users);

    // Извлекаем voting_id из request_id
    if let Some(voting_id) = payload.request_id.strip_prefix("voting_") {
        votings.update(|map| {
            if let Some(VotingState::Active { participants, .. }) = map.get_mut(voting_id) {
                *participants = payload.online_users.clone();
            }
        });

        log_event(
            state_events,
            *local_version.borrow(),
            "PRESENCE_ANNOUNCE",
            &format!(
                "Voting {} participants announced: {}",
                voting_id,
                payload.online_users.join(", ")
            ),
        );
    }
}
