use crate::components::websocket::{self, storage, utils};
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{NoteDeletePayload, NotePayload, NoteVisibility};

use super::HandlerContext;

fn sort_notes(notes: &mut [NotePayload]) {
    notes.sort_by(|left, right| {
        right
            .updated_at_ms
            .partial_cmp(&left.updated_at_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn upsert_note(notes: &mut Vec<NotePayload>, note: NotePayload) {
    match notes.iter_mut().find(|existing| existing.id == note.id) {
        Some(existing) => *existing = note,
        None => notes.push(note),
    }
    sort_notes(notes);
}

fn remove_note(notes: &mut Vec<NotePayload>, id: &str) {
    notes.retain(|note| note.id != id);
}

fn is_direct_note_for_current_user(note: &NotePayload, current_username: &str) -> bool {
    match &note.visibility {
        NoteVisibility::Direct(recipient) => {
            note.author == current_username || recipient == current_username
        }
        _ => false,
    }
}

fn is_direct_delete_for_current_user(payload: &NoteDeletePayload, current_username: &str) -> bool {
    match &payload.visibility {
        NoteVisibility::Direct(recipient) => {
            payload.author == current_username || recipient == current_username
        }
        _ => false,
    }
}

pub fn handle_note_upsert(payload: NotePayload, ctx: &HandlerContext<'_>) {
    match &payload.visibility {
        NoteVisibility::Public => {
            let current_ver =
                {
                    let mut state = ctx.room_state.borrow_mut();
                    if state.public_notes.iter().any(|existing| {
                        existing.id == payload.id && existing.author != payload.author
                    }) {
                        return;
                    }
                    upsert_note(&mut state.public_notes, payload.clone());
                    state.commit_changes();
                    state.version
                };

            *ctx.local_version.borrow_mut() = current_ver;
            if payload.author != ctx.my_username {
                *ctx.last_synced_version.borrow_mut() = current_ver;
            }

            ctx.public_notes_signal
                .set(ctx.room_state.borrow().public_notes.clone());
            storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());
            utils::log_event(
                ctx.state_events,
                current_ver,
                "NOTE_UPSERT",
                &format!("{} updated note '{}'", payload.author, payload.title),
            );
        }
        NoteVisibility::Direct(_) => {
            if !is_direct_note_for_current_user(&payload, ctx.my_username) {
                return;
            }

            ctx.direct_notes_signal.update(|notes| {
                upsert_note(notes, payload.clone());
            });

            let room_name = ctx.room_name.to_string();
            let owner = ctx.my_username.to_string();
            spawn_local(async move {
                let _ = websocket::save_note(
                    &room_name,
                    &owner,
                    websocket::StoredNoteBucket::Direct,
                    &payload,
                )
                .await;
            });
        }
        NoteVisibility::Private => {}
    }
}

pub fn handle_note_delete(payload: NoteDeletePayload, ctx: &HandlerContext<'_>) {
    match &payload.visibility {
        NoteVisibility::Public => {
            let current_ver = {
                let mut state = ctx.room_state.borrow_mut();
                if state
                    .public_notes
                    .iter()
                    .find(|existing| existing.id == payload.id)
                    .is_some_and(|existing| existing.author != payload.author)
                {
                    return;
                }
                if !state
                    .public_notes
                    .iter()
                    .any(|existing| existing.id == payload.id)
                {
                    return;
                }
                remove_note(&mut state.public_notes, &payload.id);
                state.commit_changes();
                state.version
            };

            *ctx.local_version.borrow_mut() = current_ver;
            if payload.author != ctx.my_username {
                *ctx.last_synced_version.borrow_mut() = current_ver;
            }

            ctx.public_notes_signal
                .set(ctx.room_state.borrow().public_notes.clone());
            storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());
            utils::log_event(
                ctx.state_events,
                current_ver,
                "NOTE_DELETE",
                &format!("{} deleted note {}", payload.author, payload.id),
            );
        }
        NoteVisibility::Direct(_) => {
            if !is_direct_delete_for_current_user(&payload, ctx.my_username) {
                return;
            }

            ctx.direct_notes_signal.update(|notes| {
                remove_note(notes, &payload.id);
            });

            let room_name = ctx.room_name.to_string();
            let owner = ctx.my_username.to_string();
            let note_id = payload.id.clone();
            spawn_local(async move {
                let _ = websocket::delete_note(
                    &room_name,
                    &owner,
                    websocket::StoredNoteBucket::Direct,
                    &note_id,
                )
                .await;
            });
        }
        NoteVisibility::Private => {}
    }
}
