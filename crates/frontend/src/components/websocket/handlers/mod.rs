mod chat;
mod mouse;
mod presence;
mod sync;
pub mod sync_discard;
mod voting;

use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{WsSender, types::*};
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, ClientEvent, RoomState, VotingResultPayload};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(clippy::too_many_arguments)]
pub fn handle_event(
    event: ClientEvent,
    tx: &WsSender,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    sync_candidates: &Rc<RefCell<Vec<(String, u64)>>>,
    my_username: &str,
    room_name: &str,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    conflict_signal: RwSignal<Option<SyncConflict>>,
    votings: RwSignal<HashMap<String, VotingState>>,
    voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    has_statistics_notification: RwSignal<bool>,
    notification_count: RwSignal<u32>,
    has_chat_notification: RwSignal<bool>,
    chat_notification_count: RwSignal<u32>,
    expected_snapshot_from: &Rc<RefCell<Option<String>>>,
    collected_snapshots: &Rc<RefCell<Vec<(String, RoomState)>>>,
    is_collecting_snapshots: &Rc<RefCell<bool>>,
    collected_announces: &Rc<RefCell<Vec<shared::events::SyncVersionPayload>>>,
    is_collecting_announces: &Rc<RefCell<bool>>,
) {
    match event {
        ClientEvent::ChatMessage(msg) => chat::handle_chat_message(
            msg,
            room_state,
            local_version,
            last_synced_version,
            my_username,
            room_name,
            messages_signal,
            state_events,
            has_chat_notification,
            chat_notification_count,
        ),
        ClientEvent::MouseClickPayload(mouse_event) => {
            mouse::handle_mouse_event(mouse_event, my_username, set_cursors)
        }
        ClientEvent::SyncRequest => {
            sync::handle_sync_request(tx, room_state, local_version, my_username)
        }
        ClientEvent::SyncVersionAnnounce(payload) => sync::handle_sync_announce(
            payload,
            sync_candidates,
            room_state,
            local_version,
            state_events,
            conflict_signal,
            collected_announces,
            is_collecting_announces,
        ),
        ClientEvent::SyncSnapshotRequest(payload) => sync::handle_snapshot_request(
            payload,
            tx,
            room_state,
            local_version,
            my_username,
            state_events,
        ),
        ClientEvent::SyncSnapshot(payload) => sync::handle_snapshot(
            payload,
            room_state,
            local_version,
            last_synced_version,
            room_name,
            messages_signal,
            state_events,
            conflict_signal,
            voting_results,
            expected_snapshot_from,
            tx,
            collected_snapshots,
            is_collecting_snapshots,
            my_username,
        ),
        ClientEvent::VotingStart(payload) => voting::handle_voting_start(
            payload,
            votings,
            tx,
            my_username,
            local_version,
            state_events,
            has_statistics_notification,
            notification_count,
        ),
        ClientEvent::VotingCast(payload) => {
            voting::handle_voting_cast(payload, votings, local_version, state_events)
        }
        ClientEvent::VotingResult(payload) => voting::handle_voting_result(
            payload,
            votings,
            voting_results,
            room_state,
            local_version,
            last_synced_version,
            room_name,
            state_events,
            tx,
            my_username,
            expected_snapshot_from,
            collected_snapshots,
            is_collecting_snapshots,
            messages_signal,
            conflict_signal,
        ),
        ClientEvent::VotingEnd(payload) => {
            voting::handle_voting_end(payload, votings, local_version, state_events)
        }
        ClientEvent::PresenceRequest(payload) => {
            presence::handle_presence_request(payload, tx, my_username)
        }
        ClientEvent::PresenceResponse(payload) => {
            presence::handle_presence_response(payload, votings, local_version, state_events)
        }
        ClientEvent::PresenceAnnounce(payload) => {
            presence::handle_presence_announce(payload, votings, local_version, state_events)
        }
        _ => {}
    }
}
