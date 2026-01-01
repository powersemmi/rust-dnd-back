use super::types::VotingState;
use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;
use std::collections::HashMap;

#[component]
pub fn VotingResults(
    voting_id: String,
    votings: RwSignal<HashMap<String, VotingState>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        {move || {
            votings.get().get(&voting_id).cloned().map(|state| {
                match state {
                    VotingState::Results { voting, results, total_participants, total_voted } => {
                        let options_stored = StoredValue::new(voting.options.clone());
                        let is_anonymous = voting.is_anonymous;

                        view! {
                            <div>
                                <h3 style=format!("color: {}; margin-top: 0;", theme.ui_text_primary)>
                                    {voting.question.clone()}
                                    {" - "}
                                    {t!(i18n, voting.results_title)}
                                </h3>

                                <div style=format!("margin-bottom: 1.25rem; padding: 0.9375rem; background: {}; border-radius: 0.5rem;", theme.ui_bg_secondary)>
                                    <p style=format!("color: {}; margin: 0.3125rem 0;", theme.ui_text_primary)>
                                        <strong>{t!(i18n, voting.total_participants)}</strong>
                                        {" "}
                                        {total_participants}
                                    </p>
                                    <p style=format!("color: {}; margin: 0.3125rem 0;", theme.ui_text_primary)>
                                        <strong>{t!(i18n, voting.total_voted)}</strong>
                                        {" "}
                                        {total_voted}
                                    </p>
                                </div>

                                <For
                                    each=move || results.clone()
                                    key=|r| r.option_id.clone()
                                    children=move |result| {
                                        let option_text = options_stored.get_value().iter()
                                            .find(|o| o.id == result.option_id)
                                            .map(|o| o.text.clone())
                                            .unwrap_or_default();

                                        let percentage = if total_voted > 0 {
                                            (result.count as f32 / total_voted as f32 * 100.0) as u32
                                        } else {
                                            0
                                        };

                                        let voters_opt = result.voters.clone();

                                        view! {
                                            <div style="margin-bottom: 1.25rem;">
                                                <div style="display: flex; justify-content: space-between; margin-bottom: 0.5rem;">
                                                    <span style=format!("color: {}; font-weight: bold;", theme.ui_text_primary)>{option_text}</span>
                                                    <span style=format!("color: {};", theme.ui_text_secondary)>
                                                        {result.count}
                                                        {" "}
                                                        {t!(i18n, voting.votes_count)}
                                                        {" ("}
                                                        {percentage}
                                                        {"%)"}</span>
                                                </div>
                                                <div style=format!("width: 100%; height: 1.5rem; background: {}; border-radius: 0.75rem; overflow: hidden;", theme.ui_bg_primary)>
                                                    <div style=format!("height: 100%; background: {}; width: {}%;", theme.ui_button_primary, percentage) />
                                                </div>
                                                {move || {
                                                    if !is_anonymous {
                                                        if let Some(ref voters) = voters_opt {
                                                            view! {
                                                                <div style=format!("margin-top: 0.5rem; padding: 0.5rem; background: {}; border-radius: 0.375rem;", theme.ui_bg_primary)>
                                                                    <p style=format!("color: {}; font-size: 0.75rem; margin: 0;", theme.ui_text_secondary)>
                                                                        {t!(i18n, voting.voters_label)}
                                                                        {" "}
                                                                        {voters.join(", ")}
                                                                    </p>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {}.into_any()
                                                        }
                                                    } else {
                                                        view! {}.into_any()
                                                    }
                                                }}
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_any()
                    }
                    _ => view! {}.into_any()
                }
            }).unwrap_or_else(|| view! {
                <p style=format!("color: {};", theme.ui_text_secondary)>{move || t!(i18n, voting.voting_not_found)}</p>
            }.into_any())
        }}
    }
}
