use shared::events::VotingStartPayload;
use shared::events::voting::VotingOptionResult;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum VotingState {
    Active {
        voting: VotingStartPayload,
        participants: Vec<String>,
        votes: HashMap<String, Vec<String>>, // user -> selected_option_ids
        remaining_seconds: Option<u32>,
        created_at: f64, // timestamp in milliseconds
    },
    Results {
        voting: VotingStartPayload,
        results: Vec<VotingOptionResult>,
        total_participants: u32,
        total_voted: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum VotingTab {
    List,
    Create,
    Voting(String), // voting_id
}
