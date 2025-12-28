pub mod config;
pub mod docs;
pub mod handlers;
pub mod state;
pub mod utils;

pub use config::Config;
use shared::events::{ChatMessagePayload, ClientEvent, MouseClickPayload};
pub use state::AppState;
use utoipa::{Modify, OpenApi};

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // Use a match or if let to safely get the components
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth", // The name used to reference this scheme
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        docs::websocket_docs,
        handlers::auth::register,
        handlers::auth::login,
        handlers::auth::get_me
    ),
    components(
        // 3. Регистрируем все структуры, участвующие в документации
        schemas(
            ClientEvent,
            ChatMessagePayload,
            MouseClickPayload
        ),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "dnd-back", description = "D&D Virtual Tabletop API"),
        (name = "WebSocket Protocol", description = "Формат сообщений Realtime API")
    )
)]
pub struct ApiDoc;
