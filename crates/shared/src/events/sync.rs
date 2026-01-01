use crate::events::room::RoomState;
use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct SyncVersionPayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub username: String,
    #[cfg_attr(feature = "validation", validate(range(min = 0)))]
    pub version: u64,
    pub state_hash: String,
    // Список последних хешей (до 500) для определения точки расхождения
    pub recent_hashes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct SyncSnapshotRequestPayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub target_username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct SyncSnapshotPayload {
    #[cfg_attr(feature = "validation", validate(range(min = 0)))]
    pub version: u64,
    #[cfg_attr(feature = "schemas", schema(value_type = Object))]
    pub state: RoomState,
}
