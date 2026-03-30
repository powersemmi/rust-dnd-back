use crate::components::websocket::{storage, utils};
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::ChatMessagePayload;

use super::HandlerContext;

pub fn handle_chat_message(msg: ChatMessagePayload, ctx: &HandlerContext<'_>) {
    log!("Processing ChatMessage from {}", msg.username);

    let is_from_me = msg.username == ctx.my_username;

    // Если сообщение не от текущего пользователя, увеличиваем счётчик уведомлений
    if !is_from_me {
        ctx.has_chat_notification.set(true);
        ctx.chat_notification_count.update(|count| *count += 1);
    }

    // Обновляем state и получаем новую версию
    let current_ver = {
        let mut state = ctx.room_state.borrow_mut();
        state.chat_history.push(msg.clone());
        state.commit_changes();
        state.version
    };

    *ctx.local_version.borrow_mut() = current_ver;

    // Обновляем last_synced_version только если сообщение пришло из сети
    if !is_from_me {
        *ctx.last_synced_version.borrow_mut() = current_ver;
    }

    storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());

    utils::log_event(
        ctx.state_events,
        current_ver,
        "CHAT_MESSAGE",
        &format!("{}: {}", msg.username, msg.payload),
    );

    // ВАЖНО: Берём обновлённый chat_history из room_state, а не из старого сигнала
    ctx.messages_signal
        .set(ctx.room_state.borrow().chat_history.clone());
}
