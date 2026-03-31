use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{
    FileTransferState, RoomCryptoState, SnapshotCodec, storage, types::CursorSignals,
};
use crate::config;
use futures::{FutureExt, SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket as GlooWebSocket};
use gloo_timers::future::TimeoutFuture;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use rand::seq::IndexedRandom;
use shared::events::{
    ChatMessagePayload, ClientEvent, EncryptedPayloadKind, NotePayload, RoomState, Scene,
    SyncSnapshotRequestPayload, VotingResultPayload,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    crypto: Arc<Mutex<RoomCryptoState>>,
}

impl WsSender {
    fn new(
        high: futures::channel::mpsc::Sender<Message>,
        normal: futures::channel::mpsc::Sender<Message>,
        low: futures::channel::mpsc::Sender<Message>,
        crypto: Arc<Mutex<RoomCryptoState>>,
    ) -> Self {
        Self {
            high,
            normal,
            low,
            crypto,
        }
    }

    pub fn crypto_state(&self) -> Arc<Mutex<RoomCryptoState>> {
        self.crypto.clone()
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
        if RoomCryptoState::should_encrypt_event(&event) {
            let outbound = self
                .crypto
                .lock()
                .map_err(|_| "failed to lock room crypto state".to_string())?
                .prepare_encrypted_events(&event)?;
            for item in outbound {
                let item_priority = match item {
                    ClientEvent::CryptoPayload(_) => priority,
                    _ => OutboundPriority::High,
                };
                self.send_plain_event_with_priority(item, item_priority)?;
            }
            return Ok(());
        }

        self.send_plain_event_with_priority(event, priority)
    }

