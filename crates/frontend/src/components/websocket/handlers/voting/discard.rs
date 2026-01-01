use crate::components::websocket::{WsSender, storage, types::SyncConflict};
use gloo_net::websocket::Message;
use gloo_timers::future::TimeoutFuture;
use js_sys;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{
    ChatMessagePayload, ClientEvent, RoomState, SyncSnapshotRequestPayload, SyncVersionPayload,
    VotingResultPayload,
    voting::{VotingOption, VotingStartPayload, VotingType},
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(clippy::too_many_arguments)]
pub fn handle_discard_collection_voting_result(
    payload: VotingResultPayload,
    tx: &WsSender,
    collected_snapshots: &Rc<RefCell<Vec<(String, RoomState)>>>,
    is_collecting_snapshots: &Rc<RefCell<bool>>,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    room_name: &str,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
) {
    log!(
        "🔍 This is a discard collection voting: {}",
        payload.voting_id
    );

    // Собираем всех участников из результатов голосования (все проголосовавшие "Present")
    let participants: Vec<String> = payload
        .results
        .iter()
        .flat_map(|r| r.voters.clone().unwrap_or_default())
        .collect();

    log!(
        "👥 Collected {} participants for snapshot collection: {:?}",
        participants.len(),
        participants
    );

    // Игнорируем, если участников слишком мало
    if participants.len() < 2 {
        log!(
            "⚠️ Too few participants ({}), skipping snapshot collection",
            participants.len()
        );
        return;
    }

    // Запускаем процесс сбора snapshots от всех участников
    let mut tx_clone = tx.clone();
    let collected_snapshots_clone = collected_snapshots.clone();
    let is_collecting_snapshots_clone = is_collecting_snapshots.clone();
    let room_state_clone = room_state.clone();
    let local_version_clone = local_version.clone();
    let last_synced_version_clone = last_synced_version.clone();
    let room_name_str = room_name.to_string();

    spawn_local(async move {
        // Включаем режим сбора snapshots
        *is_collecting_snapshots_clone.borrow_mut() = true;
        collected_snapshots_clone.borrow_mut().clear();

        log!("📦 Collection mode enabled");

        // Небольшая задержка для стабилизации
        TimeoutFuture::new(500).await;

        // Отправляем broadcast запрос на snapshot
        log!("📤 Sending broadcast snapshot request");
        let req = ClientEvent::SyncSnapshotRequest(SyncSnapshotRequestPayload {
            target_username: String::new(), // Broadcast
        });
        if let Ok(json) = serde_json::to_string(&req) {
            let _ = tx_clone.clone().try_send(Message::Text(json));
        }

        // Ожидаем сбора snapshots (2 секунды)
        log!("⏳ Waiting 2 seconds to collect snapshots...");
        TimeoutFuture::new(2000).await;

        // Выключаем режим сбора
        *is_collecting_snapshots_clone.borrow_mut() = false;

        // Анализируем собранные snapshots
        let snapshots = collected_snapshots_clone.borrow().clone();
        log!("📊 Analyzing {} collected snapshots", snapshots.len());

        if snapshots.is_empty() {
            log!("⚠️ No snapshots collected!");
            return;
        }

        // Подсчитываем количество одинаковых хешей
        let mut hash_counts: HashMap<String, Vec<RoomState>> = HashMap::new();
        for (_username, state) in snapshots {
            hash_counts
                .entry(state.current_hash.clone())
                .or_insert_with(Vec::new)
                .push(state);
        }

        log!("🔍 Found {} unique hashes:", hash_counts.len());
        for (hash, states) in &hash_counts {
            log!(
                "  - {}... : {} occurrences (v{})",
                &hash[..8],
                states.len(),
                states[0].version
            );
        }

        let total_snapshots = collected_snapshots_clone.borrow().len();
        let majority_threshold = (total_snapshots + 1) / 2; // >50%

        // Ищем вариант с большинством голосов
        if let Some((majority_hash, majority_states)) = hash_counts
            .iter()
            .find(|(_, states)| states.len() > majority_threshold)
        {
            apply_majority_snapshot(
                &majority_hash,
                &majority_states[0],
                local_version_clone,
                last_synced_version_clone,
                room_state_clone,
                &room_name_str,
                messages_signal,
                voting_results,
                conflict_signal,
                &mut tx_clone,
                total_snapshots,
            );
        } else {
            create_hash_selection_voting(
                hash_counts,
                majority_threshold,
                total_snapshots,
                &mut tx_clone,
            );
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn apply_majority_snapshot(
    majority_hash: &str,
    chosen_state: &RoomState,
    local_version_clone: Rc<RefCell<u64>>,
    last_synced_version_clone: Rc<RefCell<u64>>,
    room_state_clone: Rc<RefCell<RoomState>>,
    room_name_str: &str,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
    tx_clone: &mut WsSender,
    total_snapshots: usize,
) {
    log!(
        "✅ Found majority: {}... with {} votes (>50% of {})",
        &majority_hash[..8],
        1, // placeholder, will be calculated from states
        total_snapshots
    );

    *local_version_clone.borrow_mut() = chosen_state.version;
    *last_synced_version_clone.borrow_mut() = chosen_state.version;
    *room_state_clone.borrow_mut() = chosen_state.clone();

    messages_signal.set(chosen_state.chat_history.clone());
    voting_results.set(chosen_state.voting_results.clone());
    storage::save_state(room_name_str, chosen_state);

    conflict_signal.set(None);

    log!(
        "✅ Applied majority snapshot: v{} with hash {}...",
        chosen_state.version,
        &majority_hash[..8]
    );

    // Отправляем SyncVersionAnnounce чтобы другие знали о разрешении конфликта
    let announce = ClientEvent::SyncVersionAnnounce(SyncVersionPayload {
        username: String::new(),
        version: chosen_state.version,
        state_hash: majority_hash.to_string(),
        recent_hashes: vec![],
    });
    if let Ok(json) = serde_json::to_string(&announce) {
        let _ = tx_clone.clone().try_send(Message::Text(json));
        log!("📢 Sent SyncVersionAnnounce after majority selection");
    }
}

fn create_hash_selection_voting(
    hash_counts: HashMap<String, Vec<RoomState>>,
    majority_threshold: usize,
    total_snapshots: usize,
    tx_clone: &mut WsSender,
) {
    log!(
        "⚠️ No majority found (threshold: {} of {}), creating voting...",
        majority_threshold,
        total_snapshots
    );

    // Создаём варианты для голосования
    let mut voting_options: Vec<VotingOption> = hash_counts
        .iter()
        .enumerate()
        .map(|(idx, (hash, states))| {
            let count = states.len();
            let version = states[0].version;
            let hash_short = &hash[hash.len().saturating_sub(6)..];

            VotingOption {
                id: format!("hash_{}", idx),
                text: format!("{} members - {} v{}", count, hash_short, version),
            }
        })
        .collect();

    // Сортируем по количеству участников (больше голосов сверху)
    voting_options.sort_by(|a, b| {
        let a_count: u32 = a
            .text
            .split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let b_count: u32 = b
            .text
            .split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        b_count.cmp(&a_count)
    });

    let voting_id = format!("hash_select_{}", js_sys::Date::now() as u64);

    let voting_payload = VotingStartPayload {
        voting_id,
        question: "conflict.select_version".to_string(), // i18n key
        options: voting_options,
        voting_type: VotingType::SingleChoice,
        is_anonymous: false,
        timer_seconds: Some(60), // Минута для выбора
        default_option_id: None,
        creator: "system".to_string(),
    };

    let event = ClientEvent::VotingStart(voting_payload);
    if let Ok(json) = serde_json::to_string(&event) {
        let _ = tx_clone.try_send(Message::Text(json));
        log!("🗳️ Created hash selection voting");
    }
}
