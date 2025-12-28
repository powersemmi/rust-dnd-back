use crate::events::chat::ChatMessagePayload;
use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

/// Полное состояние комнаты, которое мы синхронизируем
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct RoomState {
    /// История сообщений чата
    #[cfg_attr(feature = "validation", validate(length(min = 0)))] // Просто проверяем, что это список
    pub chat_history: Vec<ChatMessagePayload>,

    // В будущем сюда добавим:
    // pub tokens: Vec<TokenState>,
    // pub scene: String,
}