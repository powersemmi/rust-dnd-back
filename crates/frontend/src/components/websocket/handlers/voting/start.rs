use crate::components::statistics::StateEvent;
use crate::components::voting::VotingState;
use crate::components::websocket::{WsSender, utils};
use gloo_net::websocket::Message;
use gloo_timers::future::TimeoutFuture;
use js_sys;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{ClientEvent, voting::VotingStartPayload};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn handle_voting_start(
    payload: VotingStartPayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    tx: &WsSender,
    my_username: &str,
    local_version: &Rc<RefCell<u64>>,
    state_events: RwSignal<Vec<StateEvent>>,
    has_statistics_notification: RwSignal<bool>,
    notification_count: RwSignal<u32>,
) {
    log!(
        "Voting started: {} (id: {})",
        payload.question,
        payload.voting_id
    );
    let voting_id = payload.voting_id.clone();
    let timer_seconds = payload.timer_seconds;

    // Устанавливаем уведомление о новом голосовании и увеличиваем счётчик
    has_statistics_notification.set(true);
    notification_count.update(|count| *count += 1);

    // Отправляем presence response
    let request_id = format!("voting_{}", voting_id);
    let response = ClientEvent::PresenceResponse(shared::events::PresenceResponsePayload {
        request_id,
        user: my_username.to_string(),
    });
    if let Ok(json) = serde_json::to_string(&response) {
        let _ = tx.clone().try_send(Message::Text(json));
    }

    votings.update(|map| {
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
        state_events,
        *local_version.borrow(),
        "VOTING_START",
        &format!(
            "Voting started: {} (by {})",
            payload.question, payload.creator
        ),
    );

    // Запускаем таймер если есть
    if let Some(seconds) = timer_seconds {
        let voting_id_timer = voting_id.clone();
        spawn_local(async move {
            let mut remaining = seconds;
            while remaining > 0 {
                TimeoutFuture::new(1000).await;
                remaining -= 1;
                votings.update(|map| {
                    if let Some(VotingState::Active {
                        remaining_seconds, ..
                    }) = map.get_mut(&voting_id_timer)
                    {
                        *remaining_seconds = Some(remaining);
                    }
                });
            }
        });
    }
}
