use ammonia::Builder;
use pulldown_cmark::{Options, Parser, html};
use shared::events::{NotePayload, NoteVisibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotesTab {
    Public,
    Private,
    Direct,
}

pub const BOARD_NOTE_DRAG_MIME: &str = "application/x-dnd-note";
pub const DIRECT_RECIPIENT_CACHE_TTL_MS: f64 = 5.0 * 60.0 * 1000.0;

pub fn recipients_cache_is_stale(cached_at_ms: Option<f64>, now_ms: f64) -> bool {
    match cached_at_ms {
        Some(cached_at_ms) => now_ms - cached_at_ms >= DIRECT_RECIPIENT_CACHE_TTL_MS,
        None => true,
    }
}

pub fn tab_for_visibility(visibility: &NoteVisibility) -> NotesTab {
    match visibility {
        NoteVisibility::Public => NotesTab::Public,
        NoteVisibility::Private => NotesTab::Private,
        NoteVisibility::Direct(_) => NotesTab::Direct,
    }
}

pub fn can_edit_note(note: &NotePayload, current_username: &str) -> bool {
    note.author == current_username
}

pub fn can_delete_note(note: &NotePayload, current_username: &str) -> bool {
    match &note.visibility {
        NoteVisibility::Public | NoteVisibility::Private => note.author == current_username,
        NoteVisibility::Direct(recipient) => {
            note.author == current_username || recipient == current_username
        }
    }
}

pub fn render_note_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    Builder::default().clean(&html_output).to_string()
}

pub fn note_heading_and_body(markdown: &str) -> (String, String) {
    let lines = markdown.lines().collect::<Vec<_>>();
    let Some((title_index, title_line)) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| !line.trim().is_empty())
    else {
        return (String::new(), String::new());
    };

    let trimmed = title_line.trim();
    let heading = trimmed.trim_start_matches('#').trim();
    let title = if heading.is_empty() {
        trimmed.to_string()
    } else {
        heading.to_string()
    };
    let body = lines
        .into_iter()
        .skip(title_index + 1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    (title, body)
}

pub fn note_title_from_markdown(markdown: &str) -> String {
    let (title, _) = note_heading_and_body(markdown);
    title.chars().take(120).collect()
}

pub fn sort_notes(notes: &mut [NotePayload]) {
    notes.sort_by(|left, right| {
        right
            .updated_at_ms
            .partial_cmp(&left.updated_at_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::events::{NoteBoardPosition, NoteBoardStyle};

    fn note(author: &str) -> NotePayload {
        NotePayload {
            id: "note-1".to_string(),
            author: author.to_string(),
            visibility: NoteVisibility::Public,
            title: "Title".to_string(),
            body: "**bold**".to_string(),
            created_at_ms: 1.0,
            updated_at_ms: 2.0,
            board_position: Some(NoteBoardPosition {
                world_x: 10.0,
                world_y: 20.0,
            }),
            board_style: NoteBoardStyle::default(),
        }
    }

    #[test]
    fn only_author_can_edit() {
        assert!(can_edit_note(&note("gm"), "gm"));
        assert!(!can_edit_note(&note("gm"), "player"));
    }

    #[test]
    fn direct_note_can_be_deleted_by_author_or_recipient() {
        let mut direct_note = note("gm");
        direct_note.visibility = NoteVisibility::Direct("player".into());

        assert!(can_delete_note(&direct_note, "gm"));
        assert!(can_delete_note(&direct_note, "player"));
        assert!(!can_delete_note(&direct_note, "spectator"));
    }

    #[test]
    fn markdown_is_sanitized() {
        let html = render_note_html("hello<script>alert(1)</script>");
        assert!(html.contains("hello"));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("alert(1)</script>"));
    }

    #[test]
    fn heading_and_body_are_split_from_first_non_empty_line() {
        let (title, body) = note_heading_and_body("\n# Title\nBody\nNext");
        assert_eq!(title, "Title");
        assert_eq!(body, "Body\nNext");
    }

    #[test]
    fn tab_selection_matches_visibility() {
        assert_eq!(
            tab_for_visibility(&NoteVisibility::Public),
            NotesTab::Public
        );
        assert_eq!(
            tab_for_visibility(&NoteVisibility::Private),
            NotesTab::Private
        );
        assert_eq!(
            tab_for_visibility(&NoteVisibility::Direct("gm".into())),
            NotesTab::Direct
        );
    }

    #[test]
    fn cache_is_stale_when_missing_or_expired() {
        assert!(recipients_cache_is_stale(None, 10_000.0));
        assert!(!recipients_cache_is_stale(Some(10_000.0), 10_001.0));
        assert!(recipients_cache_is_stale(
            Some(10_000.0),
            10_000.0 + DIRECT_RECIPIENT_CACHE_TTL_MS
        ));
    }
}
