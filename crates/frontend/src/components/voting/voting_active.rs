use super::types::VotingState;
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::voting::VotingType;
use shared::events::{ClientEvent, VotingCastPayload};
use std::collections::{HashMap, HashSet};

#[component]
pub fn VotingActive(
    voting_id: String,
    voting: RwSignal<HashMap<String, VotingState>>,
    username: ReadSignal<String>,
    ws_sender: ReadSignal<Option<super::super::websocket::WsSender>>,
    voted_in: RwSignal<HashSet<String>>,
    selected_options_map: RwSignal<HashMap<String, HashSet<String>>>,
    theme: Theme,
) -> impl IntoView {
    leptos::logging::log!(
        "VotingActive component created for voting_id: {}",
        voting_id
    );
    let i18n = use_i18n();

    let cast_vote = move |vid: String| {
        log!("cast_vote called for voting_id: {}", vid);

        let selected = selected_options_map
            .get()
            .get(&vid)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        log!("Selected options: {:?}", selected);

        if selected.is_empty() {
            log!("No options selected, aborting vote");
            return;
        }

        let payload = VotingCastPayload {
            voting_id: vid.clone(),
            user: username.get(),
            selected_option_ids: selected,
        };

        log!("Sending VotingCast payload: {:?}", payload);

        if let Some(mut sender) = ws_sender.get() {
            if let Ok(json) = serde_json::to_string(&ClientEvent::VotingCast(payload)) {
                log!("Sending VotingCast JSON: {}", json);
                let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
            }
        } else {
            log!("WebSocket sender not available!");
        }

        voted_in.update(|set| {
            set.insert(vid);
        });

        log!("Vote cast successfully");
    };

    view! {
        {move || {
            let state_opt = voting.get().get(&voting_id).cloned();
            log!("VotingActive rendering, state exists: {}", state_opt.is_some());

            state_opt.map(|state| {
                match state {
                    VotingState::Active { voting, participants: _, votes, remaining_seconds, .. } => {
                        log!("Rendering Active voting: {}", voting.question);
                        let vid = voting_id.clone();
                        let vid_check = voting_id.clone();
                        let voting_type_stored = StoredValue::new(voting.voting_type.clone());
                        let options_stored = StoredValue::new(voting.options.clone());

                        let has_voted = voted_in.get().contains(&vid_check);

                        // Подсчитываем промежуточные результаты для отображения
                        let total_voted_before = votes.len() as u32;
                        let mut vote_counts: HashMap<String, u32> = HashMap::new();
                        for option_ids in votes.values() {
                            for option_id in option_ids {
                                *vote_counts.entry(option_id.clone()).or_insert(0) += 1;
                            }
                        }
                        let vote_counts_stored = StoredValue::new(vote_counts);
                        let total_voted_stored = StoredValue::new(total_voted_before);

                        let vid_for_button_click = vid.clone();
                        let vid_for_button_disabled = vid.clone();
                        let vid_for_button_style = vid.clone();

                        view! {
                            <div>
                                    <h3 style=format!("color: {}; margin-top: 0; margin-bottom: 1rem;", theme.ui_text_primary)>{voting.question.clone()}</h3>

                                    {move || {
                                        if let Some(timer) = remaining_seconds {
                                            view! {
                                                <div style=format!("padding: 0.625rem; background: {}; border-radius: 0.375rem; margin-bottom: 1rem; text-align: center;", theme.ui_bg_secondary)>
                                                    <span style="color: #fbbf24; font-weight: bold;">
                                                        {t!(i18n, voting.time_remaining)}
                                                        {" "}
                                                        {format!("{} сек", timer)}
                                                    </span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {}.into_any()
                                        }
                                    }}

                                    <p style=format!("color: {}; margin: 0 0 1.25rem 0;", theme.ui_text_secondary)>
                                        {move || match voting.voting_type {
                                            VotingType::SingleChoice => view! { {t!(i18n, voting.select_one)} }.into_any(),
                                            VotingType::MultipleChoice => view! { {t!(i18n, voting.select_multiple)} }.into_any(),
                                        }}
                                    </p>

                                    <For
                                        each=move || options_stored.get_value()
                                        key=|opt| opt.id.clone()
                                        children=move |option| {
                                            let option_id = option.id.clone();
                                            let option_id_check = option.id.clone();
                                            let option_id_check_style = option.id.clone();
                                            let option_id_stats = option.id.clone();
                                            let voting_type = voting_type_stored.get_value();
                                            let vid_for_style = vid.clone();
                                            let vid_for_click = vid.clone();
                                            let vid_for_checked = vid_check.clone();

                                            let vote_count = vote_counts_stored.get_value().get(&option_id_stats).copied().unwrap_or(0);
                                            let percentage = if total_voted_stored.get_value() > 0 {
                                                (vote_count as f32 / total_voted_stored.get_value() as f32 * 100.0) as u32
                                            } else {
                                                0
                                            };

                                            view! {
                                                <div
                                                    style=move || {
                                                        let selected_opts = selected_options_map.get().get(&vid_for_style).cloned().unwrap_or_default();
                                                        let is_selected = selected_opts.contains(&option_id_check_style);
                                                        let border_color = if is_selected { &theme.ui_button_primary } else { &theme.ui_border };
                                                        let bg_color = if is_selected {
                                                            // Полупрозрачный цвет кнопки для выделения
                                                            format!("{}20", theme.ui_button_primary)
                                                        } else {
                                                            theme.ui_bg_primary.to_string()
                                                        };
                                                        let cursor = if has_voted { "not-allowed" } else { "pointer" };
                                                        let opacity = if has_voted { "0.6" } else { "1" };
                                                        format!("padding: 0.75rem; margin-bottom: 0.625rem; background: {}; border: 0.125rem solid {}; border-radius: 0.5rem; cursor: {}; opacity: {}; overflow: hidden;", bg_color, border_color, cursor, opacity)
                                                    }
                                                    on:click=move |_| {
                                                        if !has_voted {
                                                            log!("Option clicked: {} for voting {}", option_id, vid_for_click);
                                                            let vid_clone = vid_for_click.clone();
                                                            selected_options_map.update(|map| {
                                                                let entry = map.entry(vid_clone.clone()).or_insert_with(HashSet::new);
                                                                match voting_type {
                                                                    VotingType::SingleChoice => {
                                                                        entry.clear();
                                                                        entry.insert(option_id.clone());
                                                                        log!("SingleChoice: selected {}", option_id);
                                                                    }
                                                                    VotingType::MultipleChoice => {
                                                                        if entry.contains(&option_id) {
                                                                            entry.remove(&option_id);
                                                                            log!("MultipleChoice: deselected {}", option_id);
                                                                        } else {
                                                                            entry.insert(option_id.clone());
                                                                            log!("MultipleChoice: selected {}", option_id);
                                                                        }
                                                                    }
                                                                }
                                                                log!("Updated selection for {}: {:?}", vid_clone, entry);
                                                            });
                                                        } else {
                                                            log!("Click ignored - already voted");
                                                        }
                                                    }
                                                >
                                                    <div style="display: flex; align-items: center; gap: 0.625rem; margin-bottom: 0.5rem;">
                                                        <input
                                                            type={match voting_type {
                                                                VotingType::SingleChoice => "radio",
                                                                VotingType::MultipleChoice => "checkbox",
                                                            }}
                                                            checked=move || {
                                                                selected_options_map.get().get(&vid_for_checked).map(|s| s.contains(&option_id_check)).unwrap_or(false)
                                                            }
                                                            disabled=has_voted
                                                            style="width: 1.125rem; height: 1.125rem; pointer-events: none;"
                                                        />
                                                        <span style=format!("color: {}; font-size: 1rem; flex: 1;", theme.ui_text_primary)>
                                                            {move || {
                                                                let text = option.text.clone();
                                                                match text.as_str() {
                                                                    ".0" => t_string!(i18n, voting.no).to_string(),
                                                                    ".1" => t_string!(i18n, voting.yes).to_string(),
                                                                    _ => text,
                                                                }
                                                            }}
                                                        </span>
                                                        <span style=format!("color: {}; font-size: 0.875rem; font-weight: bold;", theme.ui_text_secondary)>
                                                            {format!("{} ({}%)", vote_count, percentage)}
                                                        </span>
                                                    </div>
                                                    // График
                                                    <div style=format!("width: 100%; height: 0.375rem; background: {}; border-radius: 0.1875rem; overflow: hidden; margin-left: 1.75rem;", theme.ui_bg_secondary)>
                                                        <div style=format!("height: 100%; background: {}; width: {}%;", theme.ui_button_primary, percentage) />
                                                    </div>
                                                </div>
                                            }
                                        }
                                    />

                                    <button
                                        on:click=move |_| cast_vote(vid_for_button_click.clone())
                                        disabled=move || {
                                            let selected = selected_options_map.get().get(&vid_for_button_disabled).cloned().unwrap_or_default();
                                            selected.is_empty() || has_voted
                                        }
                                        style=move || {
                                            let selected = selected_options_map.get().get(&vid_for_button_style).cloned().unwrap_or_default();
                                            let is_disabled = selected.is_empty() || has_voted;
                                            let opacity = if is_disabled { "0.5" } else { "1" };
                                            let cursor = if is_disabled { "not-allowed" } else { "pointer" };
                                            format!("width: 100%; padding: 0.75rem; margin-top: 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; font-size: 1rem; cursor: {}; font-weight: bold; opacity: {};", theme.ui_button_primary, theme.ui_text_primary, cursor, opacity)
                                        }
                                    >
                                        {move || t!(i18n, voting.submit_vote)}
                                    </button>
                            </div>
                        }.into_any()
                    }
                    VotingState::Results { voting, results, total_participants, total_voted } => {
                        let has_voted = voted_in.get().contains(&voting_id);
                        let options_stored = StoredValue::new(voting.options.clone());
                        let results_stored = StoredValue::new(results.clone());

                        view! {
                            <div>
                                <h3 style=format!("color: {}; margin-top: 0; margin-bottom: 1rem;", theme.ui_text_primary)>{voting.question.clone()}</h3>

                                <div style=format!("padding: 1.25rem; background: {}; border-radius: 0.5rem; text-align: center; margin-bottom: 1rem;", theme.ui_bg_primary)>
                                    <div style="font-size: 3rem; margin-bottom: 1rem;">"✓"</div>
                                    <h4 style=format!("color: {}; margin: 0 0 0.5rem 0;", theme.ui_success)>{t!(i18n, voting.completed_status)}</h4>
                                    {if has_voted {
                                        view! { <p style=format!("color: {}; margin: 0;", theme.ui_text_secondary)>{t!(i18n, voting.vote_submitted)}</p> }.into_any()
                                    } else {
                                        view! { <p style=format!("color: {}; margin: 0;", theme.ui_text_secondary)>{t!(i18n, voting.voting_ended)}</p> }.into_any()
                                    }}
                                </div>

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
                                    each=move || results_stored.get_value()
                                    key=|r| r.option_id.clone()
                                    children=move |result| {
                                        let option_text = options_stored.get_value().iter()
                                            .find(|o| o.id == result.option_id)
                                            .map(|o| {
                                                match o.text.as_str() {
                                                    ".0" => t_string!(i18n, voting.no).to_string(),
                                                    ".1" => t_string!(i18n, voting.yes).to_string(),
                                                    _ => o.text.clone(),
                                                }
                                            })
                                            .unwrap_or_default();

                                        let percentage = if total_voted > 0 {
                                            (result.count as f32 / total_voted as f32 * 100.0) as u32
                                        } else {
                                            0
                                        };

                                        let voters_opt = result.voters.clone();
                                        let is_anonymous = voting.is_anonymous;

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
                }
            }).unwrap_or_else(|| view! {
                <p style=format!("color: {};", theme.ui_text_secondary)>{move || t!(i18n, voting.voting_not_found)}</p>
            }.into_any())
        }}
    }
}
