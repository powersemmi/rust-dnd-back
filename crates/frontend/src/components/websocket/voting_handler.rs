use crate::components::voting::VotingState;
use leptos::prelude::*;
use log::debug;
use shared::events::{
    voting::{PresenceRequestPayload, VotingResultPayload},
    ClientEvent, PresenceResponsePayload, VotingCastPayload,
};
use super::WsSender;
use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::rc::Rc;

pub struct VotingManager {
    active_voting_id: Rc<RefCell<Option<String>>>,
    presence_requests: Rc<RefCell<HashMap<String, HashSet<String>>>>, // request_id -> users
    my_votes: Rc<RefCell<HashMap<String, Vec<String>>>>, // voting_id -> selected options
}

impl VotingManager {
    pub fn new() -> Self {
        Self {
            active_voting_id: Rc::new(RefCell::new(None)),
            presence_requests: Rc::new(RefCell::new(HashMap::new())),
            my_votes: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn handle_voting_start(
        &self,
        payload: shared::events::VotingStartPayload,
        voting_state: RwSignal<Option<VotingState>>,
        tx: &WsSender,
        my_username: &str,
    ) {
        debug!("Voting started: {}", payload.question);
        *self.active_voting_id.borrow_mut() = Some(payload.voting_id.clone());

        // Отправляем presence request чтобы узнать кто онлайн
        let request_id = format!("presence_{}", payload.voting_id);
        let presence_req = PresenceRequestPayload {
            request_id: request_id.clone(),
            requester: my_username.to_string(),
        };

        self.presence_requests.borrow_mut().insert(request_id.clone(), HashSet::new());

        let event = ClientEvent::PresenceRequest(presence_req);
        if let Ok(json) = serde_json::to_string(&event) {
            let _ = tx.clone().try_send(gloo_net::websocket::Message::Text(json));
        }

        // Также отвечаем сами
        let response = PresenceResponsePayload {
            request_id: request_id.clone(),
            user: my_username.to_string(),
        };
        let event = ClientEvent::PresenceResponse(response);
        if let Ok(json) = serde_json::to_string(&event) {
            let _ = tx.clone().try_send(gloo_net::websocket::Message::Text(json));
        }

        voting_state.set(Some(VotingState::Active(payload)));
    }

    pub fn handle_presence_request(
        &self,
        payload: PresenceRequestPayload,
        tx: &WsSender,
        my_username: &str,
    ) {
        debug!("Received presence request from {}", payload.requester);

        let response = PresenceResponsePayload {
            request_id: payload.request_id,
            user: my_username.to_string(),
        };

        let event = ClientEvent::PresenceResponse(response);
        if let Ok(json) = serde_json::to_string(&event) {
            let _ = tx.clone().try_send(gloo_net::websocket::Message::Text(json));
        }
    }

    pub fn handle_presence_response(
        &self,
        payload: PresenceResponsePayload,
    ) {
        debug!("Received presence response from {}", payload.user);
        if let Some(users) = self.presence_requests.borrow_mut().get_mut(&payload.request_id) {
            users.insert(payload.user);
        }
    }

    pub fn handle_presence_announce(
        &self,
        _payload: shared::events::PresenceAnnouncePayload,
    ) {
        // Можно использовать для дополнительной логики если нужно
        debug!("Received presence announce");
    }

    pub fn handle_voting_cast(
        &self,
        payload: VotingCastPayload,
        my_username: &str,
    ) {
        debug!("Vote cast by {}", payload.user);

        // Если это наш голос, запоминаем
        if payload.user == my_username {
            self.my_votes.borrow_mut().insert(
                payload.voting_id.clone(),
                payload.selected_option_ids.clone(),
            );
        }
    }

    pub fn handle_voting_result(
        &self,
        payload: VotingResultPayload,
        voting_state: RwSignal<Option<VotingState>>,
    ) {
        debug!("Voting results received");

        if let Some(VotingState::Active(voting)) = voting_state.get() {
            if voting.voting_id == payload.voting_id {
                voting_state.set(Some(VotingState::Results {
                    voting,
                    results: payload.results,
                    total_participants: payload.total_participants,
                    total_voted: payload.total_voted,
                }));
            }
        }
    }

    pub fn handle_voting_end(
        &self,
        payload: shared::events::VotingEndPayload,
        voting_state: RwSignal<Option<VotingState>>,
    ) {
        debug!("Voting ended: {}", payload.voting_id);

        if let Some(ref voting_id) = *self.active_voting_id.borrow() {
            if *voting_id == payload.voting_id {
                *self.active_voting_id.borrow_mut() = None;
                // Не закрываем окно, чтобы пользователь мог видеть результаты
            }
        }
    }

    pub fn cast_vote(
        &self,
        selected_options: Vec<String>,
        voting_state: RwSignal<Option<VotingState>>,
        tx: &WsSender,
        my_username: &str,
    ) {
        if let Some(VotingState::Active(voting)) = voting_state.get() {
            let cast_payload = VotingCastPayload {
                voting_id: voting.voting_id.clone(),
                user: my_username.to_string(),
                selected_option_ids: selected_options,
            };

            let event = ClientEvent::VotingCast(cast_payload);
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = tx.clone().try_send(gloo_net::websocket::Message::Text(json));
            }
        }
    }
}
