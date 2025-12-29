use crate::components::statistics::StateEvent;
use crate::config;
use futures::{SinkExt, StreamExt};
use gloo_timers::future::TimeoutFuture;
use js_sys;
use leptos::prelude::*;
use log::{debug, error};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use shared::events::{
    ChatMessagePayload, ClientEvent, RoomState, SyncSnapshotPayload,
    SyncSnapshotRequestPayload, SyncVersionPayload,
};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use gloo_net::websocket::{futures::WebSocket as GlooWebSocket, Message};
use leptos::task::spawn_local;

#[derive(Clone, Debug, PartialEq)]
pub struct CursorSignals {
    pub x: ReadSignal<i32>,
    pub set_x: WriteSignal<i32>,
    pub y: ReadSignal<i32>,
    pub set_y: WriteSignal<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LocalStorageData {
    version: u64,
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

fn save_state(room_name: &str, version: u64, state: &RoomState) {
    let key = get_storage_key(room_name);
    let data = LocalStorageData {
        version,
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
    config: config::Config,
) {
    // 1. Инициализация состояния (ИСПОЛЬЗУЕМ Rc<RefCell> ВМЕСТО СИГНАЛОВ)
    // Сигналы привязаны к компоненту и умирают при его размонтировании.
    // Rc<RefCell> живут пока на них есть ссылки (в замыкании spawn_local).
    let local_version = Rc::new(RefCell::new(0u64));
    let room_state = Rc::new(RefCell::new(RoomState::default()));

    // Пытаемся загрузить из LS
    if let Some(data) = load_state(&room_name) {
        debug!("Loaded state from LS: v{}", data.version);
        *local_version.borrow_mut() = data.version;
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

    let host = config.api.back_url
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
                        let best_candidates: Vec<&String> = candidates.iter()
                            .filter(|(u, v)| *v == max_ver && *u != my_username_for_timer)
                            .map(|(u, _)| u)
                            .collect();

                        let mut rng = rand::rng();
                        if let Some(target) = best_candidates.choose(&mut rng) {
                            debug!("Requesting snapshot from {}", target);
                            let req = ClientEvent::SyncSnapshotRequest(SyncSnapshotRequestPayload {
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
                                        // 1. Обновляем локальный стейт
                                        room_state.borrow_mut().chat_history.push(msg.clone());

                                        // 2. Инкрементим версию
                                        let mut ver = local_version.borrow_mut();
                                        *ver += 1;
                                        let current_ver = *ver;
                                        drop(ver); // Освобождаем borrow для save_state

                                        // 3. Сохраняем
                                        save_state(
                                            &room_name_clone,
                                            current_ver,
                                            &room_state.borrow()
                                        );

                                        // 4. Логируем событие
                                        let timestamp = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
                                        state_events.update(|events| {
                                            events.push(StateEvent {
                                                version: current_ver,
                                                event_type: "CHAT_MESSAGE".to_string(),
                                                description: format!("{}: {}", msg.username, msg.payload),
                                                timestamp,
                                            });
                                        });

                                        // 5. Обновляем UI (messages_signal живет в App, он безопасен)
                                        messages_signal.update(|msgs| msgs.push(msg));
                                    }

                                    ClientEvent::MouseClickPayload(mouse_event) => {
                                        if mouse_event.user_id == my_username_clone {
                                            continue;
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

                                    ClientEvent::SyncRequest => {
                                        let announce = ClientEvent::SyncVersionAnnounce(SyncVersionPayload {
                                            username: my_username_clone.clone(),
                                            version: *local_version.borrow(),
                                        });
                                        if let Ok(json) = serde_json::to_string(&announce) {
                                            let _ = tx.clone().try_send(Message::Text(json));
                                        }
                                    }

                                    ClientEvent::SyncVersionAnnounce(payload) => {
                                        sync_candidates.borrow_mut().push((payload.username.clone(), payload.version));

                                        let timestamp = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
                                        state_events.update(|events| {
                                            events.push(StateEvent {
                                                version: *local_version.borrow(),
                                                event_type: "SYNC_VERSION_ANNOUNCE".to_string(),
                                                description: format!("{} announced version {}", payload.username, payload.version),
                                                timestamp,
                                            });
                                        });
                                    }

                                    ClientEvent::SyncSnapshotRequest(payload) => {
                                        if payload.target_username == my_username_clone {
                                            debug!("Sending snapshot to requester");
                                            let snapshot = ClientEvent::SyncSnapshot(SyncSnapshotPayload {
                                                version: *local_version.borrow(),
                                                state: room_state.borrow().clone(),
                                            });
                                            if let Ok(json) = serde_json::to_string(&snapshot) {
                                                let _ = tx.clone().try_send(Message::Text(json));
                                            }

                                            let timestamp = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
                                            state_events.update(|events| {
                                                events.push(StateEvent {
                                                    version: *local_version.borrow(),
                                                    event_type: "SYNC_SNAPSHOT_SENT".to_string(),
                                                    description: format!("Sent snapshot v{} to requester", *local_version.borrow()),
                                                    timestamp,
                                                });
                                            });
                                        }
                                    }

                                    ClientEvent::SyncSnapshot(payload) => {
                                        let mut ver = local_version.borrow_mut();
                                        if payload.version > *ver {
                                            debug!("Applying snapshot v{}", payload.version);

                                            *ver = payload.version;
                                            *room_state.borrow_mut() = payload.state.clone();

                                            // Обновляем UI
                                            messages_signal.set(payload.state.chat_history.clone());

                                            save_state(&room_name_clone, payload.version, &payload.state);

                                            let timestamp = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
                                            let current_ver = *ver;
                                            drop(ver);
                                            state_events.update(|events| {
                                                events.push(StateEvent {
                                                    version: current_ver,
                                                    event_type: "SYNC_SNAPSHOT_RECEIVED".to_string(),
                                                    description: format!("Applied snapshot v{} ({} messages)", current_ver, payload.state.chat_history.len()),
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