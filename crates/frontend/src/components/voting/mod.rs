mod types;
mod voting_active;
mod voting_create;
mod voting_list;

pub use types::*;
use voting_active::VotingActive;
use voting_create::VotingCreate;
use voting_list::VotingList;

use crate::components::draggable_window::DraggableWindow;
use crate::components::tab_bar::{TabBar, TabItem};
use crate::config::Theme;
use crate::i18n::i18n::{t_string, use_i18n};
use leptos::prelude::*;
use shared::events::voting::VotingOptionResult;
use shared::events::{ClientEvent, VotingResultPayload, VotingStartPayload};
use std::collections::{HashMap, HashSet};

#[component]
pub fn VotingWindow(
    show_voting_window: RwSignal<bool>,
    votings: RwSignal<HashMap<String, VotingState>>,
    voted_in: RwSignal<HashSet<String>>,
    username: ReadSignal<String>,
    ws_sender: ReadSignal<Option<super::websocket::WsSender>>,
    on_create_voting: impl Fn(VotingStartPayload) + 'static + Copy + Send + Sync,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let active_tab = RwSignal::new(VotingTab::List);
    // Хранилище выбранных опций для каждого голосования: voting_id -> HashSet<option_id>
    let selected_options_map = RwSignal::new(HashMap::<String, HashSet<String>>::new());
    // Список открытых табов голосований: voting_id -> question
    let open_voting_tabs = RwSignal::new(HashMap::<String, String>::new());

    let switch_to_tab = move |tab: VotingTab| {
        active_tab.set(tab);
    };

    let close_voting_tab = move |voting_id: String| {
        // Очищаем выбранные опции для этого голосования
        selected_options_map.update(|map| {
            map.remove(&voting_id);
        });
        // Удаляем таб из списка открытых
        open_voting_tabs.update(|tabs| {
            tabs.remove(&voting_id);
        });
        // Если голосование завершено (в состоянии Results), удаляем его из votings
        votings.update(|map| {
            if let Some(state) = map.get(&voting_id) {
                if matches!(state, VotingState::Results { .. }) {
                    map.remove(&voting_id);
                }
            }
        });
        // Переключаемся на список
        switch_to_tab(VotingTab::List);
    };

    // Effect для автоматического завершения голосований
    Effect::new(move |_| {
        let votings_snapshot = votings.get();
        for (voting_id, state) in votings_snapshot.iter() {
            if let VotingState::Active {
                voting,
                participants,
                votes,
                remaining_seconds,
            } = state
            {
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
                                voters_map
                                    .entry(option_id.clone())
                                    .or_default()
                                    .push(user.clone());
                            }
                        }

                        let results: Vec<VotingOptionResult> = voting
                            .options
                            .iter()
                            .map(|opt| VotingOptionResult {
                                option_id: opt.id.clone(),
                                count: *results_map.get(&opt.id).unwrap_or(&0),
                                voters: if !voting.is_anonymous {
                                    voters_map.get(&opt.id).cloned()
                                } else {
                                    None
                                },
                            })
                            .collect();

                        let result_payload = VotingResultPayload {
                            voting_id: voting_id.clone(),
                            question: voting.question.clone(),
                            options: voting.options.clone(),
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
                            let end_event =
                                ClientEvent::VotingEnd(shared::events::VotingEndPayload {
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

    let all_tabs = move || {
        let mut tabs = vec![
            TabItem::new(VotingTab::List, t_string!(i18n, voting.tab_list)),
            TabItem::new(VotingTab::Create, t_string!(i18n, voting.tab_create)),
        ];

        // Add tabs for open votings (независимо от их состояния)
        for (voting_id, question) in open_voting_tabs.get().iter() {
            tabs.push(
                TabItem::new(VotingTab::Voting(voting_id.clone()), question.clone()).closable(),
            );
        }

        tabs
    };

    view! {
        <DraggableWindow
            is_open=show_voting_window
            title=move || t_string!(i18n, voting.title)
            initial_x=100
            initial_y=100
            is_active=is_active
            on_focus=on_focus.unwrap_or_else(|| Callback::new(|_| {}))
            theme=theme.clone()
        >
            <div style="display: flex; flex-direction: column; height: 100%;">
                // Tab navigation
                <TabBar
                    tabs=all_tabs
                    active_tab=active_tab
                    theme=theme.clone()
                    on_close=move |tab: VotingTab| {
                        // When closing a voting tab, clean up and switch to List tab
                        if let VotingTab::Voting(voting_id) = tab {
                            close_voting_tab(voting_id);
                        }
                    }
                />

                // Tab content
                <div style="flex: 1; overflow-y: auto; padding: 0.25rem 1rem 4rem 1rem;">
                    {move || {
                        match active_tab.get() {
                            VotingTab::List => view! {
                                <VotingList
                                    votings=votings
                                    active_tab=active_tab
                                    open_voting_tabs=open_voting_tabs
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
                                view! {
                                    <VotingActive
                                        voting_id=voting_id.clone()
                                        voting=votings
                                        username=username
                                        ws_sender=ws_sender
                                        voted_in=voted_in
                                        selected_options_map=selected_options_map
                                        theme=theme.clone()
                                    />
                                }.into_any()
                            }
                        }
                    }}
                </div>
            </div>
        </DraggableWindow>
    }
}
