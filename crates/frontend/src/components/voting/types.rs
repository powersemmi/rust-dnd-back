use shared::events::VotingStartPayload;
use shared::events::voting::VotingOptionResult;

#[derive(Clone, Debug)]
pub enum VotingState {
    Active(VotingStartPayload),
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
