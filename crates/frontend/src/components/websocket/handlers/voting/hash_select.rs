use crate::components::websocket::{WsSender, storage, types::SyncConflict};
use gloo_net::websocket::Message;
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::{
    ChatMessagePayload, ClientEvent, RoomState, SyncVersionPayload, VotingResultPayload,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(clippy::too_many_arguments)]
pub fn handle_hash_selection_voting_result(
    payload: VotingResultPayload,
    collected_snapshots: &Rc<RefCell<Vec<(String, RoomState)>>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    room_state: &Rc<RefCell<RoomState>>,
    room_name: &str,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
    tx: &WsSender,
) {
    log!("🔍 This is a hash selection voting: {}", payload.voting_id);

    // Находим вариант с максимальным количеством голосов
    if let Some(winner) = payload.results.iter().max_by_key(|r| r.count) {
        log!(
            "🏆 Winner option: {} with {} votes",
            winner.option_id,
            winner.count
        );

        // Извлекаем hash из stored snapshots по индексу
        let collected_snapshots_clone = collected_snapshots.clone();
        let snapshots = collected_snapshots_clone.borrow().clone();

        if !snapshots.is_empty() {
            // Пересчитываем hash_counts из снапшотов
            let mut hash_counts: HashMap<String, Vec<RoomState>> = HashMap::new();
            for (_username, state) in &snapshots {
                hash_counts
                    .entry(state.current_hash.clone())
                    .or_insert_with(Vec::new)
                    .push(state.clone());
            }

            // Извлекаем индекс из option_id (формат: "hash_N")
            if let Some(idx_str) = winner.option_id.strip_prefix("hash_") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    // Получаем выбранный hash по индексу
                    if let Some((chosen_hash, chosen_states)) = hash_counts.iter().nth(idx) {
                        let chosen_state = &chosen_states[0];

                        log!(
                            "✅ Applying chosen snapshot: v{} with hash {}...",
                            chosen_state.version,
                            &chosen_hash[..8]
                        );

                        *local_version.borrow_mut() = chosen_state.version;
                        *last_synced_version.borrow_mut() = chosen_state.version;
                        *room_state.borrow_mut() = chosen_state.clone();

                        messages_signal.set(chosen_state.chat_history.clone());
                        voting_results.set(chosen_state.voting_results.clone());
                        storage::save_state(room_name, chosen_state);

                        conflict_signal.set(None);

                        // Отправляем SyncVersionAnnounce
                        let announce = ClientEvent::SyncVersionAnnounce(SyncVersionPayload {
                            username: String::new(),
                            version: chosen_state.version,
                            state_hash: chosen_hash.clone(),
                            recent_hashes: vec![],
                        });
                        if let Ok(json) = serde_json::to_string(&announce) {
                            let _ = tx.clone().try_send(Message::Text(json));
                            log!("📢 Sent SyncVersionAnnounce after hash selection");
                        }

                        // Очищаем collected snapshots
                        collected_snapshots.borrow_mut().clear();
                    }
                }
            }
        }
    }
}
