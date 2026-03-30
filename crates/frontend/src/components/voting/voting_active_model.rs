/// Pure display helpers for an active voting view.
/// No signals, no Leptos.
use std::collections::HashMap;

/// Aggregates per-option vote counts from a votes map (user -> selected_option_ids).
pub fn compute_vote_counts(votes: &HashMap<String, Vec<String>>) -> HashMap<String, u32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for option_ids in votes.values() {
        for id in option_ids {
            *counts.entry(id.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// Returns the integer percentage (0-100) of `count` out of `total`.
/// Returns 0 if total is 0 to avoid division by zero.
pub fn vote_percentage(count: u32, total: u32) -> u32 {
    if total == 0 {
        0
    } else {
        (count as f32 / total as f32 * 100.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_vote_counts_single_vote() {
        let mut votes = HashMap::new();
        votes.insert("alice".to_string(), vec!["opt_a".to_string()]);
        let counts = compute_vote_counts(&votes);
        assert_eq!(counts["opt_a"], 1);
    }

    #[test]
    fn compute_vote_counts_multiple_users() {
        let mut votes = HashMap::new();
        votes.insert("alice".to_string(), vec!["opt_a".to_string()]);
        votes.insert("bob".to_string(), vec!["opt_a".to_string()]);
        votes.insert("carol".to_string(), vec!["opt_b".to_string()]);
        let counts = compute_vote_counts(&votes);
        assert_eq!(counts["opt_a"], 2);
        assert_eq!(counts["opt_b"], 1);
    }

    #[test]
    fn vote_percentage_basic() {
        assert_eq!(vote_percentage(1, 4), 25);
        assert_eq!(vote_percentage(0, 4), 0);
        assert_eq!(vote_percentage(4, 4), 100);
    }

    #[test]
    fn vote_percentage_zero_total() {
        assert_eq!(vote_percentage(0, 0), 0);
    }
}
