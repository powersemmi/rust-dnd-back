use super::types::{VotingState, VotingTab};
use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;
use std::collections::HashMap;

#[component]
pub fn VotingList(
    votings: RwSignal<HashMap<String, VotingState>>,
    active_tab: RwSignal<VotingTab>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div style="flex: 1; overflow-y: auto; padding: 1.25rem;">
            {move || {
                if votings.get().is_empty() {
                    view! {
                        <div style=format!("text-align: center; padding: 2.5rem; color: {};", theme.ui_text_muted)>
                            {t!(i18n, voting.no_active_votings)}
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 0.625rem;">
                            <For
                                each=move || {
                                    votings.get().into_iter().collect::<Vec<_>>()
                                }
                                key=|(id, _)| id.clone()
                                children=move |item| {
                                    let (voting_id, state) = item;
                                    let vid = voting_id.clone();
                                    view! {
                                        <div
                                            style=format!("padding: 0.9375rem; background: {}; border-radius: 0.5rem; cursor: pointer;", theme.ui_bg_primary)
                                            on:click=move |_| active_tab.set(VotingTab::Voting(vid.clone()))
                                        >
                                            {move || {
                                                match &state {
                                                    VotingState::Active { voting, .. } => view! {
                                                        <div>
                                                            <h4 style=format!("color: {}; margin: 0 0 0.5rem 0;", theme.ui_text_primary)>{voting.question.clone()}</h4>
                                                            <span style=format!("color: {};", theme.ui_success)>{t!(i18n, voting.active_status)}</span>
                                                        </div>
                                                    }.into_any(),
                                                    VotingState::Results { voting, total_voted, .. } => {
                                                        let count = *total_voted;
                                                        view! {
                                                            <div>
                                                                <h4 style=format!("color: {}; margin: 0 0 0.5rem 0;", theme.ui_text_primary)>{voting.question.clone()}</h4>
                                                                <span style=format!("color: {};", theme.ui_text_secondary)>
                                                                    {t!(i18n, voting.completed_status)}
                                                                    {format!(" ({} ", count)}
                                                                    {move || t!(i18n, voting.votes_count)}
                                                                    {")"}
                                                                </span>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }
                                            }}
                                        </div>
                                    }
                                }
                            />
                        </div>
                    }.into_any()
                }
            }}

            <button
                on:click=move |_| active_tab.set(VotingTab::Create)
                style=format!("width: 100%; padding: 0.75rem; margin-top: 1.25rem; background: {}; color: {}; border: none; border-radius: 0.375rem; font-size: 1rem; cursor: pointer; font-weight: bold;", theme.ui_button_primary, theme.ui_text_primary)
            >
                {move || t!(i18n, voting.create_button)}
            </button>
        </div>
    }
}
