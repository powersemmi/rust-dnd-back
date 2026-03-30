use leptos::prelude::*;
use shared::events::VotingStartPayload;
use shared::events::voting::{VotingOption, VotingType};
use uuid::Uuid;

/// Reactive state and logic for the voting creation form.
#[derive(Clone, Copy)]
pub struct VotingCreateViewModel {
    pub question: RwSignal<String>,
    pub options: RwSignal<Vec<String>>,
    pub voting_type: RwSignal<VotingType>,
    pub is_anonymous: RwSignal<bool>,
    pub timer_str: RwSignal<String>,
    pub default_option_index: RwSignal<usize>,
}

impl VotingCreateViewModel {
    pub fn new() -> Self {
        Self {
            question: RwSignal::new(String::new()),
            options: RwSignal::new(vec![String::new(), String::new()]),
            voting_type: RwSignal::new(VotingType::SingleChoice),
            is_anonymous: RwSignal::new(false),
            timer_str: RwSignal::new(String::new()),
            default_option_index: RwSignal::new(0),
        }
    }

    pub fn add_option(&self) {
        self.options.update(|opts| opts.push(String::new()));
    }

    /// Removes an option at the given index. Does nothing if fewer than 3 options exist.
    pub fn remove_option(&self, index: usize) {
        self.options.update(|opts| {
            if opts.len() > 2 {
                opts.remove(index);
            }
        });
    }

    pub fn update_option(&self, index: usize, value: String) {
        self.options.update(|opts| {
            if index < opts.len() {
                opts[index] = value;
            }
        });
    }

    /// Resets the form to its initial state.
    pub fn reset(&self) {
        self.question.set(String::new());
        self.options.set(vec![String::new(), String::new()]);
        self.voting_type.set(VotingType::SingleChoice);
        self.is_anonymous.set(false);
        self.timer_str.set(String::new());
        self.default_option_index.set(0);
    }

    /// Builds the voting payload if all required fields are valid.
    /// Returns `None` if the question is empty or fewer than 2 non-empty options exist.
    pub fn build_payload(&self) -> Option<VotingStartPayload> {
        let question = self.question.get_untracked();
        if question.is_empty() {
            return None;
        }

        let options = self.options.get_untracked();
        let valid_options: Vec<_> = options.iter().filter(|s| !s.is_empty()).collect();
        if valid_options.len() < 2 {
            return None;
        }

        let voting_options: Vec<VotingOption> = valid_options
            .iter()
            .map(|text| VotingOption {
                id: Uuid::new_v4().to_string(),
                text: (*text).clone(),
            })
            .collect();

        let timer_seconds = if matches!(self.voting_type.get_untracked(), VotingType::SingleChoice)
        {
            self.timer_str.get_untracked().parse::<u32>().ok()
        } else {
            None
        };

        let default_option_id = timer_seconds.and_then(|_| {
            let idx = self.default_option_index.get_untracked();
            voting_options.get(idx).map(|opt| opt.id.clone())
        });

        Some(VotingStartPayload {
            voting_id: Uuid::new_v4().to_string(),
            question,
            options: voting_options,
            voting_type: self.voting_type.get_untracked(),
            is_anonymous: self.is_anonymous.get_untracked(),
            timer_seconds,
            default_option_id,
            creator: String::new(), // filled by parent
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn new_vm_has_two_empty_options() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            assert_eq!(vm.options.get_untracked().len(), 2);
        });
    }

    #[test]
    fn add_option_increases_count() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            vm.add_option();
            assert_eq!(vm.options.get_untracked().len(), 3);
        });
    }

    #[test]
    fn remove_option_decreases_count_when_above_two() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            vm.add_option(); // 3 options
            vm.remove_option(0);
            assert_eq!(vm.options.get_untracked().len(), 2);
        });
    }

    #[test]
    fn remove_option_does_not_go_below_two() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            vm.remove_option(0);
            assert_eq!(vm.options.get_untracked().len(), 2);
        });
    }

    #[test]
    fn build_payload_returns_none_on_empty_question() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            vm.update_option(0, "Yes".into());
            vm.update_option(1, "No".into());
            assert!(vm.build_payload().is_none());
        });
    }

    #[test]
    fn build_payload_returns_none_with_fewer_than_two_filled_options() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            vm.question.set("Should we proceed?".into());
            vm.update_option(0, "Yes".into());
            // option 1 remains empty
            assert!(vm.build_payload().is_none());
        });
    }

    #[test]
    fn build_payload_succeeds_with_valid_input() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            vm.question.set("Should we proceed?".into());
            vm.update_option(0, "Yes".into());
            vm.update_option(1, "No".into());
            let payload = vm.build_payload();
            assert!(payload.is_some());
            let p = payload.unwrap();
            assert_eq!(p.question, "Should we proceed?");
            assert_eq!(p.options.len(), 2);
        });
    }

    #[test]
    fn reset_clears_all_fields() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = VotingCreateViewModel::new();
            vm.question.set("Q?".into());
            vm.update_option(0, "A".into());
            vm.add_option();
            vm.reset();
            assert_eq!(vm.question.get_untracked(), "");
            assert_eq!(vm.options.get_untracked().len(), 2);
            assert!(vm.options.get_untracked()[0].is_empty());
        });
    }
}
