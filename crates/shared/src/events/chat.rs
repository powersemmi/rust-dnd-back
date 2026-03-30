use crate::events::scene::FileRef;
use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct ChatMessagePayload {
    #[cfg_attr(feature = "validation", validate(length(max = 500)))]
    pub payload: String,

    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub username: String,

    #[serde(default)]
    #[cfg_attr(feature = "validation", validate(nested))]
    pub attachments: Vec<FileRef>,
}
