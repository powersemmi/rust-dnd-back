use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub enum ChatEventTypeEnum {
    Text = 0,
    Roll = 1,
    Spawn = 2,
    Move = 3,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct ChatMessagePayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 500)))]
    pub payload: String,

    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub username: String,
}
