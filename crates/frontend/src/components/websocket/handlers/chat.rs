use crate::components::statistics::StateEvent;
use crate::components::websocket::{storage, utils};
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, RoomState};
use std::cell::RefCell;
use std::rc::Rc;

pub fn handle_chat_message(
    msg: ChatMessagePayload,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    my_username: &str,
    room_name: &str,
    messages_signal: RwSignal<Vec<ChatMessagePayload>>,
    state_events: RwSignal<Vec<StateEvent>>,
    has_chat_notification: RwSignal<bool>,
    chat_notification_count: RwSignal<u32>,
) {
    log!("Processing ChatMessage from {}", msg.username);

    let is_from_me = msg.username == my_username;

    // Если сообщение не от текущего пользователя, увеличиваем счётчик уведомлений
    if !is_from_me {
        has_chat_notification.set(true);
        chat_notification_count.update(|count| *count += 1);
    }

    // Обновляем state и получаем новую версию
    let current_ver = {
        let mut state = room_state.borrow_mut();
        state.chat_history.push(msg.clone());
        state.commit_changes();
        state.version
    };

    *local_version.borrow_mut() = current_ver;

    // Обновляем last_synced_version только если сообщение пришло из сети
    if !is_from_me {
        *last_synced_version.borrow_mut() = current_ver;
    }

    storage::save_state(room_name, &room_state.borrow());

    utils::log_event(
        state_events,
        current_ver,
        "CHAT_MESSAGE",
        &format!("{}: {}", msg.username, msg.payload),
    );

    // ВАЖНО: Берём обновлённый chat_history из room_state, а не из старого сигнала
    messages_signal.set(room_state.borrow().chat_history.clone());
}
