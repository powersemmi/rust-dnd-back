pub mod config;
pub mod docs;
pub mod state;

pub mod handler;

pub use config::Config;
use shared::events::{ChatMessagePayload, ClientEvent, MouseMovePayload};
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
            MouseMovePayload
        )
    ),
    tags(
        (name = "dnd-back", description = "D&D Virtual Tabletop API"),
        (name = "WebSocket Protocol", description = "Формат сообщений Realtime API")
    )
)]
pub struct ApiDoc;
