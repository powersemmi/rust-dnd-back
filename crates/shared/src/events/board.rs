use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

/// A point in world coordinates used by ping events.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct WorldPoint {
    pub x: f64,
    pub y: f64,
}

/// Sent once when a user activates or deactivates the board pointer tool.
/// The trail is constructed by receivers from the existing cursor-position stream
/// (MOUSE_EVENT), so no coordinates are transmitted here.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct BoardPointerPayload {
    /// The username toggling pointer mode.
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub username: String,

    /// `true` = pointer mode activated; `false` = deactivated.
    pub active: bool,
}

/// Broadcast when a user fires an attention ping (Alt+LMB) at a world coordinate.
/// Receivers show a visual effect at the pinged position (like a pulsing ring).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct AttentionPingPayload {
    /// The username who fired the ping.
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub username: String,

    /// World position of the ping.
    pub position: WorldPoint,
}

/// A direct message sent from one user to a specific recipient via "@nick message".
/// The backend broadcasts this to the whole room; only the addressed recipient
/// processes and stores it.  The content is always encrypted.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct DirectMessagePayload {
    /// Sender username.
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub from: String,

    /// Recipient username (client-side filter: non-recipients discard).
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub to: String,

    /// Message text.
    #[cfg_attr(feature = "validation", validate(length(max = 2000)))]
    pub body: String,

    /// Unix timestamp in milliseconds.
    pub sent_at_ms: f64,
}
