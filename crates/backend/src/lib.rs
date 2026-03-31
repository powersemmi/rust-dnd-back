pub mod config;
pub mod docs;
pub mod error;
pub mod handlers;
pub mod http_rate_limit;
pub mod state;
pub mod utils;
pub mod ws_policy;

pub use config::Config;
pub use error::{AppError, AppResult};
use shared::events::{
    AttentionPingPayload, BoardPointerPayload, ChatMessagePayload, ClientEvent,
    CryptoKeyAnnouncePayload, CryptoKeyWrapPayload, CryptoPayload, DirectMessagePayload,
    EncryptedPayloadKind, FileAnnouncePayload, FileChunkPayload, FileRef, FileRequestPayload,
    MouseClickPayload, NoteDeletePayload, NotePayload, NoteVisibility, PresenceAnnouncePayload,
    PresenceRequestPayload, PresenceResponsePayload, Scene, SceneActivatePayload, SceneCreatePayload,
    SceneDeletePayload, SceneGrid, SceneUpdatePayload, SyncSnapshotPayload,
    SyncSnapshotRequestPayload, SyncVersionPayload, Token, TokenMovePayload, VotingCastPayload,
    VotingEndPayload, VotingResultPayload, VotingStartPayload, WorldPoint,
};
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
        handlers::auth::get_me,
        handlers::auth::refresh_token,
    ),
    components(
        schemas(
            // Core envelope
            ClientEvent,
            // Chat / DM
            ChatMessagePayload,
            DirectMessagePayload,
            // Crypto
            CryptoKeyAnnouncePayload,
            CryptoKeyWrapPayload,
            CryptoPayload,
            EncryptedPayloadKind,
            // Cursor / presence
            MouseClickPayload,
            PresenceRequestPayload,
            PresenceResponsePayload,
            PresenceAnnouncePayload,
            // Notes
            NotePayload,
            NoteDeletePayload,
            NoteVisibility,
            // Scenes / tokens
            Scene,
            SceneGrid,
            SceneCreatePayload,
            SceneUpdatePayload,
            SceneDeletePayload,
            SceneActivatePayload,
            Token,
            TokenMovePayload,
            FileRef,
            // Files
            FileAnnouncePayload,
            FileRequestPayload,
            FileChunkPayload,
            // Voting
            VotingStartPayload,
            VotingCastPayload,
            VotingResultPayload,
            VotingEndPayload,
            // Sync
            SyncVersionPayload,
            SyncSnapshotRequestPayload,
            SyncSnapshotPayload,
            // Board tools
            BoardPointerPayload,
            AttentionPingPayload,
            WorldPoint,
        ),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "dnd-back", description = "D&D Virtual Tabletop API"),
        (name = "WebSocket Protocol", description = "Формат сообщений WebSocket Realtime API")
    )
)]
pub struct ApiDoc;
