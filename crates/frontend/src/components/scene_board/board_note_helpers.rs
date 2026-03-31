// Pure helper functions for board-note CRUD operations and display logic.
// These functions have no dependencies on Leptos view macros or DOM.

use super::interaction_state::{
    BoardNoteEditorDraft, BoardNoteSelection, BOARD_NOTE_TOOLBAR_HEIGHT_PX,
};
use crate::components::notes::model::note_title_from_markdown;
use crate::components::websocket::{StoredNoteBucket, WsSender, delete_note, save_note};
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{ClientEvent, NoteDeletePayload, NotePayload, NoteVisibility};

pub fn current_time_ms() -> f64 {
    js_sys::Date::now()
}

pub fn note_matches(note: &NotePayload, note_id: &str, visibility: &NoteVisibility) -> bool {
    note.id == note_id && &note.visibility == visibility
}

pub fn upsert_note(notes: &mut Vec<NotePayload>, note: NotePayload) {
    match notes.iter_mut().find(|existing| existing.id == note.id) {
        Some(existing) => *existing = note,
        None => notes.push(note),
    }
    notes.sort_by(|left, right| {
        right
            .updated_at_ms
            .partial_cmp(&left.updated_at_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
}

pub fn remove_note(notes: &mut Vec<NotePayload>, note_id: &str) {
    notes.retain(|note| note.id != note_id);
}

pub fn collect_board_notes(
    public_notes: &[NotePayload],
    private_notes: &[NotePayload],
    direct_notes: &[NotePayload],
) -> Vec<NotePayload> {
    public_notes
        .iter()
        .chain(private_notes.iter())
        .chain(direct_notes.iter())
        .filter(|note| note.board_position.is_some())
        .cloned()
        .collect()
}

pub fn find_matching_note(
    note: &NotePayload,
    public_notes: &[NotePayload],
    private_notes: &[NotePayload],
    direct_notes: &[NotePayload],
) -> Option<NotePayload> {
    public_notes
        .iter()
        .chain(private_notes.iter())
        .chain(direct_notes.iter())
        .find(|existing| existing.id == note.id && existing.visibility == note.visibility)
        .cloned()
}

pub fn find_note_by_ref(
    note_id: &str,
    visibility: &NoteVisibility,
    public_notes: &[NotePayload],
    private_notes: &[NotePayload],
    direct_notes: &[NotePayload],
) -> Option<NotePayload> {
    public_notes
        .iter()
        .chain(private_notes.iter())
        .chain(direct_notes.iter())
        .find(|note| note_matches(note, note_id, visibility))
        .cloned()
}

pub fn apply_local_note_upsert(
    public_notes: RwSignal<Vec<NotePayload>>,
    private_notes: RwSignal<Vec<NotePayload>>,
    direct_notes: RwSignal<Vec<NotePayload>>,
    note: NotePayload,
) {
    match note.visibility.clone() {
        NoteVisibility::Public => public_notes.update(|notes| upsert_note(notes, note.clone())),
        NoteVisibility::Private => private_notes.update(|notes| upsert_note(notes, note.clone())),
        NoteVisibility::Direct(_) => direct_notes.update(|notes| upsert_note(notes, note.clone())),
    }
}

pub fn persist_note_upsert(
    ws_sender: &ReadSignal<Option<WsSender>>,
    room_id: &ReadSignal<String>,
    username: &ReadSignal<String>,
    note: NotePayload,
) {
    match note.visibility.clone() {
        NoteVisibility::Public | NoteVisibility::Direct(_) => {
            send_ws_event(ws_sender, ClientEvent::NoteUpsert(note));
        }
        NoteVisibility::Private => {
            let current_room = room_id.get_untracked();
            let current_user = username.get_untracked();
            spawn_local(async move {
                let _ = save_note(
                    &current_room,
                    &current_user,
                    StoredNoteBucket::Private,
                    &note,
                )
                .await;
            });
        }
    }
}

pub fn apply_local_note_delete(
    public_notes: RwSignal<Vec<NotePayload>>,
    private_notes: RwSignal<Vec<NotePayload>>,
    direct_notes: RwSignal<Vec<NotePayload>>,
    note_id: &str,
    visibility: &NoteVisibility,
) {
    match visibility {
        NoteVisibility::Public => public_notes.update(|notes| remove_note(notes, note_id)),
        NoteVisibility::Private => private_notes.update(|notes| remove_note(notes, note_id)),
        NoteVisibility::Direct(_) => direct_notes.update(|notes| remove_note(notes, note_id)),
    }
}

pub fn persist_note_delete(
    ws_sender: &ReadSignal<Option<WsSender>>,
    room_id: &ReadSignal<String>,
    username: &ReadSignal<String>,
    note: &NotePayload,
) {
    match &note.visibility {
        NoteVisibility::Public => {
            send_ws_event(
                ws_sender,
                ClientEvent::NoteDelete(NoteDeletePayload {
                    id: note.id.clone(),
                    author: note.author.clone(),
                    visibility: note.visibility.clone(),
                }),
            );
        }
        NoteVisibility::Private => {
            let current_room = room_id.get_untracked();
            let current_user = username.get_untracked();
            let note_id = note.id.clone();
            spawn_local(async move {
                let _ = delete_note(
                    &current_room,
                    &current_user,
                    StoredNoteBucket::Private,
                    &note_id,
                )
                .await;
            });
        }
        NoteVisibility::Direct(_) => {
            let current_user = username.get_untracked();
            if note.author == current_user {
                send_ws_event(
                    ws_sender,
                    ClientEvent::NoteDelete(NoteDeletePayload {
                        id: note.id.clone(),
                        author: note.author.clone(),
                        visibility: note.visibility.clone(),
                    }),
                );
            } else {
                let current_room = room_id.get_untracked();
                let note_id = note.id.clone();
                spawn_local(async move {
                    let _ = delete_note(
                        &current_room,
                        &current_user,
                        StoredNoteBucket::Direct,
                        &note_id,
                    )
                    .await;
                });
            }
        }
    }
}

pub fn board_note_body_height(
    style: &shared::events::NoteBoardStyle,
    _is_selected: bool,
    is_editing: bool,
) -> f64 {
    let controls_height = if is_editing {
        BOARD_NOTE_TOOLBAR_HEIGHT_PX
    } else {
        0.0
    };
    (style.height_px - controls_height - 24.0).max(70.0)
}

pub fn board_note_meta(note: &NotePayload, current_username: &str) -> String {
    match &note.visibility {
        NoteVisibility::Public => format!("@{}", note.author),
        NoteVisibility::Private => format!("@{} | private", note.author),
        NoteVisibility::Direct(recipient) if note.author == current_username => {
            format!("@{} -> @{}", note.author, recipient)
        }
        NoteVisibility::Direct(_) => format!("@{} -> you", note.author),
    }
}

pub fn commit_board_note_draft(
    draft: &BoardNoteEditorDraft,
    board_note_editor_error: RwSignal<Option<String>>,
    public_notes: RwSignal<Vec<NotePayload>>,
    private_notes: RwSignal<Vec<NotePayload>>,
    direct_notes: RwSignal<Vec<NotePayload>>,
    ws_sender: &ReadSignal<Option<WsSender>>,
    room_id: &ReadSignal<String>,
    username: &ReadSignal<String>,
) -> bool {
    let body = draft.body.trim().to_string();
    if body.is_empty() {
        board_note_editor_error.set(Some("Note body is required".to_string()));
        return false;
    }

    let Some(mut updated_note) = find_note_by_ref(
        &draft.note_id,
        &draft.visibility,
        &public_notes.get_untracked(),
        &private_notes.get_untracked(),
        &direct_notes.get_untracked(),
    ) else {
        return false;
    };

    updated_note.title = note_title_from_markdown(&body);
    updated_note.body = body;
    updated_note.updated_at_ms = current_time_ms();
    apply_local_note_upsert(
        public_notes,
        private_notes,
        direct_notes,
        updated_note.clone(),
    );
    persist_note_upsert(ws_sender, room_id, username, updated_note);
    board_note_editor_error.set(None);
    true
}

pub fn board_note_title_font_size_pt(font_size_pt: f64) -> f64 {
    (font_size_pt * 1.2).clamp(10.0, 96.0)
}

pub fn clear_board_note_editor_state(
    board_note_editor: RwSignal<Option<BoardNoteEditorDraft>>,
    board_note_editor_error: RwSignal<Option<String>>,
    board_note_focus_request: RwSignal<Option<BoardNoteSelection>>,
) {
    board_note_editor.set(None);
    board_note_editor_error.set(None);
    board_note_focus_request.set(None);
}

/// Thin wrapper: sends a WS event if the sender is connected.
pub fn send_ws_event(ws_sender: &ReadSignal<Option<WsSender>>, event: ClientEvent) {
    if let Some(sender) = ws_sender.get_untracked() {
        let _ = sender.try_send_event(event);
    }
}
