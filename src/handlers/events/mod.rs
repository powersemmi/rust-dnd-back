mod chat;
mod mouse_move;

pub(crate) use crate::handlers::events::chat::ChatMessagePayload;
pub(crate) use crate::handlers::events::mouse_move::MouseMoveTokenPayload;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data")]
pub enum ClientEvent {
    #[serde(rename = "MOUSE_MOVE_TOKEN")]
    MouseMoveToken(MouseMoveTokenPayload),

    #[serde(rename = "CHAT_MESSAGE")]
    ChatMessage(ChatMessagePayload),

    #[serde(rename = "PING")]
    Ping,
}

impl ClientEvent {
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            ClientEvent::MouseMoveToken(p) => p.validate(),
            ClientEvent::ChatMessage(p) => p.validate(),
            ClientEvent::Ping => Ok(()),
        }
    }
}
