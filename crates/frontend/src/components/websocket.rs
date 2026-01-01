use crate::components::statistics::StateEvent;
use crate::config;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket as GlooWebSocket};
use gloo_timers::future::TimeoutFuture;
use js_sys;
use leptos::prelude::*;
use leptos::task::spawn_local;
use log::{debug, error, warn};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use shared::events::{
    ChatMessagePayload, ClientEvent, RoomState, SyncSnapshotPayload, SyncSnapshotRequestPayload,
    SyncVersionPayload,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum ConflictType {
    SplitBrain,    // Версии равны, хеши различны
    Fork,          // Наш хеш не найден в истории удалённого стейта
    UnsyncedLocal, // Удалённый стейт новее, но у нас есть несинхронизированные изменения
}

#[derive(Clone, Debug)]
pub struct SyncConflict {
    pub conflict_type: ConflictType,
    pub local_version: u64,
    pub remote_version: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CursorSignals {
    pub x: ReadSignal<i32>,
    pub set_x: WriteSignal<i32>,
    pub y: ReadSignal<i32>,
    pub set_y: WriteSignal<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LocalStorageData {
    state: RoomState,
}

// --- Хелперы для LocalStorage ---

fn get_storage_key(room_name: &str) -> String {
    format!("dnd_room_state:{}", room_name)
}

fn load_state(room_name: &str) -> Option<LocalStorageData> {
    let key = get_storage_key(room_name);
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item(&key) {
                return serde_json::from_str(&json).ok();
            }
        }
    }
    None
}

fn save_state(room_name: &str, state: &RoomState) {
    let key = get_storage_key(room_name);
    let data = LocalStorageData {
        state: state.clone(),
    };
    if let Ok(json) = serde_json::to_string(&data) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(&key, &json);
            }
        }
    }
}

