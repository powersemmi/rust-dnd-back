pub mod chat;
pub mod mouse;
pub mod params;
pub mod sync;

pub use crate::events::chat::ChatMessagePayload;
pub use crate::events::mouse::MouseClickPayload;
pub use crate::events::params::Params;
use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;
use crate::events::sync::{SyncSnapshotPayload, SyncSnapshotRequestPayload, SyncVersionPayload};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[serde(tag = "type", content = "data")]
pub enum ClientEvent {
    #[serde(rename = "MOUSE_EVENT")]
    MouseClickPayload(MouseClickPayload),

    #[serde(rename = "CHAT_MESSAGE")]
    ChatMessage(ChatMessagePayload),

    #[serde(rename = "PING")]
    Ping,

    /// Sync events
    #[serde(rename = "SYNC_REQUEST")]
    SyncRequest,
    #[serde(rename = "SYNC_VERSION_ANNOUNCE")]
    SyncVersionAnnounce(SyncVersionPayload),
    #[serde(rename = "SYNC_SNAPSHOT_REQUEST")]
    SyncSnapshotRequest(SyncSnapshotRequestPayload),
    #[serde(rename = "SYNC_SNAPSHOT")]
    SyncSnapshot(SyncSnapshotPayload),
}

#[cfg(feature = "validation")]
impl ClientEvent {
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            ClientEvent::MouseClickPayload(p) => p.validate(),
            ClientEvent::ChatMessage(p) => p.validate(),
            ClientEvent::SyncVersionAnnounce(p) => p.validate(),
            ClientEvent::SyncSnapshotRequest(p) => p.validate(),
            ClientEvent::SyncSnapshot(p) => p.validate(),
            ClientEvent::SyncRequest => Ok(()),
            ClientEvent::Ping => Ok(()),
        }
    }
}
