use crate::components::voting::VotingState;
use crate::components::websocket::WsSender;
use gloo_timers::future::TimeoutFuture;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::{ClientEvent, SyncSnapshotRequestPayload, VotingResultPayload};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn handle_conflict_voting_result(
    payload: VotingResultPayload,
    votings: RwSignal<HashMap<String, VotingState>>,
    tx: &WsSender,
    my_username: &str,
    expected_snapshot_from: &Rc<RefCell<Option<String>>>,
) {
    log!(
        "🔍 This is a conflict resolution voting: {}",
        payload.voting_id
    );

    // Ищем результат для опции ".1" (Да)
    let yes_votes = payload
        .results
        .iter()
        .find(|r| r.option_id == ".1")
        .map(|r| r.count)
        .unwrap_or(0);

    // Ищем результат для опции ".0" (Нет)
    let no_votes = payload
        .results
        .iter()
        .find(|r| r.option_id == ".0")
        .map(|r| r.count)
        .unwrap_or(0);

    log!(
        "📊 Conflict voting result: Yes={}, No={}",
        yes_votes,
        no_votes
    );
    log!("📋 Results details: {:?}", payload.results);

    // Если большинство проголосовало "Да", запрашиваем snapshot у создателя
    if yes_votes > no_votes {
        log!("✅ Conflict resolved: requesting snapshot from voting creator");

        // Получаем создателя голосования и список проголосовавших
        let found_creator = votings.with(|map| {
            log!("🔎 Looking for voting {} in votings map (size: {})", payload.voting_id, map.len());
            if let Some(state) = map.get(&payload.voting_id) {
                log!("✓ Found voting state");
                match state {
                    VotingState::Results { voting, .. } => {
                        let creator = voting.creator.clone();

                        // Собираем всех проголосовавших (исключая создателя)
                        let mut voters: Vec<String> = payload.results.iter()
                            .flat_map(|r| r.voters.clone().unwrap_or_default())
                            .filter(|voter| voter != &creator)
                            .collect();
                        voters.sort();
                        voters.dedup();

                        log!("👤 Voting creator: {}, voters: {:?}", creator, voters);

                        // Все пользователи ожидают снапшот от создателя
                        *expected_snapshot_from.borrow_mut() = Some(creator.clone());
                        log!("🔔 Set expected_snapshot_from to: {}", creator);

                        // Первый пользователь из списка запрашивает snapshot
                        if let Some(first_voter) = voters.first() {
                            if my_username == first_voter {
                                log!("🙋 I am the first voter ({}), requesting snapshot from creator {}", my_username, creator);

                                let tx_clone = tx.clone();
                                let creator_clone = creator.clone();
                                spawn_local(async move {
                                    // Небольшая задержка для стабилизации
                                    log!("⏳ Waiting 500ms before requesting snapshot...");
                                    TimeoutFuture::new(500).await;

                                    let req = ClientEvent::SyncSnapshotRequest(SyncSnapshotRequestPayload {
                                        target_username: creator_clone.clone(),
                                    });
                                    log!("📤 Sending SyncSnapshotRequest to {}", creator_clone);
                                    let _ = tx_clone.try_send_event(req);
                                });
                            } else {
                                log!("👥 First voter is {}, I am {}, waiting for snapshot", first_voter, my_username);
                            }
                        } else {
                            log!("⚠️ No voters found (excluding creator)!");
                        }
                        Some(creator)
                    }
                    _ => {
                        log!("⚠️ Voting state is not Results yet");
                        None
                    }
                }
            } else {
                log!("❌ Voting {} not found in votings map!", payload.voting_id);
                None
            }
        });

        if found_creator.is_none() {
            log!("💥 Failed to process conflict resolution - voting not found");
        }
    } else {
        log!("❌ Conflict voting rejected: keeping current version");
    }
}
