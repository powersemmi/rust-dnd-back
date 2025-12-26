use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct MouseMovePayload {
    #[cfg_attr(feature = "validation", validate(range(min = 0, max = 10000)))]
    pub x: i32,

    #[cfg_attr(feature = "validation", validate(range(min = 0, max = 10000)))]
    pub y: i32,

    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub user_id: String,
}
