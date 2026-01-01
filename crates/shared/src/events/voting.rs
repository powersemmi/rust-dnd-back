use serde::{Deserialize, Serialize};
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct VotingOption {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub enum VotingType {
    SingleChoice,
    MultipleChoice,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct VotingStartPayload {
    pub voting_id: String,
    pub question: String,
    pub options: Vec<VotingOption>,
    pub voting_type: VotingType,
    pub is_anonymous: bool,
    pub timer_seconds: Option<u32>,
    pub default_option_id: Option<String>, // Для single choice с таймером
    pub creator: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct VotingCastPayload {
    pub voting_id: String,
    pub user: String,
    pub selected_option_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct VotingOptionResult {
    pub option_id: String,
    pub count: u32,
    pub voters: Option<Vec<String>>, // None если анонимное
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct VotingResultPayload {
    pub voting_id: String,
    pub results: Vec<VotingOptionResult>,
    pub total_participants: u32,
    pub total_voted: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct VotingEndPayload {
    pub voting_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct PresenceRequestPayload {
    pub request_id: String,
    pub requester: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct PresenceResponsePayload {
    pub request_id: String,
    pub user: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct PresenceAnnouncePayload {
    pub request_id: String,
    pub online_users: Vec<String>,
}
