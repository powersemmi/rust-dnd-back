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
use shared::events::VotingStartPayload;
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
                                .filter(|(_, state)| matches!(state, VotingState::Active(_)))
                                .map(|(id, state)| {
                                    let voting = match state {
                                        VotingState::Active(v) => v.clone(),
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
