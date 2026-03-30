use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{WsSender, utils};
use js_sys;
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::{
    ClientEvent, PresenceAnnouncePayload, PresenceRequestPayload, PresenceResponsePayload,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct DirectRecipientPresenceSignals {
    pub recipients: RwSignal<Vec<String>>,
    pub cache_updated_at_ms: RwSignal<Option<f64>>,
    pub request_id: RwSignal<Option<String>>,
}

pub fn handle_presence_request(payload: PresenceRequestPayload, tx: &WsSender, my_username: &str) {
    log!("Presence request from: {}", payload.requester);
    let response = ClientEvent::PresenceResponse(PresenceResponsePayload {
        request_id: payload.request_id,
        user: my_username.to_string(),
    });
    let _ = tx.try_send_event(response);
}

pub fn handle_presence_response(
    payload: PresenceResponsePayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    direct_recipient_signals: DirectRecipientPresenceSignals,
    my_username: &str,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    log!("Presence response from: {}", payload.user);

    if direct_recipient_signals
        .request_id
        .get_untracked()
        .as_deref()
        == Some(payload.request_id.as_str())
    {
        direct_recipient_signals.recipients.update(|users| {
            if !users.contains(&payload.user) {
                users.push(payload.user.clone());
                users.sort();
            }
        });
        direct_recipient_signals
            .cache_updated_at_ms
            .set(Some(js_sys::Date::now()));

        utils::log_event(
            state_events,
            *local_version.borrow(),
            "NOTES_RECIPIENT_PRESENCE",
            &format!("{} is active for {}", payload.user, my_username),
        );
        return;
    }

    // Извлекаем voting_id из request_id (формат: "voting_{voting_id}")
    if let Some(voting_id) = payload.request_id.strip_prefix("voting_") {
        votings.update(|map| {
            if let Some(VotingState::Active { participants, .. }) = map.get_mut(voting_id)
                && !participants.contains(&payload.user)
            {
                participants.push(payload.user.clone());
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
