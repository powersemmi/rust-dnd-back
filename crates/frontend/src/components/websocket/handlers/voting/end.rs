use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::utils;
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::VotingEndPayload;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn handle_voting_end(
    payload: VotingEndPayload,
    _votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    log!("Voting ended: {}", payload.voting_id);
    // Не удаляем голосование, оно уже должно быть в состоянии Results после VotingResult
    // Просто логируем событие
    utils::log_event(
        state_events,
        *local_version.borrow(),
        "VOTING_END",
        &format!("Voting {} ended", payload.voting_id),
    );
}
