use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub enum NoteVisibility {
    Public,
    Private,
    Direct(String),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct NoteBoardPosition {
    pub world_x: f64,
    pub world_y: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct NoteBoardStyle {
    #[serde(default = "default_note_board_width_px")]
    pub width_px: f64,
    #[serde(default = "default_note_board_height_px")]
    pub height_px: f64,
    #[serde(default = "default_note_board_font_size_pt")]
    pub font_size_pt: f64,
    #[serde(default = "default_note_board_color")]
    pub color: String,
}

const fn default_note_board_width_px() -> f64 {
    280.0
}

const fn default_note_board_height_px() -> f64 {
    220.0
}

const fn default_note_board_font_size_pt() -> f64 {
    14.0
}

fn default_note_board_color() -> String {
    "#F8EE96".to_string()
}

impl Default for NoteBoardStyle {
    fn default() -> Self {
        Self {
            width_px: default_note_board_width_px(),
            height_px: default_note_board_height_px(),
            font_size_pt: default_note_board_font_size_pt(),
            color: default_note_board_color(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct NotePayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub author: String,
    pub visibility: NoteVisibility,
    #[serde(default)]
    #[cfg_attr(feature = "validation", validate(length(max = 120)))]
    pub title: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 20_000)))]
    pub body: String,
    pub created_at_ms: f64,
    pub updated_at_ms: f64,
    #[serde(default)]
    pub board_position: Option<NoteBoardPosition>,
    #[serde(default)]
    pub board_style: NoteBoardStyle,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct NoteDeletePayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub author: String,
    pub visibility: NoteVisibility,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_payload_defaults_to_board_defaults() {
        let raw = r#"{
            "id":"note-1",
            "author":"gm",
            "visibility":"Public",
            "title":"Plan",
            "body":"Body",
            "created_at_ms":1,
            "updated_at_ms":2
        }"#;

        let note: NotePayload = serde_json::from_str(raw).unwrap();
        assert_eq!(note.board_position, None);
        assert_eq!(note.board_style, NoteBoardStyle::default());
    }
}
