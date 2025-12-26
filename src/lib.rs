pub mod config;
pub mod docs;
pub mod handlers;
pub mod state;

use crate::handlers::events::{ChatMessagePayload, ClientEvent, MouseMoveTokenPayload};
pub use config::Config;
pub use state::AppState;
use utoipa::OpenApi;

// Регистрируем структуру документации
#[derive(OpenApi)]
#[openapi(
    paths(
        docs::websocket_docs
    ),
    components(
        // 3. Регистрируем все структуры, участвующие в документации
        schemas(
            ClientEvent,
            ChatMessagePayload,
            MouseMoveTokenPayload
        )
    ),
    tags(
        (name = "dnd-back", description = "D&D Virtual Tabletop API"),
        (name = "WebSocket Protocol", description = "Формат сообщений Realtime API")
    )
)]
pub struct ApiDoc;
