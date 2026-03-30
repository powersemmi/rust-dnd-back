use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{FileTransferState, storage, types::CursorSignals};
use crate::config;
use futures::{FutureExt, SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket as GlooWebSocket};
use gloo_timers::future::TimeoutFuture;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use rand::seq::IndexedRandom;
use shared::events::{
    ChatMessagePayload, ClientEvent, RoomState, Scene, SyncSnapshotRequestPayload,
    VotingResultPayload,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use super::handlers;
use super::types::{ConflictResolutionHandle, SyncConflict};

const HIGH_PRIORITY_QUEUE_CAPACITY: usize = 256;
const NORMAL_PRIORITY_QUEUE_CAPACITY: usize = 512;
const LOW_PRIORITY_QUEUE_CAPACITY: usize = 2048;
const LOW_PRIORITY_DELAY_MS: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutboundPriority {
    High,
    Normal,
    Low,
}

#[derive(Clone)]
pub struct WsSender {
    high: futures::channel::mpsc::Sender<Message>,
    normal: futures::channel::mpsc::Sender<Message>,
    low: futures::channel::mpsc::Sender<Message>,
}

impl WsSender {
    fn new(
        high: futures::channel::mpsc::Sender<Message>,
        normal: futures::channel::mpsc::Sender<Message>,
        low: futures::channel::mpsc::Sender<Message>,
    ) -> Self {
        Self { high, normal, low }
    }

    pub fn try_send_event(&self, event: ClientEvent) -> Result<(), String> {
        let priority = Self::priority_for_event(&event);
        self.try_send_event_with_priority(event, priority)
    }

    pub fn try_send_event_with_priority(
        &self,
        event: ClientEvent,
        priority: OutboundPriority,
    ) -> Result<(), String> {
        let json = serde_json::to_string(&event)
            .map_err(|error| format!("failed to serialize websocket event: {error}"))?;
        self.try_send_text_with_priority(json, priority)
    }

    pub fn try_send_text_with_priority(
        &self,
        text: String,
        priority: OutboundPriority,
    ) -> Result<(), String> {
        let queue = match priority {
            OutboundPriority::High => &self.high,
            OutboundPriority::Normal => &self.normal,
            OutboundPriority::Low => &self.low,
        };

        queue
            .clone()
            .try_send(Message::Text(text))
            .map_err(|error| format!("failed to enqueue {priority:?} websocket message: {error:?}"))
    }

    fn priority_for_event(event: &ClientEvent) -> OutboundPriority {
        match event {
            ClientEvent::Ping
            | ClientEvent::ChatMessage(_)
            | ClientEvent::MouseClickPayload(_)
            | ClientEvent::VotingCast(_)
            | ClientEvent::SyncSnapshotRequest(_)
            | ClientEvent::SyncSnapshot(_) => OutboundPriority::High,
            ClientEvent::FileChunk(_) => OutboundPriority::Low,
            ClientEvent::RoomState(_)
            | ClientEvent::FileAnnounce(_)
            | ClientEvent::FileRequest(_)
            | ClientEvent::FileAbort(_)
            | ClientEvent::SceneCreate(_)
            | ClientEvent::SceneUpdate(_)
            | ClientEvent::SceneDelete(_)
            | ClientEvent::SceneActivate(_)
            | ClientEvent::TokenMove(_)
            | ClientEvent::SyncRequest
            | ClientEvent::SyncVersionAnnounce(_)
            | ClientEvent::VotingStart(_)
            | ClientEvent::VotingResult(_)
            | ClientEvent::VotingEnd(_)
            | ClientEvent::PresenceRequest(_)
            | ClientEvent::PresenceResponse(_)
            | ClientEvent::PresenceAnnounce(_) => OutboundPriority::Normal,
        }
    }
}

pub struct ConnectWebSocketArgs {
    pub room_name: String,
    pub jwt_token: String,
    pub my_username: String,
    pub file_transfer: FileTransferState,
    pub set_ws_sender: WriteSignal<Option<WsSender>>,
    pub set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    pub messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    pub state_events: RwSignal<Vec<StateEvent>>,
    pub scenes_signal: RwSignal<Vec<Scene>>,
    pub active_scene_id_signal: RwSignal<Option<String>>,
    pub conflict_signal: RwSignal<Option<SyncConflict>>,
    pub votings: RwSignal<HashMap<String, VotingState>>,
    pub voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    pub has_statistics_notification: RwSignal<bool>,
    pub notification_count: RwSignal<u32>,
    pub has_chat_notification: RwSignal<bool>,
    pub chat_notification_count: RwSignal<u32>,
    pub config: config::Config,
    pub conflict_resolution_handle: ConflictResolutionHandle,
}

struct MessageProcessingContext {
    tx: WsSender,
    file_transfer: FileTransferState,
    room_state: Rc<RefCell<RoomState>>,
    local_version: Rc<RefCell<u64>>,
    last_synced_version: Rc<RefCell<u64>>,
    sync_candidates: Rc<RefCell<Vec<(String, u64)>>>,
    my_username: String,
    room_name: String,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    scenes_signal: RwSignal<Vec<Scene>>,
    active_scene_id_signal: RwSignal<Option<String>>,
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
    collected_announces: Rc<RefCell<Vec<shared::events::SyncVersionPayload>>>,
    is_collecting_announces: Rc<RefCell<bool>>,
}

impl MessageProcessingContext {
    fn handler_context(&self) -> handlers::HandlerContext<'_> {
        handlers::HandlerContext {
            tx: &self.tx,
            file_transfer: &self.file_transfer,
            room_state: &self.room_state,
            local_version: &self.local_version,
            last_synced_version: &self.last_synced_version,
            sync_candidates: &self.sync_candidates,
            my_username: &self.my_username,
            room_name: &self.room_name,
            set_cursors: self.set_cursors,
            messages_signal: self.messages_signal,
            state_events: self.state_events,
            scenes_signal: self.scenes_signal,
            active_scene_id_signal: self.active_scene_id_signal,
            conflict_signal: self.conflict_signal,
            votings: self.votings,
            voting_results: self.voting_results,
            has_statistics_notification: self.has_statistics_notification,
            notification_count: self.notification_count,
            has_chat_notification: self.has_chat_notification,
            chat_notification_count: self.chat_notification_count,
            expected_snapshot_from: &self.expected_snapshot_from,
            collected_snapshots: &self.collected_snapshots,
            is_collecting_snapshots: &self.is_collecting_snapshots,
            collected_announces: &self.collected_announces,
            is_collecting_announces: &self.is_collecting_announces,
        }
    }
}

