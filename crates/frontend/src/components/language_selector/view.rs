use super::model::{parse_locale, save_locale_to_storage};
use super::view_model::LanguageSelectorViewModel;
use crate::config::Theme;
use crate::i18n::i18n::{Locale, use_i18n};
use leptos::prelude::*;

#[component]
pub fn LanguageSelector(initial_locale: Locale, theme: Theme) -> impl IntoView {
    let i18n = use_i18n();
    let vm = LanguageSelectorViewModel::new(initial_locale);

    // Apply the saved locale on mount
    leptos::task::spawn_local(async move {
        i18n.set_locale(initial_locale);
    });

    let on_change = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        let locale = parse_locale(&value);
        vm.current_locale.set(locale);
        i18n.set_locale(locale);
        save_locale_to_storage(&value);
    };

    let button_style = format!(
        "background: {}; color: {}; padding: 0.5rem 1rem; border: none; \
         border-radius: 0.25rem; cursor: pointer; font-size: 0.875rem;",
        theme.ui_button_primary, theme.ui_text_primary
    );

    view! {
        <div style="position: fixed; top: 1.25rem; right: 1.25rem; z-index: 1000;">
            <select
                on:change=on_change
                style=button_style
                prop:value=move || vm.locale_code()
            >
                <option value="en">"EN"</option>
                <option value="ru">"RU"</option>
            </select>
        </div>
    }
}
