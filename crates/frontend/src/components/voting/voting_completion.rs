/// Pure functions for determining when a voting should auto-complete.
/// No signals, no Leptos imports - fully testable.
use shared::events::voting::VotingOptionResult;
use shared::events::{VotingResultPayload, VotingStartPayload};
use std::collections::HashMap;

/// Outcome of a completion check.
pub struct CompletionCheck {
    pub should_complete: bool,
}

/// Determines whether a voting session should be closed automatically.
///
/// Rules:
/// - At least 5 seconds must have elapsed since creation (`age_ms >= 5000`).
/// - AND one of: all participants voted (`total_participants > 1 && total_voted == total_participants`)
///   OR the countdown timer reached zero (`remaining_seconds == Some(0)`).
pub fn check_should_complete(
    total_participants: usize,
    total_voted: usize,
    remaining_seconds: Option<u32>,
    age_ms: f64,
) -> CompletionCheck {
    let min_age_reached = age_ms >= 5000.0;
    let all_voted = total_participants > 1 && total_voted == total_participants;
    let timer_expired = remaining_seconds == Some(0);
    CompletionCheck {
        should_complete: min_age_reached && (all_voted || timer_expired),
    }
}

/// Builds a `VotingResultPayload` by tallying votes.
pub fn compute_results(
    voting: &VotingStartPayload,
    votes: &HashMap<String, Vec<String>>,
    total_participants: usize,
) -> VotingResultPayload {
    let mut results_map: HashMap<String, u32> = HashMap::new();
    let mut voters_map: HashMap<String, Vec<String>> = HashMap::new();

    for (user, option_ids) in votes.iter() {
        for option_id in option_ids {
            *results_map.entry(option_id.clone()).or_insert(0) += 1;
            voters_map
                .entry(option_id.clone())
                .or_default()
                .push(user.clone());
        }
    }

    let results: Vec<VotingOptionResult> = voting
        .options
        .iter()
        .map(|opt| VotingOptionResult {
            option_id: opt.id.clone(),
            count: *results_map.get(&opt.id).unwrap_or(&0),
            voters: if !voting.is_anonymous {
                voters_map.get(&opt.id).cloned()
            } else {
                None
            },
        })
        .collect();

    VotingResultPayload {
        voting_id: voting.voting_id.clone(),
        question: voting.question.clone(),
        options: voting.options.clone(),
        results,
        total_participants: total_participants as u32,
        total_voted: votes.len() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::events::voting::{VotingOption, VotingType};

    fn make_voting(options: &[(&str, &str)]) -> VotingStartPayload {
        VotingStartPayload {
            voting_id: "v1".into(),
            question: "Test?".into(),
            options: options
                .iter()
                .map(|(id, text)| VotingOption {
                    id: id.to_string(),
                    text: text.to_string(),
                })
                .collect(),
            voting_type: VotingType::SingleChoice,
            is_anonymous: false,
            timer_seconds: None,
            default_option_id: None,
            creator: "alice".into(),
        }
    }

    #[test]
    fn does_not_complete_before_min_age() {
        let result = check_should_complete(3, 3, None, 2000.0);
        assert!(!result.should_complete);
    }

    #[test]
    fn completes_when_all_voted_after_min_age() {
        let result = check_should_complete(3, 3, None, 6000.0);
        assert!(result.should_complete);
    }

    #[test]
    fn completes_on_timer_expiry_after_min_age() {
        let result = check_should_complete(2, 1, Some(0), 6000.0);
        assert!(result.should_complete);
    }

    #[test]
    fn does_not_complete_with_one_participant() {
        // Single participant can never satisfy `total_participants > 1 && all_voted`
        let result = check_should_complete(1, 1, None, 6000.0);
        assert!(!result.should_complete);
    }

    #[test]
    fn compute_results_tallies_votes_correctly() {
        let voting = make_voting(&[("opt_a", "Yes"), ("opt_b", "No")]);
        let mut votes: HashMap<String, Vec<String>> = HashMap::new();
        votes.insert("alice".into(), vec!["opt_a".into()]);
        votes.insert("bob".into(), vec!["opt_a".into()]);
        votes.insert("carol".into(), vec!["opt_b".into()]);

        let result = compute_results(&voting, &votes, 3);
        assert_eq!(result.total_participants, 3);
        assert_eq!(result.total_voted, 3);
        let yes = result
            .results
            .iter()
            .find(|r| r.option_id == "opt_a")
            .unwrap();
        let no = result
            .results
            .iter()
            .find(|r| r.option_id == "opt_b")
            .unwrap();
        assert_eq!(yes.count, 2);
        assert_eq!(no.count, 1);
    }

    #[test]
    fn compute_results_hides_voters_when_anonymous() {
        let mut voting = make_voting(&[("opt_a", "Yes")]);
        voting.is_anonymous = true;
        let mut votes: HashMap<String, Vec<String>> = HashMap::new();
        votes.insert("alice".into(), vec!["opt_a".into()]);

        let result = compute_results(&voting, &votes, 2);
        let opt = &result.results[0];
        assert!(opt.voters.is_none());
    }

    #[test]
    fn compute_results_shows_voters_when_not_anonymous() {
        let voting = make_voting(&[("opt_a", "Yes")]);
        let mut votes: HashMap<String, Vec<String>> = HashMap::new();
        votes.insert("alice".into(), vec!["opt_a".into()]);

        let result = compute_results(&voting, &votes, 2);
        let opt = &result.results[0];
        assert!(opt.voters.is_some());
    }
}
