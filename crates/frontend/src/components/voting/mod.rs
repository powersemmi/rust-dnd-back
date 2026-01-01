mod types;
mod voting_active;
mod voting_create;
mod voting_list;
mod voting_results;

pub use types::*;
use voting_active::VotingActive;
use voting_create::VotingCreate;
use voting_list::VotingList;
use voting_results::VotingResults;

use crate::components::draggable_window::DraggableWindow;
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use shared::events::{ClientEvent, VotingResultPayload, VotingStartPayload};
use shared::events::voting::VotingOptionResult;
use std::collections::{HashMap, HashSet};

#[component]
pub fn VotingWindow(
    show_voting_window: RwSignal<bool>,
    votings: RwSignal<HashMap<String, VotingState>>,
    voted_in: RwSignal<HashSet<String>>,
    username: ReadSignal<String>,
    ws_sender: ReadSignal<Option<super::websocket::WsSender>>,
    on_create_voting: impl Fn(VotingStartPayload) + 'static + Copy + Send + Sync,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let active_tab = RwSignal::new(VotingTab::List);
    let selected_options = RwSignal::new(HashSet::<String>::new());

    let switch_to_tab = move |tab: VotingTab| {
        active_tab.set(tab);
    };

    // Effect для автоматического завершения голосований
    Effect::new(move |_| {
        let votings_snapshot = votings.get();
        for (voting_id, state) in votings_snapshot.iter() {
            if let VotingState::Active { voting, participants, votes, remaining_seconds } = state {
                let creator = voting.creator.clone();
                let my_name = username.get();

                // Только создатель голосования завершает его
                if creator == my_name {
                    let total_participants = participants.len();
                    let total_voted = votes.len();

                    // Условие 1: Все проголосовали
                    let all_voted = total_participants > 0 && total_voted == total_participants;

                    // Условие 2: Таймер истёк
                    let timer_expired = *remaining_seconds == Some(0);

                    if all_voted || timer_expired {
                        // Подсчитываем результаты
                        let mut results_map: HashMap<String, u32> = HashMap::new();
                        let mut voters_map: HashMap<String, Vec<String>> = HashMap::new();

                        for (user, option_ids) in votes.iter() {
                            for option_id in option_ids {
                                *results_map.entry(option_id.clone()).or_insert(0) += 1;
                                voters_map.entry(option_id.clone()).or_default().push(user.clone());
                            }
                        }

                        let results: Vec<VotingOptionResult> = voting.options.iter().map(|opt| {
                            VotingOptionResult {
                                option_id: opt.id.clone(),
                                count: *results_map.get(&opt.id).unwrap_or(&0),
                                voters: if !voting.is_anonymous {
                                    voters_map.get(&opt.id).cloned()
                                } else {
                                    None
                                },
                            }
                        }).collect();

                        let result_payload = VotingResultPayload {
                            voting_id: voting_id.clone(),
                            results,
                            total_participants: total_participants as u32,
                            total_voted: total_voted as u32,
                        };

                        // Отправляем результат
                        if let Some(mut sender) = ws_sender.get() {
                            let event = ClientEvent::VotingResult(result_payload);
                            if let Ok(json) = serde_json::to_string(&event) {
                                let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
                            }

                            // Отправляем событие завершения
                            let end_event = ClientEvent::VotingEnd(shared::events::VotingEndPayload {
                                voting_id: voting_id.clone(),
                            });
                            if let Ok(json) = serde_json::to_string(&end_event) {
                                let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
                            }
                        }
                    }
                }
            }
        }
    });

    view! {
        <DraggableWindow
            is_open=show_voting_window
            title=move || t_string!(i18n, voting.title)
            initial_x=100
            initial_y=100
            theme=theme.clone()
        >
            <div style="display: flex; flex-direction: column; height: 100%;">
                // Tab navigation
                <div style=format!("display: flex; gap: 0.625rem; margin-bottom: 1.25rem; border-bottom: 0.125rem solid {}; padding-bottom: 0.625rem;", theme.ui_bg_primary)>
                    <button
                        style=move || {
                            let base = "padding: 0.5rem 1rem; border: none; border-radius: 0.25rem; cursor: pointer; font-size: 0.875rem; font-weight: 500;";
                            if matches!(active_tab.get(), VotingTab::List) {
                                format!("{} background: {}; color: {};", base, theme.ui_button_primary, theme.ui_text_primary)
                            } else {
                                format!("{} background: {}; color: {};", base, theme.ui_bg_secondary, theme.ui_text_secondary)
                            }
                        }
                        on:click=move |_| switch_to_tab(VotingTab::List)
                    >
                        {move || t!(i18n, voting.tab_list)}
                    </button>
                    <button
                        style=move || {
                            let base = "padding: 0.5rem 1rem; border: none; border-radius: 0.25rem; cursor: pointer; font-size: 0.875rem; font-weight: 500;";
                            if matches!(active_tab.get(), VotingTab::Create) {
                                format!("{} background: {}; color: {};", base, theme.ui_button_primary, theme.ui_text_primary)
                            } else {
                                format!("{} background: {}; color: {};", base, theme.ui_bg_secondary, theme.ui_text_secondary)
                            }
                        }
                        on:click=move |_| switch_to_tab(VotingTab::Create)
                    >
                        {move || t!(i18n, voting.tab_create)}
                    </button>

                    // Dynamic tabs for individual votings
                    <For
                        each=move || {
                            votings.get()
                                .iter()
                                .filter(|(_, state)| matches!(state, VotingState::Active { .. }))
                                .map(|(id, state)| {
                                    let voting = match state {
                                        VotingState::Active { voting, .. } => voting.clone(),
                                        _ => unreachable!(),
                                    };
                                    (id.clone(), voting)
                                })
                                .collect::<Vec<_>>()
                        }
                        key=|(id, _)| id.clone()
                        children=move |(voting_id, voting)| {
                            let voting_id_clone = voting_id.clone();
                            view! {
                                <button
                                    style=move || {
                                        let base = "padding: 0.5rem 1rem; border: none; border-radius: 0.25rem; cursor: pointer; font-size: 0.875rem; font-weight: 500; max-width: 9.375rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;";
                                        if matches!(active_tab.get(), VotingTab::Voting(ref id) if id == &voting_id_clone) {
                                            format!("{} background: {}; color: {};", base, theme.ui_button_primary, theme.ui_text_primary)
                                        } else {
                                            format!("{} background: {}; color: {};", base, theme.ui_bg_secondary, theme.ui_text_secondary)
                                        }
                                    }
                                    on:click=move |_| switch_to_tab(VotingTab::Voting(voting_id.clone()))
                                    title=voting.question.clone()
                                >
                                    {voting.question.clone()}
                                </button>
                            }
                        }
                    />
                </div>

                // Tab content
                <div style="flex: 1; overflow-y: auto; padding-right: 0.625rem;">
                    {move || {
                        match active_tab.get() {
                            VotingTab::List => view! {
                                <VotingList
                                    votings=votings
                                    active_tab=active_tab
                                    theme=theme.clone()
                                />
                            }.into_any(),

                            VotingTab::Create => view! {
                                <VotingCreate
                                    theme=theme.clone()
                                    on_create=move |payload| {
                                        on_create_voting(payload);
                                        switch_to_tab(VotingTab::List);
                                    }
                                    on_cancel=move || switch_to_tab(VotingTab::List)
                                />
                            }.into_any(),

                            VotingTab::Voting(voting_id) => {
                                // Check if user has voted
                                if voted_in.get().contains(&voting_id) {
                                    view! {
                                        <VotingResults
                                            voting_id=voting_id
                                            votings=votings
                                            theme=theme.clone()
                                        />
                                    }.into_any()
                                } else {
                                    view! {
                                        <VotingActive
                                            voting_id=voting_id
                                            voting=votings
                                            username=username
                                            ws_sender=ws_sender
                                            voted_in=voted_in
                                            selected_options=selected_options
                                            theme=theme.clone()
                                        />
                                    }.into_any()
                                }
                            }
                        }
                    }}
                </div>
            </div>
        </DraggableWindow>
    }
}
