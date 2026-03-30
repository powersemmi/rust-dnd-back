mod types;
mod voting_active;
pub mod voting_active_model;
pub mod voting_completion;
mod voting_create;
pub mod voting_create_view_model;
mod voting_list;

pub use types::*;
pub use voting_active::VotingActive;
pub use voting_completion::{check_should_complete, compute_results};
use voting_create::VotingCreate;
use voting_list::VotingList;

use crate::components::draggable_window::DraggableWindow;
use crate::components::tab_bar::{TabBar, TabItem};
use crate::config::Theme;
use crate::i18n::i18n::{t_string, use_i18n};
use leptos::prelude::*;
use shared::events::{ClientEvent, VotingStartPayload};
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
    let selected_options_map = RwSignal::new(HashMap::<String, HashSet<String>>::new());
    let open_voting_tabs = RwSignal::new(HashMap::<String, String>::new());

    let switch_to_tab = move |tab: VotingTab| {
        active_tab.set(tab);
    };

    let close_voting_tab = move |voting_id: String| {
        selected_options_map.update(|map| {
            map.remove(&voting_id);
        });
        open_voting_tabs.update(|tabs| {
            tabs.remove(&voting_id);
        });
        votings.update(|map| {
            if let Some(state) = map.get(&voting_id)
                && matches!(state, VotingState::Results { .. })
            {
                map.remove(&voting_id);
            }
        });
        switch_to_tab(VotingTab::List);
    };

    // Auto-complete effect: creator sends result + end when conditions are met
    Effect::new(move |_| {
        let votings_snapshot = votings.get();
        for (voting_id, state) in votings_snapshot.iter() {
            if let VotingState::Active {
                voting,
                participants,
                votes,
                remaining_seconds,
                created_at,
            } = state
            {
                if voting.creator != username.get() {
                    continue;
                }

                let age_ms = js_sys::Date::now() - created_at;
                let completion = check_should_complete(
                    participants.len(),
                    votes.len(),
                    *remaining_seconds,
                    age_ms,
                );

                if completion.should_complete {
                    log::debug!("Voting {} completing", voting_id);
                    let result_payload = compute_results(voting, votes, participants.len());

                    if let Some(sender) = ws_sender.get() {
                        let _ = sender.try_send_event(ClientEvent::VotingResult(result_payload));
                        let _ = sender.try_send_event(ClientEvent::VotingEnd(
                            shared::events::VotingEndPayload {
                                voting_id: voting_id.clone(),
                            },
                        ));
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
                <TabBar
                    tabs=all_tabs
                    active_tab=active_tab
                    theme=theme.clone()
                    on_close=move |tab: VotingTab| {
                        if let VotingTab::Voting(voting_id) = tab {
                            close_voting_tab(voting_id);
                        }
                    }
                />

                <div style="flex: 1; overflow-y: auto; padding: 0.25rem 1rem 4rem 1rem;">
                    {move || match active_tab.get() {
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

                        VotingTab::Voting(voting_id) => view! {
                            <VotingActive
                                voting_id=voting_id.clone()
                                voting=votings
                                username=username
                                ws_sender=ws_sender
                                voted_in=voted_in
                                selected_options_map=selected_options_map
                                theme=theme.clone()
                            />
                        }.into_any(),
                    }}
                </div>
            </div>
        </DraggableWindow>
    }
}
