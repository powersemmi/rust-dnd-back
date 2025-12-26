use serde::Deserialize;
#[cfg(feature = "schemas")]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct Params {
    #[validate(length(min = 1, max = 255))]
    pub room_id: String,
}
