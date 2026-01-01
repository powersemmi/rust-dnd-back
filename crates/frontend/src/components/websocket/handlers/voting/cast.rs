use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::utils;
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::VotingCastPayload;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn handle_voting_cast(
    payload: VotingCastPayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    log!(
        "Vote cast by {}: {:?}",
        payload.user,
        payload.selected_option_ids
    );
    votings.update(|map| {
        if let Some(VotingState::Active { votes, .. }) = map.get_mut(&payload.voting_id) {
            votes.insert(payload.user.clone(), payload.selected_option_ids.clone());
        }
    });

    utils::log_event(
        state_events,
        *local_version.borrow(),
        "VOTING_CAST",
        &format!("{} voted in {}", payload.user, payload.voting_id),
    );
}
