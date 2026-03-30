use super::model::StatisticsTab;
use leptos::prelude::*;

/// Reactive state for the statistics window.
#[derive(Clone, Copy)]
pub struct StatisticsViewModel {
    pub active_tab: RwSignal<StatisticsTab>,
}

impl StatisticsViewModel {
    pub fn new() -> Self {
        Self {
            active_tab: RwSignal::new(StatisticsTab::VotingResults),
        }
    }

    pub fn switch_to_voting_results(&self) {
        self.active_tab.set(StatisticsTab::VotingResults);
    }

    pub fn switch_to_event_log(&self) {
        self.active_tab.set(StatisticsTab::EventLog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn default_tab_is_voting_results() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = StatisticsViewModel::new();
            assert_eq!(vm.active_tab.get_untracked(), StatisticsTab::VotingResults);
        });
    }

    #[test]
    fn switch_to_event_log_changes_tab() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = StatisticsViewModel::new();
            vm.switch_to_event_log();
            assert_eq!(vm.active_tab.get_untracked(), StatisticsTab::EventLog);
        });
    }
}
