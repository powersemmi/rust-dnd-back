use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{WsSender, storage, types::SyncConflict, utils};
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, RoomState, VotingResultPayload};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{conflict, discard, hash_select};

#[allow(clippy::too_many_arguments)]
pub fn handle_voting_result(
    payload: VotingResultPayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    room_name: &str,
    state_events: RwSignal<Vec<StateEvent>>,
    tx: &WsSender,
    my_username: &str,
    expected_snapshot_from: &Rc<RefCell<Option<String>>>,
    collected_snapshots: &Rc<RefCell<Vec<(String, RoomState)>>>,
    is_collecting_snapshots: &Rc<RefCell<bool>>,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
) {
    log!("🎯 VOTING RESULT RECEIVED for: {}", payload.voting_id);
    log!("Voting results received for: {}", payload.voting_id);

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

    utils::log_event(
        state_events,
        current_ver,
        "VOTING_RESULT",
        &format!("Voting {} completed", payload.voting_id),
    );

    // Проверяем, является ли это голосованием для разрешения конфликта
    if payload.voting_id.starts_with("conflict_vote_") {
        conflict::handle_conflict_voting_result(
            payload.clone(),
            votings,
            tx,
            my_username,
            expected_snapshot_from,
        );
    }

    // Проверяем, является ли это голосованием для сбора snapshots при discard
    if payload.voting_id.starts_with("discard_collect_") {
        discard::handle_discard_collection_voting_result(
            payload.clone(),
            tx,
            collected_snapshots,
            is_collecting_snapshots,
            room_state,
            local_version,
            last_synced_version,
            room_name,
            messages_signal,
            voting_results,
            conflict_signal,
        );
    }

    // Проверяем, является ли это голосованием для выбора хеша
    if payload.voting_id.starts_with("hash_select_") {
        hash_select::handle_hash_selection_voting_result(
            payload,
            collected_snapshots,
            local_version,
            last_synced_version,
            room_state,
            room_name,
            messages_signal,
            voting_results,
            conflict_signal,
            tx,
        );
    }
}
