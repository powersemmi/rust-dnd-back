use crate::components::websocket::CursorSignals;
use crate::config;
use leptos::prelude::*;
use shared::events::{ClientEvent, MouseClickPayload, mouse::MouseEventTypeEnum};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen_futures::spawn_local;

use super::super::websocket::WsSender;

pub fn update_local_cursor_world(
    user: &str,
    x: f64,
    y: f64,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
) {
    let now = js_sys::Date::now();

    set_cursors.update(|map| {
        if let Some(cursor_signals) = map.get(user) {
            cursor_signals.set_x.set(x);
            cursor_signals.set_y.set(y);
            cursor_signals.set_last_activity.set(now);
        } else {
            let (rx_x, tx_x) = signal(x);
            let (rx_y, tx_y) = signal(y);
            let (last_activity, set_last_activity) = signal(now);
            let (visible, set_visible) = signal(false);
            map.insert(
                user.to_string(),
                CursorSignals {
                    x: rx_x,
                    set_x: tx_x,
                    y: rx_y,
                    set_y: tx_y,
                    last_activity,
                    set_last_activity,
                    visible,
                    set_visible,
                },
            );
        }
    });
}

pub fn send_mouse_event_throttled(
    x: f64,
    y: f64,
    user: String,
    ws_sender: ReadSignal<Option<WsSender>>,
    cfg: StoredValue<config::Config>,
) {
    let event = ClientEvent::MouseClickPayload(MouseClickPayload {
        x,
        y,
        mouse_event_type: MouseEventTypeEnum::Move,
        user_id: user.clone(),
    });

    thread_local! {
        static IS_THROTTLED: AtomicBool = const { AtomicBool::new(false) };
    }

    let should_send = IS_THROTTLED.with(|throttled| {
        if !throttled.load(Ordering::Relaxed) {
            throttled.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    });

    if should_send {
        if let Some(sender) = ws_sender.get() {
            let _ = sender.try_send_event(event);
        }

        let throttle_ms = cfg.get_value().theme.mouse_throttle_ms;
        spawn_local(async move {
            gloo_timers::future::sleep(std::time::Duration::from_millis(throttle_ms)).await;
            IS_THROTTLED.with(|throttled| {
                throttled.store(false, Ordering::Relaxed);
            });
        });
    }
}
