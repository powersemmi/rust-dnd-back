use crate::components::voting::VotingState;
use crate::components::websocket::{storage, utils};
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::VotingResultPayload;

use super::super::HandlerContext;
use super::{conflict, hash_select};

pub fn handle_voting_result(payload: VotingResultPayload, ctx: &HandlerContext<'_>) {
    log!("🎯 VOTING RESULT RECEIVED for: {}", payload.voting_id);
    log!("Voting results received for: {}", payload.voting_id);

    // Обновляем текущее состояние голосования
    ctx.votings.update(|map| {
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
    ctx.voting_results.update(|results| {
        results.insert(payload.voting_id.clone(), payload.clone());
    });

    // Сохраняем результаты в RoomState
    let current_ver = {
        let mut state = ctx.room_state.borrow_mut();
        state
            .voting_results
            .insert(payload.voting_id.clone(), payload.clone());
        state.commit_changes();
        state.version
    };

    *ctx.local_version.borrow_mut() = current_ver;
    *ctx.last_synced_version.borrow_mut() = current_ver;

    storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());

    utils::log_event(
        ctx.state_events,
        current_ver,
        "VOTING_RESULT",
        &format!("Voting {} completed", payload.voting_id),
    );

    // Проверяем, является ли это голосованием для разрешения конфликта
    if payload.voting_id.starts_with("conflict_vote_") {
        conflict::handle_conflict_voting_result(
            payload.clone(),
            ctx.votings,
            ctx.tx,
            ctx.my_username,
            ctx.expected_snapshot_from,
        );
    }

    // Проверяем, является ли это голосованием для выбора хеша
    if payload.voting_id.starts_with("hash_select_") {
        hash_select::handle_hash_selection_voting_result(payload, ctx);
    }
}