pub fn connect_websocket(args: ConnectWebSocketArgs) {
    let ConnectWebSocketArgs {
        room_name,
        jwt_token,
        my_username,
        file_transfer,
        set_ws_sender,
        set_cursors,
        messages_signal,
        state_events,
        scenes_signal,
        active_scene_id_signal,
        conflict_signal,
        votings,
        voting_results,
        has_statistics_notification,
        notification_count,
        has_chat_notification,
        chat_notification_count,
        config,
        conflict_resolution_handle,
    } = args;

    // Инициализация состояния
    let local_version = Rc::new(RefCell::new(0u64));
    let room_state = Rc::new(RefCell::new(RoomState::default()));
    let last_synced_version = Rc::new(RefCell::new(0u64));
    let expected_snapshot_from: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let collected_snapshots: Rc<RefCell<Vec<(String, RoomState)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let is_collecting_snapshots: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    // Для конфликт-резолюции через сбор анонсов
    let collected_announces: Rc<RefCell<Vec<shared::events::SyncVersionPayload>>> =
        Rc::new(RefCell::new(Vec::new()));
    let is_collecting_announces: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    let room_name_for_storage = room_name.clone();
    let room_name_clone = room_name.clone();
    let my_username_clone = my_username.clone();

    // Построение WebSocket URL
    let ws_url = build_ws_url(&config, &room_name, &jwt_token);

    // Подключение
    spawn_local(async move {
        match storage::load_state(&room_name_for_storage).await {
            Ok(Some(data)) => {
                log!("Loaded state from IndexedDB: v{}", data.state.version);
                *local_version.borrow_mut() = data.state.version;
                *last_synced_version.borrow_mut() = data.state.version;
                *room_state.borrow_mut() = data.state.clone();
                messages_signal.set(data.state.chat_history.clone());
                file_transfer.reconcile_chat_attachments(&data.state.chat_history);
                voting_results.set(data.state.voting_results.clone());
                scenes_signal.set(data.state.scenes.clone());
                active_scene_id_signal.set(data.state.active_scene_id.clone());
            }
            Ok(None) => {}
            Err(error) => log!("Failed to load state from IndexedDB: {}", error),
        }

        match GlooWebSocket::open(&ws_url) {
            Ok(ws) => {
                let (write, read) = ws.split();
                let (high_tx, high_rx) =
                    futures::channel::mpsc::channel::<Message>(HIGH_PRIORITY_QUEUE_CAPACITY);
                let (normal_tx, normal_rx) =
                    futures::channel::mpsc::channel::<Message>(NORMAL_PRIORITY_QUEUE_CAPACITY);
                let (low_tx, low_rx) =
                    futures::channel::mpsc::channel::<Message>(LOW_PRIORITY_QUEUE_CAPACITY);
                let tx = WsSender::new(high_tx, normal_tx, low_tx);
                set_ws_sender.set(Some(tx.clone()));

                // Устанавливаем callback для разрешения конфликта
                let tx_for_callback = tx.clone();
                let collected_announces_cb = collected_announces.clone();
                let is_collecting_announces_cb = is_collecting_announces.clone();
                let expected_snapshot_from_cb = expected_snapshot_from.clone();

                conflict_resolution_handle.set_callback(move || {
                    use crate::components::websocket::handlers::sync_discard;
                    sync_discard::start_conflict_resolution(
                        sync_discard::ConflictResolutionContext {
                            tx: &tx_for_callback,
                            collected_announces: &collected_announces_cb,
                            is_collecting_announces: &is_collecting_announces_cb,
                            expected_snapshot_from: &expected_snapshot_from_cb,
                        },
                    );
                });

                spawn_local(run_outbound_scheduler(write, high_rx, normal_rx, low_rx));

                // Инициализация синхронизации
                let sync_candidates = init_sync(&tx);

                // Таймер выбора донора для синхронизации
                start_sync_timer(
                    sync_candidates.clone(),
                    local_version.clone(),
                    my_username_clone.clone(),
                    tx.clone(),
                );

                // Таймер пинга для поддержания WebSocket соединения
                start_ping_timer(tx.clone());

                // Таймер для скрытия неактивных курсоров
                start_cursor_cleanup_timer(set_cursors);

                // Основной цикл обработки сообщений
                process_messages(
                    read,
                    MessageProcessingContext {
                        tx,
                        file_transfer,
                        room_state,
                        local_version,
                        last_synced_version,
                        sync_candidates,
                        my_username: my_username_clone,
                        room_name: room_name_clone,
                        set_cursors,
                        messages_signal,
                        state_events,
                        scenes_signal,
                        active_scene_id_signal,
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
                        collected_announces,
                        is_collecting_announces,
                    },
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

async fn run_outbound_scheduler(
    mut write: futures::stream::SplitSink<GlooWebSocket, Message>,
    mut high_rx: futures::channel::mpsc::Receiver<Message>,
    mut normal_rx: futures::channel::mpsc::Receiver<Message>,
    mut low_rx: futures::channel::mpsc::Receiver<Message>,
) {
    let mut high_queue = VecDeque::new();
    let mut normal_queue = VecDeque::new();
    let mut low_queue = VecDeque::new();
    let mut high_closed = false;
    let mut normal_closed = false;
    let mut low_closed = false;

    loop {
        drain_receiver(&mut high_rx, &mut high_queue, &mut high_closed);
        drain_receiver(&mut normal_rx, &mut normal_queue, &mut normal_closed);
        drain_receiver(&mut low_rx, &mut low_queue, &mut low_closed);

        if let Some(message) = high_queue.pop_front() {
            if write.send(message).await.is_err() {
                break;
            }
            continue;
        }

        if let Some(message) = normal_queue.pop_front() {
            if write.send(message).await.is_err() {
                break;
            }
            continue;
        }

        if let Some(message) = low_queue.pop_front() {
            drain_receiver(&mut high_rx, &mut high_queue, &mut high_closed);
            drain_receiver(&mut normal_rx, &mut normal_queue, &mut normal_closed);

            if let Some(high_priority) = high_queue.pop_front() {
                low_queue.push_front(message);
                if write.send(high_priority).await.is_err() {
                    break;
                }
                continue;
            }

            if let Some(normal_priority) = normal_queue.pop_front() {
                low_queue.push_front(message);
                if write.send(normal_priority).await.is_err() {
                    break;
                }
                continue;
            }

            if write.send(message).await.is_err() {
                break;
            }
            TimeoutFuture::new(LOW_PRIORITY_DELAY_MS).await;
            continue;
        }

        if high_closed && normal_closed && low_closed {
            break;
        }

        futures::select_biased! {
            next = high_rx.next().fuse() => match next {
                Some(message) => high_queue.push_back(message),
                None => high_closed = true,
            },
            next = normal_rx.next().fuse() => match next {
                Some(message) => normal_queue.push_back(message),
                None => normal_closed = true,
            },
            next = low_rx.next().fuse() => match next {
                Some(message) => low_queue.push_back(message),
                None => low_closed = true,
            },
        }
    }
}

fn drain_receiver(
    receiver: &mut futures::channel::mpsc::Receiver<Message>,
    queue: &mut VecDeque<Message>,
    closed: &mut bool,
) {
    if *closed {
        return;
    }

    loop {
        match receiver.try_next() {
            Ok(Some(message)) => queue.push_back(message),
            Ok(None) => {
                *closed = true;
                break;
            }
            Err(_) => break,
        }
    }
}

fn init_sync(tx: &WsSender) -> Rc<RefCell<Vec<(String, u64)>>> {
    let _ = tx.try_send_event(ClientEvent::SyncRequest);
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
                let _ = tx.try_send_event(req);
            }
        }
    });
}

fn start_ping_timer(tx: WsSender) {
    spawn_local(async move {
        loop {
            // Пинг каждые 10 минут (600000 мс)
            TimeoutFuture::new(600_000).await;

            if tx
                .try_send_event_with_priority(ClientEvent::Ping, OutboundPriority::High)
                .is_err()
            {
                log!("⚠️ Failed to send ping - connection may be closed");
                break;
            }
        }
    });
}

fn start_cursor_cleanup_timer(set_cursors: WriteSignal<HashMap<String, CursorSignals>>) {
    spawn_local(async move {
        loop {
            // Проверяем каждую секунду для более плавной анимации
            TimeoutFuture::new(1_000).await;

            let now = js_sys::Date::now();
            let inactivity_threshold = 5_000.0; // 5 секунд

            set_cursors.update(|cursors| {
                for (_, cursor) in cursors.iter() {
                    let last_activity = cursor.last_activity.get();
                    if now - last_activity > inactivity_threshold {
                        // Скрываем неактивный курсор
                        cursor.set_visible.set(false);
                    }
                }
            });
        }
    });
}

async fn process_messages(
    mut read: futures::stream::SplitStream<GlooWebSocket>,
    context: MessageProcessingContext,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    let handler_context = context.handler_context();
                    handlers::handle_event(event, &handler_context);
                } else {
                    log!("Failed to parse event: {}", text);
                }
            }
            Err(e) => log!("WebSocket error: {:?}", e),
            _ => {}
        }
    }
}
