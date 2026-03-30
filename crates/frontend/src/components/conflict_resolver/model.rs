/// Pure logic for conflict resolution voting.
use shared::events::voting::{VotingOption, VotingStartPayload, VotingType};

/// Option ID used for "Yes" (proceed with forced sync).
pub const VOTE_YES_ID: &str = ".1";
/// Option ID used for "No" (default timeout vote).
pub const VOTE_NO_ID: &str = ".0";
/// Timer for conflict votes (seconds).
pub const CONFLICT_VOTE_TIMER_SECS: u32 = 60;

/// Builds the voting payload used to resolve a sync conflict by majority vote.
/// The `question` should already be translated.
pub fn build_conflict_vote_payload(
    voting_id: String,
    question: String,
    creator: String,
) -> VotingStartPayload {
    VotingStartPayload {
        voting_id,
        question,
        options: vec![
            VotingOption {
                id: VOTE_NO_ID.to_string(),
                text: VOTE_NO_ID.to_string(),
            },
            VotingOption {
                id: VOTE_YES_ID.to_string(),
                text: VOTE_YES_ID.to_string(),
            },
        ],
        voting_type: VotingType::SingleChoice,
        is_anonymous: false,
        timer_seconds: Some(CONFLICT_VOTE_TIMER_SECS),
        default_option_id: Some(VOTE_NO_ID.to_string()),
        creator,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_payload_sets_correct_fields() {
        let payload = build_conflict_vote_payload(
            "conflict_vote_1".to_string(),
            "Force sync?".to_string(),
            "alice".to_string(),
        );
        assert_eq!(payload.voting_id, "conflict_vote_1");
        assert_eq!(payload.creator, "alice");
        assert_eq!(payload.options.len(), 2);
        assert_eq!(payload.default_option_id.unwrap(), VOTE_NO_ID);
        assert_eq!(payload.timer_seconds.unwrap(), CONFLICT_VOTE_TIMER_SECS);
    }

    #[test]
    fn vote_yes_id_is_distinct_from_no() {
        assert_ne!(VOTE_YES_ID, VOTE_NO_ID);
    }
}
