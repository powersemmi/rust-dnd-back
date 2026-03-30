use crate::events::scene::FileRef;
use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct FileAnnouncePayload {
    #[cfg_attr(feature = "validation", validate(nested))]
    pub file: FileRef,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub from: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct FileRequestPayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub hash: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub requester: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct FileChunkPayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub hash: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub requester: String,
    #[cfg_attr(feature = "validation", validate(range(min = 0)))]
    pub chunk_index: u32,
    #[cfg_attr(feature = "validation", validate(range(min = 1)))]
    pub total_chunks: u32,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 131072)))]
    pub data: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct FileAbortPayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub hash: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub requester: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 500)))]
    pub reason: String,
}
