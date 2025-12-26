use serde::Deserialize;
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChatMessagePayload {
    #[validate(length(min = 1, max = 500))]
    pub text: String,

    #[validate(length(min = 1, max = 255))]
    pub user_id: String,
}
