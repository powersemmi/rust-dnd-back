use crate::components::websocket::storage;
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::{ClientEvent, RoomState, SyncVersionPayload, VotingResultPayload};
use std::collections::HashMap;

use super::super::HandlerContext;

pub fn handle_hash_selection_voting_result(payload: VotingResultPayload, ctx: &HandlerContext<'_>) {
    log!("🔍 This is a hash selection voting: {}", payload.voting_id);

    // Находим вариант с максимальным количеством голосов
    if let Some(winner) = payload.results.iter().max_by_key(|r| r.count) {
        log!(
            "🏆 Winner option: {} with {} votes",
            winner.option_id,
            winner.count
        );

        // Извлекаем hash из stored snapshots по индексу
        let collected_snapshots_clone = ctx.collected_snapshots.clone();
        let snapshots = collected_snapshots_clone.borrow().clone();

        if !snapshots.is_empty() {
            // Пересчитываем hash_counts из снапшотов
            let mut hash_counts: HashMap<String, Vec<RoomState>> = HashMap::new();
            for (_username, state) in &snapshots {
                hash_counts
                    .entry(state.current_hash.clone())
                    .or_default()
                    .push(state.clone());
            }

            // Извлекаем индекс из option_id (формат: "hash_N")
            if let Some(idx_str) = winner.option_id.strip_prefix("hash_")
                && let Ok(idx) = idx_str.parse::<usize>()
                && let Some((chosen_hash, chosen_states)) = hash_counts.iter().nth(idx)
            {
                let chosen_state = &chosen_states[0];

                log!(
                    "✅ Applying chosen snapshot: v{} with hash {}...",
                    chosen_state.version,
                    &chosen_hash[..8]
                );

                *ctx.local_version.borrow_mut() = chosen_state.version;
                *ctx.last_synced_version.borrow_mut() = chosen_state.version;
                *ctx.room_state.borrow_mut() = chosen_state.clone();

                ctx.messages_signal.set(chosen_state.chat_history.clone());
                ctx.file_transfer
                    .reconcile_chat_attachments(&chosen_state.chat_history);
                ctx.scenes_signal.set(chosen_state.scenes.clone());
                ctx.active_scene_id_signal
                    .set(chosen_state.active_scene_id.clone());
                ctx.voting_results.set(chosen_state.voting_results.clone());
                storage::save_state_in_background(ctx.room_name, chosen_state);

                ctx.conflict_signal.set(None);

                // Отправляем SyncVersionAnnounce
                let announce = ClientEvent::SyncVersionAnnounce(SyncVersionPayload {
                    username: String::new(),
                    version: chosen_state.version,
                    state_hash: chosen_hash.clone(),
                    recent_hashes: vec![],
                });
                if ctx.tx.try_send_event(announce).is_ok() {
                    log!("📢 Sent SyncVersionAnnounce after hash selection");
                }

                // Очищаем collected snapshots
                ctx.collected_snapshots.borrow_mut().clear();
            }
        }
    }
}
