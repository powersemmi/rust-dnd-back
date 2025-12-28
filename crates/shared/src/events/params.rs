use serde::Deserialize;
#[cfg(feature = "schemas")]
use utoipa::{IntoParams, ToSchema};
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema, IntoParams))]
pub struct Params {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub room_id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 32, max = 32)))]
    #[cfg_attr(feature = "schemas", schema(value_type = String, format = "uuid"))]
    pub token: String,
}
