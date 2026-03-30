use super::model::{NotesTab, note_title_from_markdown, tab_for_visibility};
use leptos::prelude::*;
use shared::events::{NoteBoardPosition, NoteBoardStyle, NotePayload, NoteVisibility};
use uuid::Uuid;

fn current_time_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as f64)
            .unwrap_or(0.0)
    }
}

#[derive(Clone, Copy)]
pub struct NotesViewModel {
    pub active_tab: RwSignal<NotesTab>,
    pub body: RwSignal<String>,
    pub recipient: RwSignal<String>,
    pub editing_note_id: RwSignal<Option<String>>,
    editing_created_at_ms: RwSignal<Option<f64>>,
    pub error_message: RwSignal<Option<String>>,
    pub is_loading_recipients: RwSignal<bool>,
}

impl NotesViewModel {
    pub fn new() -> Self {
        Self {
            active_tab: RwSignal::new(NotesTab::Public),
            body: RwSignal::new(String::new()),
            recipient: RwSignal::new(String::new()),
            editing_note_id: RwSignal::new(None),
            editing_created_at_ms: RwSignal::new(None),
            error_message: RwSignal::new(None),
            is_loading_recipients: RwSignal::new(false),
        }
    }

    pub fn reset_form(&self) {
        self.body.set(String::new());
        self.recipient.set(String::new());
        self.editing_note_id.set(None);
        self.editing_created_at_ms.set(None);
        self.error_message.set(None);
        self.is_loading_recipients.set(false);
    }

    pub fn start_edit(&self, note: &NotePayload) {
        self.active_tab.set(tab_for_visibility(&note.visibility));
        self.body.set(note.body.clone());
        self.recipient.set(match &note.visibility {
            NoteVisibility::Direct(recipient) => recipient.clone(),
            _ => String::new(),
        });
        self.editing_note_id.set(Some(note.id.clone()));
        self.editing_created_at_ms.set(Some(note.created_at_ms));
        self.error_message.set(None);
    }

    pub fn build_note(
        &self,
        author: &str,
        board_position: Option<NoteBoardPosition>,
        board_style: NoteBoardStyle,
    ) -> Result<NotePayload, String> {
        let body = self.body.get_untracked().trim().to_string();
        if body.is_empty() {
            return Err("Note body is required".to_string());
        }

        let visibility = match self.active_tab.get_untracked() {
            NotesTab::Public => NoteVisibility::Public,
            NotesTab::Private => NoteVisibility::Private,
            NotesTab::Direct => {
                let recipient = self.recipient.get_untracked().trim().to_string();
                if recipient.is_empty() {
                    return Err("Recipient username is required".to_string());
                }
                NoteVisibility::Direct(recipient)
            }
        };

        let now = current_time_ms();
        Ok(NotePayload {
            id: self
                .editing_note_id
                .get_untracked()
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            author: author.to_string(),
            visibility,
            title: note_title_from_markdown(&body),
            body,
            created_at_ms: self.editing_created_at_ms.get_untracked().unwrap_or(now),
            updated_at_ms: now,
            board_position,
            board_style,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn direct_note_requires_recipient() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = NotesViewModel::new();
            vm.active_tab.set(NotesTab::Direct);
            vm.body.set("hello".into());
            assert!(
                vm.build_note("gm", None, NoteBoardStyle::default())
                    .is_err()
            );
        });
    }

    #[test]
    fn edit_mode_preserves_note_id() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = NotesViewModel::new();
            let note = NotePayload {
                id: "note-1".into(),
                author: "gm".into(),
                visibility: NoteVisibility::Public,
                title: "Body".into(),
                body: "Body".into(),
                created_at_ms: 10.0,
                updated_at_ms: 11.0,
                board_position: None,
                board_style: NoteBoardStyle::default(),
            };
            vm.start_edit(&note);
            let built = vm
                .build_note("gm", None, NoteBoardStyle::default())
                .unwrap();
            assert_eq!(built.id, "note-1");
            assert_eq!(built.created_at_ms, 10.0);
        });
    }
}
