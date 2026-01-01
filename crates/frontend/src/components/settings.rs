use crate::config::Theme;
use crate::i18n::i18n::{Locale, t, use_i18n};
use leptos::prelude::*;

#[component]
pub fn Settings(#[prop(into)] is_open: RwSignal<bool>, theme: Theme) -> impl IntoView {
    let i18n = use_i18n();
    let current_locale = i18n.get_locale();

    let on_close = move |_| {
        is_open.set(false);
    };

    let on_language_change = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        let new_locale = if value == "ru" {
            Locale::ru
        } else {
            Locale::en
        };

        i18n.set_locale(new_locale);

        // Save to localStorage
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("locale", &value);
            }
        }
    };

    view! {
        <Show when=move || is_open.get()>
            <div
                style="
                    position: fixed;
                    top: 0;
                    left: 0;
                    width: 100vw;
                    height: 100vh;
                    background: rgba(0, 0, 0, 0.7);
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    z-index: 2000;
                "
                on:click=on_close
            >
                <div
                    style=format!(
                        "background: {}; padding: 2.5rem; border-radius: 0.625rem; max-width: 31.25rem; width: 100%; position: relative;",
                        theme.ui_bg_primary
                    )
                    on:click=|ev| ev.stop_propagation()
                >
                    <h2 style=format!("color: {}; margin-bottom: 1.875rem;", theme.ui_text_primary)>{t!(i18n, settings.title)}</h2>

                    <div style="display: flex; flex-direction: column; gap: 1.25rem;">
                        <div style="display: flex; flex-direction: column; gap: 0.5rem;">
                            <label style=format!("color: {};", theme.ui_text_primary)>{t!(i18n, settings.language)}</label>
                            <select
                                on:change=on_language_change
                                style=format!(
                                    "padding: 0.75rem; border-radius: 0.3125rem; border: 0.0625rem solid {}; background: {}; color: {}; font-size: 1rem; cursor: pointer;",
                                    theme.ui_border, theme.ui_bg_secondary, theme.ui_text_primary
                                )
                                prop:value=move || {
                                    match current_locale {
                                        Locale::en => "en",
                                        Locale::ru => "ru",
                                    }
                                }
                            >
                                <option value="en">"English"</option>
                                <option value="ru">"Русский"</option>
                            </select>
                        </div>

                        <button
                            on:click=on_close
                            style=format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; font-size: 1rem; cursor: pointer; font-weight: bold;",
                                theme.ui_button_primary, theme.ui_text_primary
                            )
                        >
                            {t!(i18n, settings.close)}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
