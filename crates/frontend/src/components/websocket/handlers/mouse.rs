use crate::components::websocket::types::CursorSignals;
use leptos::prelude::*;
use std::collections::HashMap;

pub fn handle_mouse_event(
    mouse_event: shared::events::MouseClickPayload,
    my_username: &str,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
) {
    if mouse_event.user_id == my_username {
        return;
    }

    leptos::logging::log!(
        "🖱️ [MOUSE] Received cursor from {}: ({}, {})",
        mouse_event.user_id,
        mouse_event.x,
        mouse_event.y
    );

    set_cursors.update(|cursors| {
        if let Some(cursor_signals) = cursors.get(&mouse_event.user_id) {
            cursor_signals.set_x.set(mouse_event.x);
            cursor_signals.set_y.set(mouse_event.y);
        } else {
            leptos::logging::log!(
                "🖱️ [MOUSE] Creating new cursor for: {}",
                mouse_event.user_id
            );
            let (x, set_x) = signal(mouse_event.x);
            let (y, set_y) = signal(mouse_event.y);
            cursors.insert(
                mouse_event.user_id.clone(),
                CursorSignals { x, set_x, y, set_y },
            );
        }
    });
}
