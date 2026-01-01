use crate::components::websocket::CursorSignals;
use crate::config;
use leptos::prelude::*;
use shared::events::{ClientEvent, MouseClickPayload, mouse::MouseEventTypeEnum};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen_futures::spawn_local;

use super::AppState;
use super::super::websocket::WsSender;

/// Обрабатывает движение мыши и отправляет координаты через WebSocket
pub fn create_mouse_move_handler(
    app_state: ReadSignal<AppState>,
    username: ReadSignal<String>,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    ws_sender: ReadSignal<Option<WsSender>>,
    cfg: StoredValue<config::Config>,
) -> impl Fn(leptos::web_sys::MouseEvent) + Clone {
    move |ev: leptos::web_sys::MouseEvent| {
        if app_state.get() != AppState::Connected {
            return;
        }

        let x = ev.client_x();
        let y = ev.client_y();
        let user = username.get();

        // Обновляем локальный курсор
        update_local_cursor(&user, x, y, set_cursors);

        // Отправляем событие через WebSocket с троттлингом
        send_mouse_event_throttled(x, y, user, ws_sender, cfg);
    }
}

/// Обновляет позицию локального курсора в мапе
fn update_local_cursor(
    user: &str,
    x: i32,
    y: i32,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
) {
    set_cursors.update(|map| {
        if let Some(cursor_signals) = map.get(user) {
            cursor_signals.set_x.set(x);
            cursor_signals.set_y.set(y);
        } else {
            let (rx_x, tx_x) = signal(x);
            let (rx_y, tx_y) = signal(y);
            map.insert(
                user.to_string(),
                CursorSignals {
                    x: rx_x,
                    set_x: tx_x,
                    y: rx_y,
                    set_y: tx_y,
                },
            );
        }
    });
}

/// Отправляет событие движения мыши через WebSocket с троттлингом
fn send_mouse_event_throttled(
    x: i32,
    y: i32,
    user: String,
    ws_sender: ReadSignal<Option<WsSender>>,
    cfg: StoredValue<config::Config>,
) {
    let event = ClientEvent::MouseClickPayload(MouseClickPayload {
        x,
        y,
        mouse_event_type: MouseEventTypeEnum::Move,
        user_id: user,
    });

    thread_local! {
        static IS_THROTTLED: AtomicBool = AtomicBool::new(false);
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
        if let Some(mut sender) = ws_sender.get() {
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
            }
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
