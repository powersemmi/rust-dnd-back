use super::types::{VotingState, VotingTab};
use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;
use std::collections::HashMap;

#[component]
pub fn VotingList(
    votings: RwSignal<HashMap<String, VotingState>>,
    active_tab: RwSignal<VotingTab>,
    open_voting_tabs: RwSignal<HashMap<String, String>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div>
            {move || {
                let all_votings = votings.get();
                leptos::logging::log!("VotingList: Total votings: {}", all_votings.len());

                let active_votings: Vec<_> = all_votings.into_iter()
                    .filter(|(_, state)| matches!(state, VotingState::Active { .. }))
                    .collect();

                leptos::logging::log!("VotingList: Active votings: {}", active_votings.len());

                if active_votings.is_empty() {
                    view! {
                        <div style=format!("text-align: center; padding: 2.5rem; color: {};", theme.ui_text_muted)>
                            {t!(i18n, voting.no_active_votings)}
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 1rem;">
                            <For
                                each=move || {
                                    // Показываем только активные голосования
                                    votings.get().into_iter()
                                        .filter(|(_, state)| matches!(state, VotingState::Active { .. }))
                                        .collect::<Vec<_>>()
                                }
                                key=|(id, _)| id.clone()
                                children=move |item| {
                                    let (voting_id, state) = item;
                                    let vid = voting_id.clone();

                                    // Получаем вопрос для таба
                                    let question = match &state {
                                        VotingState::Active { voting, .. } => voting.question.clone(),
                                        VotingState::Results { voting, .. } => voting.question.clone(),
                                    };

                                    view! {
                                        <div
                                            style=format!("padding: 1rem; background: {}; border-radius: 0.5rem; cursor: pointer; transition: background 0.2s;", theme.ui_bg_primary)
                                            on:click=move |_| {
                                                let vid_clone = vid.clone();
                                                let question_clone = question.clone();
                                                // Добавляем таб в список открытых
                                                open_voting_tabs.update(|tabs| {
                                                    tabs.insert(vid_clone.clone(), question_clone);
                                                });
                                                // Переключаемся на таб
                                                active_tab.set(VotingTab::Voting(vid_clone));
                                            }
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
