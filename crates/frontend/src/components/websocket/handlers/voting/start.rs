use crate::components::voting::VotingState;
use crate::components::websocket::utils;
use gloo_timers::future::TimeoutFuture;
use js_sys;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{ClientEvent, voting::VotingStartPayload};
use std::collections::HashMap;

use super::super::HandlerContext;

pub fn handle_voting_start(payload: VotingStartPayload, ctx: &HandlerContext<'_>) {
    log!(
        "Voting started: {} (id: {})",
        payload.question,
        payload.voting_id
    );
    let voting_id = payload.voting_id.clone();
    let timer_seconds = payload.timer_seconds;

    // Устанавливаем уведомление о новом голосовании и увеличиваем счётчик
    ctx.has_statistics_notification.set(true);
    ctx.notification_count.update(|count| *count += 1);

    // Отправляем presence response
    let request_id = format!("voting_{}", voting_id);
    let response = ClientEvent::PresenceResponse(shared::events::PresenceResponsePayload {
        request_id,
        user: ctx.my_username.to_string(),
    });
    let _ = ctx.tx.try_send_event(response);

    ctx.votings.update(|map| {
        log!(
            "Adding voting {} to votings map (current size: {})",
            voting_id,
            map.len()
        );
        map.insert(
            voting_id.clone(),
            VotingState::Active {
                voting: payload.clone(),
                participants: vec![],
                votes: HashMap::new(),
                remaining_seconds: timer_seconds,
                created_at: js_sys::Date::now(),
            },
        );
        log!("Votings map size after insert: {}", map.len());
    });

    utils::log_event(
        ctx.state_events,
        *ctx.local_version.borrow(),
        "VOTING_START",
        &format!(
            "Voting started: {} (by {})",
            payload.question, payload.creator
        ),
    );

    // Запускаем таймер если есть
    if let Some(seconds) = timer_seconds {
        let voting_id_timer = voting_id.clone();
        let votings = ctx.votings;
        spawn_local(async move {
            let mut remaining = seconds as i32;
            // Считаем до 0, затем ещё 5 секунд ожидания (отрицательные значения)
            let grace_period = -5i32;

            while remaining > grace_period {
                TimeoutFuture::new(1000).await;
                remaining -= 1;

                votings.update(|map| {
                    if let Some(VotingState::Active {
                        remaining_seconds, ..
                    }) = map.get_mut(&voting_id_timer)
                    {
                        // Показываем 0 если уже в grace period, иначе показываем оставшееся время
                        *remaining_seconds =
                            Some(if remaining >= 0 { remaining as u32 } else { 0 });
                    }
                });
            }

            log!(
                "Voting {} timer finished (including grace period)",
                voting_id_timer
            );
        });
    }
}
