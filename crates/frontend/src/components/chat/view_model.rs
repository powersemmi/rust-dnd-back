use crate::components::websocket::WsSender;
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, ClientEvent, DirectMessagePayload, FileRef};

/// Reactive state and logic for the chat window.
#[derive(Clone, Copy)]
pub struct ChatViewModel {
    pub input_text: RwSignal<String>,
    pub pending_attachments: RwSignal<Vec<FileRef>>,
    pub error_message: RwSignal<Option<String>>,
}

impl ChatViewModel {
    pub fn new() -> Self {
        Self {
            input_text: RwSignal::new(String::new()),
            pending_attachments: RwSignal::new(Vec::new()),
            error_message: RwSignal::new(None),
        }
    }

    /// Returns true if the current input has content that can be sent.
    pub fn can_send(&self) -> bool {
        !self.input_text.get_untracked().trim().is_empty()
            || !self.pending_attachments.get_untracked().is_empty()
    }

    pub fn add_attachment(&self, file: FileRef) {
        self.pending_attachments.update(|attachments| {
            if attachments
                .iter()
                .any(|existing| existing.hash == file.hash)
            {
                return;
            }
            attachments.push(file);
        });
        self.error_message.set(None);
    }

    pub fn remove_attachment(&self, hash: &str) {
        self.pending_attachments
            .update(|attachments| attachments.retain(|file| file.hash != hash));
    }

    /// Reads and clears the current draft, returning text and attachments.
    pub fn clear_draft(&self) {
        self.input_text.set(String::new());
        self.pending_attachments.set(Vec::new());
        self.error_message.set(None);
    }

    /// Parses `@nick message` syntax from the draft text.
    /// Returns `Some(("nick", "message body"))` if the text starts with `@nick`.
    pub fn parse_direct_message(text: &str) -> Option<(&str, &str)> {
        let text = text.trim();
        if !text.starts_with('@') {
            return None;
        }
        // "@nick rest of message" – split at first whitespace after the nick.
        let rest = &text[1..];
        let mut parts = rest.splitn(2, |c: char| c.is_whitespace());
        let nick = parts.next().filter(|n| !n.is_empty())?;
        let body = parts.next().map(|s| s.trim()).unwrap_or("").trim();
        if body.is_empty() {
            return None;
        }
        Some((nick, body))
    }

    /// Sends the current message via WebSocket if non-empty.
    /// If the text matches `@nick message`, it is sent as a `DirectMessage` instead.
    /// Returns `true` if a message was sent.
    pub fn send_message(&self, username: &str, ws_sender: ReadSignal<Option<WsSender>>) -> bool {
        let Some(sender) = ws_sender.get_untracked() else {
            self.error_message
                .set(Some("WebSocket connection is not available".to_string()));
            return false;
        };
        if !self.can_send() {
            return false;
        }

        let text = self.input_text.get_untracked();
        let trimmed = text.trim().to_string();

        // Route "@nick message" as a DirectMessage.
        if let Some((recipient, body)) = Self::parse_direct_message(&trimmed) {
            let now = js_sys::Date::now();
            let dm = DirectMessagePayload {
                from: username.to_string(),
                to: recipient.to_string(),
                body: body.to_string(),
                sent_at_ms: now,
            };
            match sender.try_send_event(ClientEvent::DirectMessage(dm)) {
                Ok(()) => {
                    self.clear_draft();
                    return true;
                }
                Err(error) => {
                    self.error_message.set(Some(error));
                    return false;
                }
            }
        }

        let attachments = self.pending_attachments.get_untracked();
        let msg = ChatMessagePayload {
            payload: trimmed,
            username: username.to_string(),
            attachments,
        };

        match sender.try_send_event(ClientEvent::ChatMessage(msg)) {
            Ok(()) => {
                self.clear_draft();
                true
            }
            Err(error) => {
                self.error_message.set(Some(error));
                false
            }
        }
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
    fn can_send_is_true_with_attachment_only() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ChatViewModel::new();
            vm.add_attachment(FileRef {
                hash: "hash".into(),
                mime_type: "application/pdf".into(),
                file_name: "sheet.pdf".into(),
                size: 42,
            });
            assert!(vm.can_send());
        });
    }

    // --- Direct-message parsing tests ---

    #[test]
    fn parse_direct_message_basic() {
        let result = ChatViewModel::parse_direct_message("@Alice hello there");
        assert_eq!(result, Some(("Alice", "hello there")));
    }

    #[test]
    fn parse_direct_message_with_leading_spaces() {
        let result = ChatViewModel::parse_direct_message("  @Bob   how are you  ");
        assert_eq!(result, Some(("Bob", "how are you")));
    }

    #[test]
    fn parse_direct_message_no_at() {
        assert_eq!(ChatViewModel::parse_direct_message("hello there"), None);
    }

    #[test]
    fn parse_direct_message_no_body() {
        // "@nick" with no body → None.
        assert_eq!(ChatViewModel::parse_direct_message("@alice"), None);
    }

    #[test]
    fn parse_direct_message_empty_body_after_nick() {
        // "@alice   " → None (body is whitespace only).
        assert_eq!(ChatViewModel::parse_direct_message("@alice   "), None);
    }

    #[test]
    fn parse_direct_message_at_without_nick() {
        // "@ message" → nick is empty → None.
        assert_eq!(ChatViewModel::parse_direct_message("@ message"), None);
    }

    #[test]
    fn clear_draft_resets_text_and_attachments() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = ChatViewModel::new();
            vm.input_text.set("test message".into());
            vm.add_attachment(FileRef {
                hash: "hash".into(),
                mime_type: "application/pdf".into(),
                file_name: "sheet.pdf".into(),
                size: 42,
            });
            vm.clear_draft();
            assert_eq!(vm.input_text.get_untracked(), "");
            assert!(vm.pending_attachments.get_untracked().is_empty());
        });
    }
}
