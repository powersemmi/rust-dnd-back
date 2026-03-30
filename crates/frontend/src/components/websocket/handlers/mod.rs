mod chat;
mod file;
mod mouse;
mod presence;
mod scene;
mod sync;
pub mod sync_discard;
mod voting;

use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{FileTransferState, WsSender, types::*};
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, ClientEvent, RoomState, Scene, VotingResultPayload};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct HandlerContext<'a> {
    pub tx: &'a WsSender,
    pub file_transfer: &'a FileTransferState,
    pub room_state: &'a Rc<RefCell<RoomState>>,
    pub local_version: &'a Rc<RefCell<u64>>,
    pub last_synced_version: &'a Rc<RefCell<u64>>,
    pub sync_candidates: &'a Rc<RefCell<Vec<(String, u64)>>>,
    pub my_username: &'a str,
    pub room_name: &'a str,
    pub set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    pub messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    pub state_events: RwSignal<Vec<StateEvent>>,
    pub scenes_signal: RwSignal<Vec<Scene>>,
    pub active_scene_id_signal: RwSignal<Option<String>>,
    pub conflict_signal: RwSignal<Option<SyncConflict>>,
    pub votings: RwSignal<HashMap<String, VotingState>>,
    pub voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    pub has_statistics_notification: RwSignal<bool>,
    pub notification_count: RwSignal<u32>,
    pub has_chat_notification: RwSignal<bool>,
    pub chat_notification_count: RwSignal<u32>,
    pub expected_snapshot_from: &'a Rc<RefCell<Option<String>>>,
    pub collected_snapshots: &'a Rc<RefCell<Vec<(String, RoomState)>>>,
    pub is_collecting_snapshots: &'a Rc<RefCell<bool>>,
    pub collected_announces: &'a Rc<RefCell<Vec<shared::events::SyncVersionPayload>>>,
    pub is_collecting_announces: &'a Rc<RefCell<bool>>,
}

pub fn handle_event(event: ClientEvent, ctx: &HandlerContext<'_>) {
    match event {
        ClientEvent::ChatMessage(msg) => chat::handle_chat_message(msg, ctx),
        ClientEvent::FileAnnounce(payload) => {
            file::handle_file_announce(payload, ctx.file_transfer, ctx.my_username, ctx.tx)
        }
        ClientEvent::FileRequest(payload) => file::handle_file_request(
            payload,
            ctx.file_transfer,
            ctx.room_name,
            ctx.my_username,
            ctx.tx,
        ),
        ClientEvent::FileChunk(payload) => file::handle_file_chunk(
            payload,
            ctx.file_transfer,
            ctx.room_name,
            ctx.my_username,
            ctx.tx,
        ),
        ClientEvent::FileAbort(payload) => {
            file::handle_file_abort(payload, ctx.file_transfer, ctx.my_username)
        }
        ClientEvent::SceneCreate(payload) => scene::handle_scene_create(payload, ctx),
        ClientEvent::SceneUpdate(payload) => scene::handle_scene_update(payload, ctx),
        ClientEvent::SceneDelete(payload) => scene::handle_scene_delete(payload, ctx),
        ClientEvent::SceneActivate(payload) => scene::handle_scene_activate(payload, ctx),
        ClientEvent::MouseClickPayload(mouse_event) => {
            mouse::handle_mouse_event(mouse_event, ctx.my_username, ctx.set_cursors)
        }
        ClientEvent::SyncRequest => {
            sync::handle_sync_request(ctx.tx, ctx.room_state, ctx.local_version, ctx.my_username);
            ctx.file_transfer.reannounce_scene_files(
                ctx.room_name.to_string(),
                ctx.my_username.to_string(),
                Some(ctx.tx.clone()),
                &ctx.room_state.borrow(),
            );
        }
        ClientEvent::SyncVersionAnnounce(payload) => sync::handle_sync_announce(payload, ctx),
        ClientEvent::SyncSnapshotRequest(payload) => sync::handle_snapshot_request(
            payload,
            ctx.tx,
            ctx.room_state,
            ctx.local_version,
            ctx.my_username,
            ctx.state_events,
        ),
        ClientEvent::SyncSnapshot(payload) => sync::handle_snapshot(payload, ctx),
        ClientEvent::VotingStart(payload) => voting::handle_voting_start(payload, ctx),
        ClientEvent::VotingCast(payload) => {
            voting::handle_voting_cast(payload, ctx.votings, ctx.local_version, ctx.state_events)
        }
        ClientEvent::VotingResult(payload) => voting::handle_voting_result(payload, ctx),
        ClientEvent::VotingEnd(payload) => {
            voting::handle_voting_end(payload, ctx.votings, ctx.local_version, ctx.state_events)
        }
        ClientEvent::PresenceRequest(payload) => {
            presence::handle_presence_request(payload, ctx.tx, ctx.my_username)
        }
        ClientEvent::PresenceResponse(payload) => presence::handle_presence_response(
            payload,
            ctx.votings,
            ctx.local_version,
            ctx.state_events,
        ),
        ClientEvent::PresenceAnnounce(payload) => presence::handle_presence_announce(
            payload,
            ctx.votings,
            ctx.local_version,
            ctx.state_events,
        ),
        _ => {}
    }
}
