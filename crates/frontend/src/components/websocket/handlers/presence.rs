use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{WsSender, utils};
use gloo_net::websocket::Message;
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::{
    ClientEvent, PresenceAnnouncePayload, PresenceRequestPayload, PresenceResponsePayload,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn handle_presence_request(payload: PresenceRequestPayload, tx: &WsSender, my_username: &str) {
    log!("Presence request from: {}", payload.requester);
    let response = ClientEvent::PresenceResponse(PresenceResponsePayload {
        request_id: payload.request_id,
        user: my_username.to_string(),
    });
    if let Ok(json) = serde_json::to_string(&response) {
        let _ = tx.clone().try_send(Message::Text(json));
    }
}

pub fn handle_presence_response(
    payload: PresenceResponsePayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    log!("Presence response from: {}", payload.user);

    // Извлекаем voting_id из request_id (формат: "voting_{voting_id}")
    if let Some(voting_id) = payload.request_id.strip_prefix("voting_") {
        votings.update(|map| {
            if let Some(VotingState::Active { participants, .. }) = map.get_mut(voting_id) {
                if !participants.contains(&payload.user) {
                    participants.push(payload.user.clone());
                }
            }
        });

        utils::log_event(
            state_events,
            *local_version.borrow(),
            "PRESENCE_RESPONSE",
            &format!("{} joined voting {}", payload.user, voting_id),
        );
    }
}

pub fn handle_presence_announce(
    payload: PresenceAnnouncePayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    log!("Presence announce: {:?}", payload.online_users);

    // Извлекаем voting_id из request_id
    if let Some(voting_id) = payload.request_id.strip_prefix("voting_") {
        votings.update(|map| {
            if let Some(VotingState::Active { participants, .. }) = map.get_mut(voting_id) {
                *participants = payload.online_users.clone();
            }
        });

        utils::log_event(
            state_events,
            *local_version.borrow(),
            "PRESENCE_ANNOUNCE",
            &format!(
                "Voting {} participants announced: {}",
                voting_id,
                payload.online_users.join(", ")
            ),
        );
    }
}
