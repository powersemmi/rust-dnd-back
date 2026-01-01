use super::types::VotingState;
use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
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
    selected_options: RwSignal<HashSet<String>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    let cast_vote = move |vid: String| {
        let selected = selected_options.get().into_iter().collect::<Vec<_>>();
        if selected.is_empty() {
            return;
        }

        let payload = VotingCastPayload {
            voting_id: vid.clone(),
            user: username.get(),
            selected_option_ids: selected,
        };

        if let Some(mut sender) = ws_sender.get() {
            if let Ok(json) = serde_json::to_string(&ClientEvent::VotingCast(payload)) {
                let _ = sender.try_send(gloo_net::websocket::Message::Text(json));
            }
        }

        voted_in.update(|set| {
            set.insert(vid);
        });

        selected_options.set(HashSet::new());
    };

    view! {
        {move || {
            voting.get().get(&voting_id).cloned().map(|state| {
                match state {
                    VotingState::Active(voting) => {
                        let vid = voting_id.clone();
                        let vid_check = voting_id.clone();
                        let voting_type_stored = StoredValue::new(voting.voting_type.clone());
                        let options_stored = StoredValue::new(voting.options.clone());

                        let has_voted = voted_in.get().contains(&vid_check);

                        if has_voted {
                            view! {
                                <div>
                                    <h3 style=format!("color: {}; margin-top: 0;", theme.ui_text_primary)>{voting.question.clone()}</h3>

                                    <div style=format!("padding: 1.25rem; background: {}; border-radius: 0.5rem; text-align: center;", theme.ui_bg_primary)>
                                        <div style="font-size: 3rem; margin-bottom: 1rem;">"✓"</div>
                                        <h4 style=format!("color: {}; margin: 0 0 0.5rem 0;", theme.ui_success)>{t!(i18n, voting.vote_submitted)}</h4>
                                        <p style=format!("color: {}; margin: 0;", theme.ui_text_secondary)>{t!(i18n, voting.waiting_results)}</p>
                                    </div>

                                    // TODO: Здесь будут промежуточные результаты когда придет обновление
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div>
                                    <h3 style=format!("color: {}; margin-top: 0;", theme.ui_text_primary)>{voting.question.clone()}</h3>

                                    {move || {
                                        if let Some(timer) = voting.timer_seconds {
                                            view! {
                                                <div style=format!("padding: 0.625rem; background: {}; border-radius: 0.375rem; margin-bottom: 1rem; text-align: center;", theme.ui_bg_secondary)>
                                                    <span style="color: #fbbf24; font-weight: bold;">
                                                        {t!(i18n, voting.time_remaining)}
                                                        {" "}
                                                        // TODO: Добавить живой таймер
                                                        {format!("{} сек", timer)}
                                                    </span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {}.into_any()
                                        }
                                    }}

                                    <p style=format!("color: {}; margin-bottom: 1.25rem;", theme.ui_text_secondary)>
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
                                            let voting_type = voting_type_stored.get_value();
                                            view! {
                                                <div
                                                    style=format!("padding: 0.75rem; margin-bottom: 0.625rem; background: {}; border: 0.125rem solid {}; border-radius: 0.5rem; cursor: pointer;", theme.ui_bg_primary, theme.ui_border)
                                                    on:click=move |_| {
                                                        selected_options.update(|sel| {
                                                            match voting_type {
                                                                VotingType::SingleChoice => {
                                                                    sel.clear();
                                                                    sel.insert(option_id.clone());
                                                                }
                                                                VotingType::MultipleChoice => {
                                                                    if sel.contains(&option_id) {
                                                                        sel.remove(&option_id);
                                                                    } else {
                                                                        sel.insert(option_id.clone());
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    }
                                                >
                                                    <div style="display: flex; align-items: center; gap: 0.625rem;">
                                                        <input
                                                            type={match voting_type {
                                                                VotingType::SingleChoice => "radio",
                                                                VotingType::MultipleChoice => "checkbox",
                                                            }}
                                                            checked=move || selected_options.get().contains(&option_id_check)
                                                            style="width: 1.125rem; height: 1.125rem;"
                                                        />
                                                        <span style=format!("color: {}; font-size: 1rem;", theme.ui_text_primary)>{option.text.clone()}</span>
                                                    </div>
                                                </div>
                                            }
                                        }
                                    />

                                    <button
                                        on:click=move |_| cast_vote(vid.clone())
                                        disabled=move || selected_options.get().is_empty()
                                        style=format!("width: 100%; padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.375rem; font-size: 1rem; cursor: pointer; font-weight: bold;", theme.ui_button_primary, theme.ui_text_primary)
                                    >
                                        {move || t!(i18n, voting.submit_vote)}
                                    </button>
                                </div>
                            }.into_any()
                        }
                    }
                    _ => view! {}.into_any()
                }
            }).unwrap_or_else(|| view! {
                <p style=format!("color: {};", theme.ui_text_secondary)>{move || t!(i18n, voting.voting_not_found)}</p>
            }.into_any())
        }}
    }
}
