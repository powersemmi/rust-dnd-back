use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MouseMoveTokenPayload {
    #[validate(range(min = 0, max = 10000))]
    pub x: u32,

    #[validate(range(min = 0, max = 10000))]
    pub y: u32,

    #[validate(length(min = 1, max = 255))]
    pub user_id: String,
}
