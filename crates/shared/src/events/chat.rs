use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct ChatMessagePayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 500)))]
    pub text: String,

    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub user_id: String,
}