    fn try_send_text_with_priority(
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

    fn send_plain_event_with_priority(
        &self,
        event: ClientEvent,
        priority: OutboundPriority,
    ) -> Result<(), String> {
        let json = serde_json::to_string(&event)
            .map_err(|error| format!("failed to serialize websocket event: {error}"))?;
        self.try_send_text_with_priority(json, priority)
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
            ClientEvent::CryptoPayload(payload) => match payload.kind {
                EncryptedPayloadKind::FileChunk => OutboundPriority::Low,
                EncryptedPayloadKind::Chat
                | EncryptedPayloadKind::Note
                | EncryptedPayloadKind::Sync => OutboundPriority::High,
                EncryptedPayloadKind::FileControl => OutboundPriority::Normal,
            },
            ClientEvent::RoomState(_)
            | ClientEvent::NoteUpsert(_)
            | ClientEvent::NoteDelete(_)
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
            | ClientEvent::PresenceAnnounce(_)
            | ClientEvent::CryptoKeyAnnounce(_)
            | ClientEvent::CryptoKeyWrap(_)
            | ClientEvent::BoardPointer(_)
            | ClientEvent::AttentionPing(_)
            | ClientEvent::DirectMessage(_) => OutboundPriority::Normal,
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
    pub public_notes_signal: RwSignal<Vec<NotePayload>>,
    pub direct_notes_signal: RwSignal<Vec<NotePayload>>,
    pub direct_note_recipients_signal: RwSignal<Vec<String>>,
    pub direct_note_recipients_cache_updated_at_ms_signal: RwSignal<Option<f64>>,
    pub direct_note_recipients_request_id_signal: RwSignal<Option<String>>,
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
    pub board_pointers: RwSignal<std::collections::HashSet<String>>,
    pub attention_pings: RwSignal<Vec<shared::events::AttentionPingPayload>>,
    pub direct_messages: RwSignal<Vec<shared::events::DirectMessagePayload>>,
}

struct MessageProcessingContext {
    tx: WsSender,
    snapshot_codec: SnapshotCodec,
    file_transfer: FileTransferState,
    room_state: Rc<RefCell<RoomState>>,
    local_version: Rc<RefCell<u64>>,
    last_synced_version: Rc<RefCell<u64>>,
    sync_candidates: Rc<RefCell<Vec<(String, u64)>>>,
    my_username: String,
    room_name: String,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    public_notes_signal: RwSignal<Vec<NotePayload>>,
    direct_notes_signal: RwSignal<Vec<NotePayload>>,
    direct_note_recipients_signal: RwSignal<Vec<String>>,
    direct_note_recipients_cache_updated_at_ms_signal: RwSignal<Option<f64>>,
    direct_note_recipients_request_id_signal: RwSignal<Option<String>>,
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
    board_pointers: RwSignal<std::collections::HashSet<String>>,
    attention_pings: RwSignal<Vec<shared::events::AttentionPingPayload>>,
    direct_messages: RwSignal<Vec<shared::events::DirectMessagePayload>>,
}

impl MessageProcessingContext {
    fn handler_context(&self) -> handlers::HandlerContext<'_> {
        handlers::HandlerContext {
            tx: &self.tx,
            snapshot_codec: &self.snapshot_codec,
            file_transfer: &self.file_transfer,
            room_state: &self.room_state,
            local_version: &self.local_version,
            last_synced_version: &self.last_synced_version,
            sync_candidates: &self.sync_candidates,
            my_username: &self.my_username,
            room_name: &self.room_name,
            set_cursors: self.set_cursors,
            messages_signal: self.messages_signal,
            public_notes_signal: self.public_notes_signal,
            direct_notes_signal: self.direct_notes_signal,
            direct_note_recipients_signal: self.direct_note_recipients_signal,
            direct_note_recipients_cache_updated_at_ms_signal: self
                .direct_note_recipients_cache_updated_at_ms_signal,
            direct_note_recipients_request_id_signal: self.direct_note_recipients_request_id_signal,
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
            board_pointers: self.board_pointers,
            attention_pings: self.attention_pings,
            direct_messages: self.direct_messages,
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
        public_notes_signal,
        direct_notes_signal,
        direct_note_recipients_signal,
        direct_note_recipients_cache_updated_at_ms_signal,
        direct_note_recipients_request_id_signal,
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
        board_pointers,
        attention_pings,
        direct_messages,
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
    let my_username_for_crypto = my_username.clone();
    let snapshot_codec = SnapshotCodec::new();

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
                public_notes_signal.set(data.state.public_notes.clone());
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
                let crypto_state = Arc::new(Mutex::new(RoomCryptoState::new(
                    &room_name,
                    &my_username_for_crypto,
                )));
                let tx = WsSender::new(high_tx, normal_tx, low_tx, crypto_state);
                set_ws_sender.set(Some(tx.clone()));

                // Устанавливаем callback для разрешения конфликта
                let tx_for_callback = tx.clone();
                let collected_announces_cb = collected_announces.clone();
                let is_collecting_announces_cb = is_collecting_announces.clone();
                let expected_snapshot_from_cb = expected_snapshot_from.clone();
                let room_state_for_callback = room_state.clone();
                let local_version_for_callback = local_version.clone();
                let last_synced_version_for_callback = last_synced_version.clone();
                let messages_signal_for_callback = messages_signal;
                let public_notes_signal_for_callback = public_notes_signal;
                let scenes_signal_for_callback = scenes_signal;
                let active_scene_id_signal_for_callback = active_scene_id_signal;
                let voting_results_for_callback = voting_results;
                let conflict_signal_for_callback = conflict_signal;
                let state_events_for_callback = state_events;
                let room_name_for_callback = room_name.clone();
                let file_transfer_for_callback = file_transfer.clone();

                conflict_resolution_handle.set_callback(move || {
                    use crate::components::websocket::handlers::sync_discard;
                    *room_state_for_callback.borrow_mut() = RoomState::default();
                    *local_version_for_callback.borrow_mut() = 0;
                    *last_synced_version_for_callback.borrow_mut() = 0;
                    messages_signal_for_callback.set(Vec::new());
                    public_notes_signal_for_callback.set(Vec::new());
                    scenes_signal_for_callback.set(Vec::new());
                    active_scene_id_signal_for_callback.set(None);
                    voting_results_for_callback.set(HashMap::new());
                    conflict_signal_for_callback.set(None);
                    file_transfer_for_callback.reset();
                    storage::save_state_in_background(
                        &room_name_for_callback,
                        &RoomState::default(),
                    );
                    super::utils::log_event(
                        state_events_for_callback,
                        0,
                        "LOCAL_STATE_RESET",
                        "Cleared runtime room state before resync",
                    );
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

                let announce_event = tx
                    .crypto_state()
                    .lock()
                    .map(|state| state.key_announce_event())
                    .ok();
                if let Some(announce_event) = announce_event {
                    let _ = tx.try_send_event(announce_event);
                }

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
                        snapshot_codec: snapshot_codec.clone(),
                        file_transfer,
                        room_state,
                        local_version,
                        last_synced_version,
                        sync_candidates,
                        my_username: my_username_clone,
                        room_name: room_name_clone,
                        set_cursors,
                        messages_signal,
                        public_notes_signal,
                        direct_notes_signal,
                        direct_note_recipients_signal,
                        direct_note_recipients_cache_updated_at_ms_signal,
                        direct_note_recipients_request_id_signal,
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
                        board_pointers,
                        attention_pings,
                        direct_messages,
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
    let crypto_state = context.tx.crypto_state();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    match event {
                        ClientEvent::CryptoKeyAnnounce(payload) => {
                            let outbound = crypto_state
                                .lock()
                                .map_err(|_| "failed to lock room crypto state".to_string())
                                .and_then(|mut state| state.handle_key_announce(&payload));
                            match outbound {
                                Ok(events) => {
                                    for event in events {
                                        // Send key-wrap responses on High priority so they
                                        // arrive at the newcomer before any encrypted snapshot
                                        // that is also queued on High priority.  The outbound
                                        // scheduler drains High before Normal, so without this
                                        // the snapshot could win the race and land before the
                                        // wrap even though the wrap was enqueued first.
                                        let _ = context.tx.try_send_event_with_priority(
                                            event,
                                            OutboundPriority::High,
                                        );
                                    }
                                }
                                Err(error) => {
                                    log!("Failed to process CRYPTO_KEY_ANNOUNCE: {}", error)
                                }
                            }
                        }
                        ClientEvent::CryptoKeyWrap(payload) => {
                            let decrypted = crypto_state
                                .lock()
                                .map_err(|_| "failed to lock room crypto state".to_string())
                                .and_then(|mut state| state.handle_key_wrap(&payload));
                            match decrypted {
                                Ok(events) => {
                                    for event in events {
                                        let handler_context = context.handler_context();
                                        handlers::handle_event(event, &handler_context);
                                    }
                                }
                                Err(error) => log!("Failed to process CRYPTO_KEY_WRAP: {}", error),
                            }
                        }
                        ClientEvent::CryptoPayload(payload) => {
                            let decrypted = crypto_state
                                .lock()
                                .map_err(|_| "failed to lock room crypto state".to_string())
                                .and_then(|mut state| state.decrypt_payload(&payload));
                            match decrypted {
                                Ok(Some(event)) => {
                                    let handler_context = context.handler_context();
                                    handlers::handle_event(event, &handler_context);
                                }
                                Ok(None) => {}
                                Err(error) => log!("Failed to decrypt CRYPTO_PAYLOAD: {}", error),
                            }
                        }
                        other => {
                            if RoomCryptoState::should_encrypt_event(&other) {
                                log!("Blocked legacy plaintext event: {:?}", other);
                                continue;
                            }
                            let handler_context = context.handler_context();
                            handlers::handle_event(other, &handler_context);
                        }
                    }
                } else {
                    log!("Failed to parse event: {}", text);
                }
            }
            Err(e) => log!("WebSocket error: {:?}", e),
            _ => {}
        }
    }
}
