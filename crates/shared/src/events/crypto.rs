use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct CryptoKeyAnnouncePayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub username: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 1024)))]
    pub public_key_b64: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct CryptoKeyWrapPayload {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 64)))]
    pub key_id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub sender_username: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub recipient_username: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 1024)))]
    pub sender_public_key_b64: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 2048)))]
    pub nonce_b64: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1)))]
    pub wrapped_key_b64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub enum EncryptedPayloadKind {
    #[serde(rename = "CHAT")]
    Chat,
    #[serde(rename = "NOTE")]
    Note,
    #[serde(rename = "SYNC")]
    Sync,
    #[serde(rename = "FILE_CONTROL")]
    FileControl,
    #[serde(rename = "FILE_CHUNK")]
    FileChunk,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct CryptoPayload {
    #[cfg_attr(feature = "validation", validate(range(min = 1)))]
    pub version: u8,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 64)))]
    pub key_id: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub sender_username: String,
    pub kind: EncryptedPayloadKind,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 2048)))]
    pub nonce_b64: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1)))]
    pub ciphertext_b64: String,
}
