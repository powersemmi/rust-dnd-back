pub mod chat;
pub mod file;
pub mod mouse;
pub mod params;
pub mod room;
pub mod scene;
pub mod sync;
pub mod voting;

pub use crate::events::chat::ChatMessagePayload;
pub use crate::events::file::{
    FileAbortPayload, FileAnnouncePayload, FileChunkPayload, FileRequestPayload,
};
pub use crate::events::mouse::MouseClickPayload;
pub use crate::events::params::Params;
pub use crate::events::room::RoomState;
pub use crate::events::scene::{
    FileRef, Scene, SceneActivatePayload, SceneCreatePayload, SceneDeletePayload, SceneGrid,
    SceneUpdatePayload,
};
pub use crate::events::sync::{
    SyncSnapshotPayload, SyncSnapshotRequestPayload, SyncVersionPayload,
};
pub use crate::events::voting::{
    PresenceAnnouncePayload, PresenceRequestPayload, PresenceResponsePayload, VotingCastPayload,
    VotingEndPayload, VotingResultPayload, VotingStartPayload,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[serde(tag = "type", content = "data")]
pub enum ClientEvent {
    #[serde(rename = "ROOM_STATE")]
    RoomState(RoomState),
    #[serde(rename = "MOUSE_EVENT")]
    MouseClickPayload(MouseClickPayload),
    #[serde(rename = "CHAT_MESSAGE")]
    ChatMessage(ChatMessagePayload),
    #[serde(rename = "FILE_ANNOUNCE")]
    FileAnnounce(FileAnnouncePayload),
    #[serde(rename = "FILE_REQUEST")]
    FileRequest(FileRequestPayload),
    #[serde(rename = "FILE_CHUNK")]
    FileChunk(FileChunkPayload),
    #[serde(rename = "FILE_ABORT")]
    FileAbort(FileAbortPayload),
    #[serde(rename = "SCENE_CREATE")]
    SceneCreate(SceneCreatePayload),
    #[serde(rename = "SCENE_UPDATE")]
    SceneUpdate(SceneUpdatePayload),
    #[serde(rename = "SCENE_DELETE")]
    SceneDelete(SceneDeletePayload),
    #[serde(rename = "SCENE_ACTIVATE")]
    SceneActivate(SceneActivatePayload),

    /// Sync events
    #[serde(rename = "SYNC_REQUEST")]
    SyncRequest,
    #[serde(rename = "SYNC_VERSION_ANNOUNCE")]
    SyncVersionAnnounce(SyncVersionPayload),
    #[serde(rename = "SYNC_SNAPSHOT_REQUEST")]
    SyncSnapshotRequest(SyncSnapshotRequestPayload),
    #[serde(rename = "SYNC_SNAPSHOT")]
    SyncSnapshot(SyncSnapshotPayload),

    /// Voting events
    #[serde(rename = "VOTING_START")]
    VotingStart(VotingStartPayload),
    #[serde(rename = "VOTING_CAST")]
    VotingCast(VotingCastPayload),
    #[serde(rename = "VOTING_RESULT")]
    VotingResult(VotingResultPayload),
    #[serde(rename = "VOTING_END")]
    VotingEnd(VotingEndPayload),

    /// Presence events
    #[serde(rename = "PRESENCE_REQUEST")]
    PresenceRequest(PresenceRequestPayload),
    #[serde(rename = "PRESENCE_RESPONSE")]
    PresenceResponse(PresenceResponsePayload),
    #[serde(rename = "PRESENCE_ANNOUNCE")]
    PresenceAnnounce(PresenceAnnouncePayload),

    #[serde(rename = "PING")]
    Ping,
}

#[cfg(feature = "validation")]
impl ClientEvent {
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            ClientEvent::RoomState(p) => p.validate(),
            ClientEvent::MouseClickPayload(p) => p.validate(),
            ClientEvent::ChatMessage(p) => p.validate(),
            ClientEvent::FileAnnounce(p) => p.validate(),
            ClientEvent::FileRequest(p) => p.validate(),
            ClientEvent::FileChunk(p) => p.validate(),
            ClientEvent::FileAbort(p) => p.validate(),
            ClientEvent::SceneCreate(p) => p.validate(),
            ClientEvent::SceneUpdate(p) => p.validate(),
            ClientEvent::SceneDelete(p) => p.validate(),
            ClientEvent::SceneActivate(p) => p.validate(),
            ClientEvent::SyncVersionAnnounce(p) => p.validate(),
            ClientEvent::SyncSnapshotRequest(p) => p.validate(),
            ClientEvent::SyncSnapshot(p) => p.validate(),
            ClientEvent::VotingStart(p) => p.validate(),
            ClientEvent::VotingCast(p) => p.validate(),
            ClientEvent::VotingResult(p) => p.validate(),
            ClientEvent::VotingEnd(p) => p.validate(),
            ClientEvent::PresenceRequest(p) => p.validate(),
            ClientEvent::PresenceResponse(p) => p.validate(),
            ClientEvent::PresenceAnnounce(p) => p.validate(),
            ClientEvent::SyncRequest => Ok(()),
            ClientEvent::Ping => Ok(()),
        }
    }
}
