use crate::config::Theme;
use crate::i18n::i18n::{Locale, use_i18n};
use leptos::prelude::*;

#[component]
pub fn LanguageSelector(initial_locale: Locale, theme: Theme) -> impl IntoView {
    let i18n = use_i18n();
    let (current_locale, set_current_locale) = signal(initial_locale);

    // Set initial locale
    leptos::task::spawn_local(async move {
        i18n.set_locale(initial_locale);
    });

    let on_change = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        let new_locale = if value == "ru" {
            Locale::ru
        } else {
            Locale::en
        };

        i18n.set_locale(new_locale);
        set_current_locale.set(new_locale);

        // Save to localStorage
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("locale", &value);
            }
        }
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
                prop:value=move || {
                    match current_locale.get() {
                        Locale::en => "en",
                        Locale::ru => "ru",
                    }
                }
            >
                <option value="en">"EN"</option>
                <option value="ru">"RU"</option>
            </select>
        </div>
    }
}
