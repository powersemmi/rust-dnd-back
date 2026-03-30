use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct FileRef {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub hash: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub mime_type: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct SceneGrid {
    #[cfg_attr(feature = "validation", validate(range(min = 1, max = 200)))]
    pub columns: u16,
    #[cfg_attr(feature = "validation", validate(range(min = 1, max = 200)))]
    pub rows: u16,
    #[cfg_attr(feature = "validation", validate(range(min = 1, max = 100)))]
    pub cell_size_feet: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct Scene {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub name: String,
    #[cfg_attr(feature = "validation", validate(nested))]
    pub grid: SceneGrid,
    #[serde(default)]
    pub workspace_x: f32,
    #[serde(default)]
    pub workspace_y: f32,
    #[serde(default)]
    #[cfg_attr(feature = "validation", validate(nested))]
    pub background: Option<FileRef>,
    #[serde(default = "default_background_scale")]
    pub background_scale: f32,
    #[serde(default)]
    pub background_offset_x: f32,
    #[serde(default)]
    pub background_offset_y: f32,
    #[serde(default)]
    pub background_rotation_deg: f32,
}

const fn default_background_scale() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct SceneCreatePayload {
    #[cfg_attr(feature = "validation", validate(nested))]
    pub scene: Scene,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub actor: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct SceneUpdatePayload {
    #[cfg_attr(feature = "validation", validate(nested))]
    pub scene: Scene,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub actor: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct SceneDeletePayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub scene_id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub actor: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct SceneActivatePayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub scene_id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 255)))]
    pub actor: String,
}
