/// A single entry in the state event log.
#[derive(Clone, Debug)]
pub struct StateEvent {
    pub version: u64,
    pub event_type: String,
    pub description: String,
    pub timestamp: String,
}

/// Tab variants for the statistics window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsTab {
    VotingResults,
    EventLog,
}

/// Compute vote percentage as an integer (0-100), guarding against division by zero.
pub fn compute_percentage(count: u32, total_voted: u32) -> u32 {
    if total_voted == 0 {
        return 0;
    }
    (count as f32 / total_voted as f32 * 100.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_with_zero_total_returns_zero() {
        assert_eq!(compute_percentage(5, 0), 0);
    }

    #[test]
    fn full_vote_is_100_percent() {
        assert_eq!(compute_percentage(10, 10), 100);
    }

    #[test]
    fn half_vote_is_50_percent() {
        assert_eq!(compute_percentage(5, 10), 50);
    }

    #[test]
    fn one_of_three_rounds_down() {
        assert_eq!(compute_percentage(1, 3), 33);
    }
}