// --- Main Logic ---

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
    config: config::Config,
) {
    // 1. Инициализация состояния (ИСПОЛЬЗУЕМ Rc<RefCell> ВМЕСТО СИГНАЛОВ)
    // Сигналы привязаны к компоненту и умирают при его размонтировании.
    // Rc<RefCell> живут пока на них есть ссылки (в замыкании spawn_local).
    let local_version = Rc::new(RefCell::new(0u64));
    let room_state = Rc::new(RefCell::new(RoomState::default()));
    let last_synced_version = Rc::new(RefCell::new(0u64));

    // Пытаемся загрузить из LS
    if let Some(data) = load_state(&room_name) {
        debug!("Loaded state from LS: v{}", data.state.version);
        *local_version.borrow_mut() = data.state.version;
        *last_synced_version.borrow_mut() = data.state.version;
        *room_state.borrow_mut() = data.state.clone();
        messages_signal.set(data.state.chat_history);
    }

    let room_name_clone = room_name.clone();
    let my_username_clone = my_username.clone();

    // 2. URL
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

    let ws_url = format!(
        "{}{}{}?room_id={}&token={}",
        ws_protocol, host, config.api.ws_path, room_name, jwt_token
    );

    // 3. Подключение
    spawn_local(async move {
        match GlooWebSocket::open(&ws_url) {
            Ok(ws) => {
                let (mut write, mut read) = ws.split();
                let (tx, mut rx) = futures::channel::mpsc::channel::<Message>(1000);
                set_ws_sender.set(Some(tx.clone()));

                spawn_local(async move {
                    while let Some(msg) = rx.next().await {
                        let _ = write.send(msg).await;
                    }
                });

                // --- ЛОГИКА СИНХРОНИЗАЦИИ ---

                let sync_req = ClientEvent::SyncRequest;
                if let Ok(json) = serde_json::to_string(&sync_req) {
                    let _ = tx.clone().try_send(Message::Text(json));
                }

                // Кандидаты (Rc<RefCell> вместо RwSignal)
                let sync_candidates = Rc::new(RefCell::new(Vec::<(String, u64)>::new()));

                let tx_clone_for_timer = tx.clone();
                let my_username_for_timer = my_username_clone.clone();
                let sync_candidates_timer = sync_candidates.clone();
                let local_version_timer = local_version.clone();

                spawn_local(async move {
                    TimeoutFuture::new(1000).await;

                    let candidates = sync_candidates_timer.borrow();
                    if candidates.is_empty() {
                        debug!("No sync candidates found. Assuming I am alone or up-to-date.");
                        return;
                    }

                    let my_ver = *local_version_timer.borrow();
                    let max_ver = candidates.iter().map(|(_, v)| *v).max().unwrap_or(0);

                    if max_ver > my_ver {
                        debug!("Found newer version {}. Selecting donor...", max_ver);
                        let best_candidates: Vec<&String> = candidates
                            .iter()
                            .filter(|(u, v)| *v == max_ver && *u != my_username_for_timer)
                            .map(|(u, _)| u)
                            .collect();

                        let mut rng = rand::rng();
                        if let Some(target) = best_candidates.choose(&mut rng) {
                            debug!("Requesting snapshot from {}", target);
                            let req =
                                ClientEvent::SyncSnapshotRequest(SyncSnapshotRequestPayload {
                                    target_username: target.to_string(),
                                });
                            if let Ok(json) = serde_json::to_string(&req) {
                                let _ = tx_clone_for_timer.clone().try_send(Message::Text(json));
                            }
                        }
                    }
                });

                // Основной цикл чтения
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            debug!("WS Received: {}", text);
                            if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                                match event {
                                    ClientEvent::ChatMessage(msg) => {
                                        debug!("Processing ChatMessage from {}", msg.username);

                                        let is_from_me = msg.username == my_username_clone;

                                        // 1. Обновляем локальный стейт
                                        {
                                            let mut state = room_state.borrow_mut();
                                            state.chat_history.push(msg.clone());
                                            state.commit_changes();
                                        }

                                        // 2. Получаем текущую версию
                                        let current_ver = room_state.borrow().version;
                                        *local_version.borrow_mut() = current_ver;

                                        // 3. Обновляем last_synced_version только если сообщение пришло из сети
                                        if !is_from_me {
                                            *last_synced_version.borrow_mut() = current_ver;
                                        }

                                        // 4. Сохраняем
                                        save_state(&room_name_clone, &room_state.borrow());

                                        // 5. Логируем событие
                                        let timestamp = js_sys::Date::new_0()
                                            .to_iso_string()
                                            .as_string()
                                            .unwrap_or_default();
                                        state_events.update(|events| {
                                            events.push(StateEvent {
                                                version: current_ver,
                                                event_type: "CHAT_MESSAGE".to_string(),
                                                description: format!(
                                                    "{}: {}",
                                                    msg.username, msg.payload
                                                ),
                                                timestamp,
                                            });
                                        });

                                        // 6. Обновляем UI (messages_signal живет в App, он безопасен)
                                        messages_signal.update(|msgs| msgs.push(msg));
                                    }

                                    ClientEvent::MouseClickPayload(mouse_event) => {
                                        if mouse_event.user_id == my_username_clone {
                                            continue;
                                        }
                                        set_cursors.update(|cursors| {
                                            if let Some(cursor_signals) =
                                                cursors.get(&mouse_event.user_id)
                                            {
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

                                    ClientEvent::SyncRequest => {
                                        let current_ver = room_state.borrow().version;
                                        let announce =
                                            ClientEvent::SyncVersionAnnounce(SyncVersionPayload {
                                                username: my_username_clone.clone(),
                                                version: current_ver,
                                            });
                                        if let Ok(json) = serde_json::to_string(&announce) {
                                            let _ = tx.clone().try_send(Message::Text(json));
                                        }
                                    }

                                    ClientEvent::SyncVersionAnnounce(payload) => {
                                        sync_candidates
                                            .borrow_mut()
                                            .push((payload.username.clone(), payload.version));

                                        let timestamp = js_sys::Date::new_0()
                                            .to_iso_string()
                                            .as_string()
                                            .unwrap_or_default();
                                        state_events.update(|events| {
                                            events.push(StateEvent {
                                                version: *local_version.borrow(),
                                                event_type: "SYNC_VERSION_ANNOUNCE".to_string(),
                                                description: format!(
                                                    "{} announced version {}",
                                                    payload.username, payload.version
                                                ),
                                                timestamp,
                                            });
                                        });
                                    }

                                    ClientEvent::SyncSnapshotRequest(payload) => {
                                        if payload.target_username == my_username_clone {
                                            debug!("Sending snapshot to requester");
                                            let state = room_state.borrow().clone();
                                            let snapshot =
                                                ClientEvent::SyncSnapshot(SyncSnapshotPayload {
                                                    version: state.version,
                                                    state: state.clone(),
                                                });
                                            if let Ok(json) = serde_json::to_string(&snapshot) {
                                                let _ = tx.clone().try_send(Message::Text(json));
                                            }

                                            let timestamp = js_sys::Date::new_0()
                                                .to_iso_string()
                                                .as_string()
                                                .unwrap_or_default();
                                            state_events.update(|events| {
                                                events.push(StateEvent {
                                                    version: state.version,
                                                    event_type: "SYNC_SNAPSHOT_SENT".to_string(),
                                                    description: format!(
                                                        "Sent snapshot v{} to requester",
                                                        state.version
                                                    ),
                                                    timestamp,
                                                });
                                            });
                                        }
                                    }

                                    ClientEvent::SyncSnapshot(payload) => {
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

                                        // Проверка на конфликты
                                        let mut has_conflict = false;
                                        let mut conflict_type = None;

                                        if remote_ver == local_ver {
                                            // Сценарий 2: Split Brain
                                            if remote_hash != &local_hash {
                                                warn!(
                                                    "CONFLICT: Split Brain detected! Same version but different hashes."
                                                );
                                                has_conflict = true;
                                                conflict_type = Some(ConflictType::SplitBrain);
                                            }
                                        } else if remote_ver > local_ver {
                                            // Сценарий 1: Fast-Forward
                                            if !payload
                                                .state
                                                .has_version_with_hash(local_ver, &local_hash)
                                            {
                                                warn!(
                                                    "CONFLICT: Fork detected! Our hash not found in remote history."
                                                );
                                                has_conflict = true;
                                                conflict_type = Some(ConflictType::Fork);
                                            }

                                            // Сценарий 3: Unsynced Local Changes
                                            if local_ver > local_synced {
                                                warn!("CONFLICT: Unsynced local changes detected!");
                                                has_conflict = true;
                                                conflict_type = Some(ConflictType::UnsyncedLocal);
                                            }
                                        }

                                        if has_conflict {
                                            // Сохраняем конфликт для показа пользователю
                                            conflict_signal.set(Some(SyncConflict {
                                                conflict_type: conflict_type.unwrap(),
                                                local_version: local_ver,
                                                remote_version: remote_ver,
                                            }));

                                            let timestamp = js_sys::Date::new_0()
                                                .to_iso_string()
                                                .as_string()
                                                .unwrap_or_default();
                                            state_events.update(|events| {
                                                events.push(StateEvent {
                                                    version: local_ver,
                                                    event_type: "SYNC_CONFLICT".to_string(),
                                                    description: format!(
                                                        "Conflict detected: local v{} vs remote v{}",
                                                        local_ver, remote_ver
                                                    ),
                                                    timestamp,
                                                });
                                            });
                                        } else if remote_ver > local_ver {
                                            // Валидация пройдена, применяем стейт
                                            debug!("Applying snapshot v{}", remote_ver);

                                            *local_version.borrow_mut() = remote_ver;
                                            *last_synced_version.borrow_mut() = remote_ver;
                                            *room_state.borrow_mut() = payload.state.clone();

                                            // Обновляем UI
                                            messages_signal.set(payload.state.chat_history.clone());
                                            save_state(&room_name_clone, &payload.state);

                                            let timestamp = js_sys::Date::new_0()
                                                .to_iso_string()
                                                .as_string()
                                                .unwrap_or_default();
                                            state_events.update(|events| {
                                                events.push(StateEvent {
                                                    version: remote_ver,
                                                    event_type: "SYNC_SNAPSHOT_RECEIVED"
                                                        .to_string(),
                                                    description: format!(
                                                        "Applied snapshot v{} ({} messages)",
                                                        remote_ver,
                                                        payload.state.chat_history.len()
                                                    ),
                                                    timestamp,
                                                });
                                            });
                                        }
                                    }

                                    _ => {}
                                }
                            } else {
                                error!("Failed to parse event: {}", text);
                            }
                        }
                        Err(e) => error!("WebSocket error: {:?}", e),
                        _ => {}
                    }
                }
            }
            Err(e) => error!("WebSocket open error: {:?}", e),
        }
    });
}
