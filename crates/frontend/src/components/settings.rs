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

    let form_bg = theme.auth_form_bg;
    let input_bg = theme.auth_input_bg;
    let input_border = theme.auth_input_border;
    let input_text = theme.auth_input_text;
    let button_color = theme.auth_button_room;

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
                        "background: {}; padding: 40px; border-radius: 10px; max-width: 500px; width: 100%; position: relative;",
                        form_bg
                    )
                    on:click=|ev| ev.stop_propagation()
                >
                    <h2 style="color: white; margin-bottom: 30px;">{t!(i18n, settings.title)}</h2>

                    <div style="display: flex; flex-direction: column; gap: 20px;">
                        <div style="display: flex; flex-direction: column; gap: 8px;">
                            <label style="color: #ccc;">{t!(i18n, settings.language)}</label>
                            <select
                                on:change=on_language_change
                                style=format!(
                                    "padding: 12px; border-radius: 5px; border: 1px solid {}; background: {}; color: {}; font-size: 16px; cursor: pointer;",
                                    input_border, input_bg, input_text
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
                                "padding: 12px; background: {}; color: white; border: none; border-radius: 5px; font-size: 16px; cursor: pointer; font-weight: bold;",
                                button_color
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
