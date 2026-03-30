use super::voting_create_view_model::VotingCreateViewModel;
use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;
use shared::events::VotingStartPayload;
use shared::events::voting::VotingType;

#[component]
pub fn VotingCreate(
    theme: Theme,
    on_create: impl Fn(VotingStartPayload) + 'static + Copy + Send + Sync,
    on_cancel: impl Fn() + 'static + Copy + Send + Sync,
) -> impl IntoView {
    let i18n = use_i18n();
    let vm = VotingCreateViewModel::new();

    let submit = move || {
        if let Some(payload) = vm.build_payload() {
            on_create(payload);
            vm.reset();
        }
    };

    view! {
        <div style="max-width: 37.5rem; margin: 0 auto;">
            // Question field
            <div style="margin-bottom: 1.25rem;">
                <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                    {move || t!(i18n, voting.question_label)}
                </label>
                <input
                    type="text"
                    prop:value=move || vm.question.get()
                    on:input=move |ev| vm.question.set(event_target_value(&ev))
                    style=format!(
                        "width: 100%; padding: 0.625rem; background: {}; color: {}; \
                         border: 0.0625rem solid {}; border-radius: 0.375rem; box-sizing: border-box;",
                        theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border
                    )
                />
            </div>

            // Options list
            <div style="margin-bottom: 1.25rem;">
                <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                    {move || t!(i18n, voting.options_label)}
                </label>
                <For
                    each=move || { (0..vm.options.get().len()).collect::<Vec<_>>() }
                    key=|idx| *idx
                    children=move |idx| {
                        let option_val = vm.options.with(|opts| opts.get(idx).cloned().unwrap_or_default());
                        view! {
                            <div style="display: flex; gap: 0.5rem; margin-bottom: 0.5rem;">
                                <input
                                    type="text"
                                    prop:value=option_val.clone()
                                    on:input=move |ev| vm.update_option(idx, event_target_value(&ev))
                                    style=format!(
                                        "flex: 1; padding: 0.5rem; background: {}; color: {}; \
                                         border: 0.0625rem solid {}; border-radius: 0.375rem; box-sizing: border-box;",
                                        theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border
                                    )
                                />
                                {move || if vm.options.get().len() > 2 {
                                    view! {
                                        <button
                                            on:click=move |_| vm.remove_option(idx)
                                            style=format!(
                                                "padding: 0.5rem 0.75rem; background: {}; color: {}; \
                                                 border: none; border-radius: 0.375rem; cursor: pointer;",
                                                theme.ui_button_danger, theme.ui_text_primary
                                            )
                                        >
                                            {"x"}
                                        </button>
                                    }.into_any()
                                } else { ().into_any() }}
                            </div>
                        }
                    }
                />
                <button
                    on:click=move |_| vm.add_option()
                    style=format!(
                        "padding: 0.5rem 1rem; background: {}; color: {}; border: none; \
                         border-radius: 0.375rem; cursor: pointer; margin-top: 0.5rem;",
                        theme.ui_bg_secondary, theme.ui_text_primary
                    )
                >
                    {move || t!(i18n, voting.add_option)}
                </button>
            </div>

            // Voting type selector
            <div style="margin-bottom: 1.25rem;">
                <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                    {move || t!(i18n, voting.voting_type_label)}
                </label>
                <select
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        vm.voting_type.set(if value == "multiple" {
                            VotingType::MultipleChoice
                        } else {
                            VotingType::SingleChoice
                        });
                    }
                    style=format!(
                        "width: 100%; padding: 0.625rem; background: {}; color: {}; \
                         border: 0.0625rem solid {}; border-radius: 0.375rem;",
                        theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border
                    )
                >
                    <option value="single">{move || t!(i18n, voting.single_choice)}</option>
                    <option value="multiple">{move || t!(i18n, voting.multiple_choice)}</option>
                </select>
            </div>

            // Anonymous checkbox
            <div style="margin-bottom: 1.25rem;">
                <label style=format!("color: {}; display: flex; align-items: center; cursor: pointer;", theme.ui_text_secondary)>
                    <input
                        type="checkbox"
                        prop:checked=move || vm.is_anonymous.get()
                        on:change=move |ev| vm.is_anonymous.set(event_target_checked(&ev))
                        style="margin-right: 0.5rem; cursor: pointer;"
                    />
                    {move || t!(i18n, voting.anonymous_label)}
                </label>
            </div>

            // Timer + default option (SingleChoice only)
            {move || if matches!(vm.voting_type.get(), VotingType::SingleChoice) {
                view! {
                    <div>
                        <div style="margin-bottom: 1.25rem;">
                            <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                                {move || t!(i18n, voting.timer_label)}
                            </label>
                            <input
                                type="number"
                                prop:value=move || vm.timer_str.get()
                                on:input=move |ev| vm.timer_str.set(event_target_value(&ev))
                                placeholder="60"
                                style=format!(
                                    "width: 100%; padding: 0.625rem; background: {}; color: {}; \
                                     border: 0.0625rem solid {}; border-radius: 0.375rem; box-sizing: border-box;",
                                    theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border
                                )
                            />
                        </div>

                        {move || if !vm.timer_str.get().is_empty() {
                            view! {
                                <div style="margin-bottom: 1.25rem;">
                                    <label style=format!("color: {}; display: block; margin-bottom: 0.5rem;", theme.ui_text_secondary)>
                                        {move || t!(i18n, voting.default_option_label)}
                                    </label>
                                    <select
                                        on:change=move |ev| {
                                            if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                vm.default_option_index.set(idx);
                                            }
                                        }
                                        style=format!(
                                            "width: 100%; padding: 0.625rem; background: {}; color: {}; \
                                             border: 0.0625rem solid {}; border-radius: 0.375rem;",
                                            theme.ui_bg_primary, theme.ui_text_primary, theme.ui_border
                                        )
                                    >
                                        <For
                                            each=move || {
                                                vm.options.get().into_iter().enumerate()
                                                    .filter(|(_, opt)| !opt.is_empty())
                                                    .map(|(i, opt)| (i, opt))
                                                    .collect::<Vec<_>>()
                                            }
                                            key=|(i, _)| *i
                                            children=move |(idx, text)| {
                                                view! { <option value=idx.to_string()>{text}</option> }
                                            }
                                        />
                                    </select>
                                </div>
                            }.into_any()
                        } else { ().into_any() }}
                    </div>
                }.into_any()
            } else { ().into_any() }}

            // Action buttons
            <div style="display: flex; gap: 0.75rem; margin-top: 1.875rem; margin-bottom: 0.5rem;">
                <button
                    on:click=move |_| submit()
                    style=format!(
                        "flex: 1; padding: 0.75rem; background: {}; color: {}; border: none; \
                         border-radius: 0.375rem; cursor: pointer; font-weight: 500;",
                        theme.ui_button_primary, theme.ui_text_primary
                    )
                >
                    {move || t!(i18n, voting.create_voting_button)}
                </button>
                <button
                    on:click=move |_| on_cancel()
                    style=format!(
                        "padding: 0.75rem 1.5rem; background: {}; color: {}; border: none; \
                         border-radius: 0.375rem; cursor: pointer;",
                        theme.ui_bg_secondary, theme.ui_text_primary
                    )
                >
                    {move || t!(i18n, voting.cancel_button)}
                </button>
            </div>
        </div>
    }
}
