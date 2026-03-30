use crate::components::websocket::WsSender;
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, ClientEvent};

/// Reactive state and logic for the chat window.
#[derive(Clone, Copy)]
pub struct ChatViewModel {
    pub input_text: RwSignal<String>,
}

impl ChatViewModel {
    pub fn new() -> Self {
        Self {
            input_text: RwSignal::new(String::new()),
        }
    }

    /// Returns true if the current input has content that can be sent.
    pub fn can_send(&self) -> bool {
        !self.input_text.get_untracked().is_empty()
    }

    /// Reads and clears the input text, returning the previous value.
    pub fn take_input(&self) -> String {
        let text = self.input_text.get_untracked();
        self.input_text.set(String::new());
        text
    }

    /// Sends the current message via WebSocket if non-empty.
    /// Returns `true` if a message was sent.
    pub fn send_message(&self, username: &str, ws_sender: ReadSignal<Option<WsSender>>) -> bool {
        if !self.can_send() {
            return false;
        }
        let text = self.take_input();
        let msg = ChatMessagePayload {
            payload: text,
            username: username.to_string(),
        };
        if let Some(sender) = ws_sender.get_untracked() {
            let _ = sender.try_send_event(ClientEvent::ChatMessage(msg));
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn can_send_is_false_when_empty() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ChatViewModel::new();
            assert!(!vm.can_send());
        });
    }

    #[test]
    fn can_send_is_true_with_content() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ChatViewModel::new();
            vm.input_text.set("Hello".into());
            assert!(vm.can_send());
        });
    }

    #[test]
    fn take_input_clears_text() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ChatViewModel::new();
            vm.input_text.set("test message".into());
            let taken = vm.take_input();
            assert_eq!(taken, "test message");
            assert_eq!(vm.input_text.get_untracked(), "");
        });
    }

    #[test]
    fn take_input_on_empty_returns_empty() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ChatViewModel::new();
            let taken = vm.take_input();
            assert_eq!(taken, "");
        });
    }
}
