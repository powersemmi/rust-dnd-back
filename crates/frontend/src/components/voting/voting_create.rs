use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;
use shared::events::VotingStartPayload;
use shared::events::voting::{VotingOption, VotingType};
use uuid::Uuid;

#[component]
pub fn VotingCreate(
    theme: Theme,
    on_create: impl Fn(VotingStartPayload) + 'static + Copy + Send + Sync,
    on_cancel: impl Fn() + 'static + Copy + Send + Sync,
) -> impl IntoView {
    let i18n = use_i18n();

    let create_question = RwSignal::new(String::new());
    let create_options = RwSignal::new(vec![String::new(), String::new()]);
    let create_type = RwSignal::new(VotingType::SingleChoice);
    let create_anonymous = RwSignal::new(false);
    let create_timer = RwSignal::new(String::new());
    let create_default_option = RwSignal::new(0usize);

    let add_option = move || {
        create_options.update(|opts| opts.push(String::new()));
    };

    let remove_option = move |index: usize| {
        create_options.update(|opts| {
            if opts.len() > 2 {
                opts.remove(index);
            }
        });
    };

    let update_option = move |index: usize, value: String| {
        create_options.update(|opts| {
            if index < opts.len() {
                opts[index] = value;
            }
        });
    };

    let submit_create = move || {
        let question = create_question.get();
        let options = create_options.get();
        let valid_options: Vec<_> = options.iter().filter(|s| !s.is_empty()).collect();

        if !question.is_empty() && valid_options.len() >= 2 {
            let voting_options: Vec<VotingOption> = valid_options
                .into_iter()
                .map(|text| VotingOption {
                    id: Uuid::new_v4().to_string(),
                    text: text.clone(),
                })
                .collect();

            let timer_seconds = if matches!(create_type.get(), VotingType::SingleChoice) {
                create_timer.get().parse::<u32>().ok()
            } else {
                None
            };

            let default_option_id = timer_seconds.and_then(|_| {
                let idx = create_default_option.get();
                voting_options.get(idx).map(|opt| opt.id.clone())
            });

            let payload = VotingStartPayload {
                voting_id: Uuid::new_v4().to_string(),
                question,
                options: voting_options,
                voting_type: create_type.get(),
                is_anonymous: create_anonymous.get(),
                timer_seconds,
                default_option_id,
                creator: String::new(), // Will be filled by parent component
            };

            on_create(payload);

            // Reset form
            create_question.set(String::new());
            create_options.set(vec![String::new(), String::new()]);
            create_type.set(VotingType::SingleChoice);
            create_anonymous.set(false);
            create_timer.set(String::new());
            create_default_option.set(0);
        }
    };

    view! {
        <div style="flex: 1; overflow-y: auto; padding: 1.25rem;">
            <div style="max-width: 37.5rem; margin: 0 auto;">
                <div style="margin-bottom: 1.25rem;">
                    <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                        {move || t!(i18n, voting.question_label)}
                    </label>
                    <input
                        type="text"
                        prop:value=move || create_question.get()
                        on:input=move |ev| create_question.set(event_target_value(&ev))
                        style=format!("width: 100%; padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border)
                    />
                </div>

                <div style="margin-bottom: 1.25rem;">
                    <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                        {move || t!(i18n, voting.options_label)}
                    </label>
                    <For
                        each=move || {
                            let opts = create_options.get();
                            (0..opts.len()).collect::<Vec<_>>()
                        }
                        key=|idx| *idx
                        children=move |idx| {
                            let option_val = create_options.with(|opts| opts.get(idx).cloned().unwrap_or_default());

                            view! {
                                <div style="display: flex; gap: 0.5rem; margin-bottom: 0.5rem;">
                                    <input
                                        type="text"
                                        prop:value=option_val.clone()
                                        on:input=move |ev| update_option(idx, event_target_value(&ev))
                                        style=format!("flex: 1; padding: 0.5rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border)
                                    />
                                    {move || {
                                        if create_options.get().len() > 2 {
                                            view! {
                                                <button
                                                    on:click=move |_| remove_option(idx)
                                                    style=format!("padding: 0.5rem 0.75rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;", theme.ui_button_danger, theme.ui_text_primary)
                                                >
                                                    {"✕"}
                                                </button>
                                            }.into_any()
                                        } else {
                                            view! {}.into_any()
                                        }
                                    }}
                                </div>
                            }
                        }
                    />
                    <button
                        on:click=move |_| add_option()
                        style=format!("padding: 0.5rem 1rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer; margin-top: 0.5rem;", theme.ui_bg_secondary, theme.ui_text_primary)
                    >
                        {move || t!(i18n, voting.add_option)}
                    </button>
                </div>

                <div style="margin-bottom: 1.25rem;">
                    <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                        {move || t!(i18n, voting.voting_type_label)}
                    </label>
                    <select
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            create_type.set(if value == "multiple" {
                                VotingType::MultipleChoice
                            } else {
                                VotingType::SingleChoice
                            });
                        }
                        style=format!("width: 100%; padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border)
                    >
                        <option value="single">{move || t!(i18n, voting.single_choice)}</option>
                        <option value="multiple">{move || t!(i18n, voting.multiple_choice)}</option>
                    </select>
                </div>

                <div style="margin-bottom: 1.25rem;">
                    <label style=format!("color: {}; display: flex; align-items: center; cursor: pointer;", theme.ui_text_secondary)>
                        <input
                            type="checkbox"
                            prop:checked=move || create_anonymous.get()
                            on:change=move |ev| create_anonymous.set(event_target_checked(&ev))
                            style="margin-right: 0.5rem; cursor: pointer;"
                        />
                        {move || t!(i18n, voting.anonymous_label)}
                    </label>
                </div>

                {move || {
                    if matches!(create_type.get(), VotingType::SingleChoice) {
                        view! {
                            <div>
                                <div style="margin-bottom: 1.25rem;">
                                    <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                                        {move || t!(i18n, voting.timer_label)}
                                    </label>
                                    <input
                                        type="number"
                                        prop:value=move || create_timer.get()
                                        on:input=move |ev| create_timer.set(event_target_value(&ev))
                                        style=format!("width: 100%; padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border)
                                        placeholder="60"
                                    />
                                </div>

                                {move || {
                                    if !create_timer.get().is_empty() {
                                        view! {
                                            <div style="margin-bottom: 1.25rem;">
                                                <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                                                    {move || t!(i18n, voting.default_option_label)}
                                                </label>
                                                <select
                                                    on:change=move |ev| {
                                                        if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                            create_default_option.set(idx);
                                                        }
                                                    }
                                                    style=format!("width: 100%; padding: 0.625rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.375rem;", theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border)
                                                >
                                                    <For
                                                        each=move || {
                                                            create_options.get().iter()
                                                                .enumerate()
                                                                .filter(|(_, opt)| !opt.is_empty())
                                                                .map(|(i, opt)| (i, opt.clone()))
                                                                .collect::<Vec<_>>()
                                                        }
                                                        key=|(i, _)| *i
                                                        children=move |(idx, text)| {
                                                            view! {
                                                                <option value=idx.to_string()>{text}</option>
                                                            }
                                                        }
                                                    />
                                                </select>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }
                }}

                <div style="display: flex; gap: 0.75rem; margin-top: 1.875rem;">
                    <button
                        on:click=move |_| submit_create()
                        style=format!("flex: 1; padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 500;", theme.ui_button_primary, theme.ui_text_primary)
                    >
                        {move || t!(i18n, voting.create_voting_button)}
                    </button>
                    <button
                        on:click=move |_| on_cancel()
                        style=format!("padding: 0.75rem 1.5rem; background: {}; color: {}; border: none; border-radius: 0.375rem; cursor: pointer;", theme.ui_bg_secondary, theme.ui_text_primary)
                    >
                        {move || t!(i18n, voting.cancel_button)}
                    </button>
                </div>
            </div>
        </div>
    }
}
