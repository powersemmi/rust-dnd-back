use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub enum MouseEventTypeEnum {
    Left = 0,
    Right = 1,
    Middle = 2,
    Move = 3,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct MouseClickPayload {
    pub x: f64,

    pub y: f64,

    pub mouse_event_type: MouseEventTypeEnum,

    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub user_id: String,
}
