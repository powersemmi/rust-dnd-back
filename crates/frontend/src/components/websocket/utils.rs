use crate::components::statistics::StateEvent;
use js_sys;
use leptos::prelude::*;

pub fn log_event(
    state_events: RwSignal<Vec<StateEvent>>,
    version: u64,
    event_type: &str,
    description: &str,
) {
    let timestamp = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    state_events.update(|events| {
        events.push(StateEvent {
            version,
            event_type: event_type.to_string(),
            description: description.to_string(),
            timestamp,
        });
    });
}
