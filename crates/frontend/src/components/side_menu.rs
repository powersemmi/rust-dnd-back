use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;

#[component]
pub fn SideMenu(
    #[prop(into)] is_open: RwSignal<bool>,
    on_chat_open: Callback<()>,
    on_settings_open: Callback<()>,
    on_statistics_open: Callback<()>,
    on_voting_open: Callback<()>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let toggle_menu = move |_| {
        is_open.update(|open| *open = !*open);
    };

    let form_bg = theme.ui_bg_primary;
    let button_bg = theme.ui_button_primary;
    let button_hover = "#1d4ed8"; // Darker blue for hover

    view! {
        <div>
            // Кнопка открытия меню
            <button
                on:click=toggle_menu
                style=format!(
                    "position: fixed; top: 1.25rem; left: 1.25rem; z-index: 1000; padding: 0.625rem 0.9375rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; font-size: 1.125rem;",
                    button_bg, theme.ui_text_primary
                )
            >
                "☰"
            </button>

            // Боковое меню
            <div
                style=move || format!(
                    "position: fixed; top: 0; left: {}; width: 15.625rem; height: 100vh; background: {}; box-shadow: 0.125rem 0 0.625rem rgba(0,0,0,0.3); transition: left 0.3s ease; z-index: 999; padding: 4.375rem 1.25rem 1.25rem 1.25rem;",
                    if is_open.get() { "0" } else { "-15.625rem" },
                    form_bg
                )
            >
                <div style="display: flex; flex-direction: column; gap: 0.625rem;">
                    <button
                        on:click=move |_| {
                            on_chat_open.run(());
                        }
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; text-align: left; transition: background 0.2s;",
                            button_bg, theme.ui_text_primary
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        {"💬 "}{move || t!(i18n, menu.chat)}
                    </button>

                    <button
                        on:click=move |_| {
                            on_settings_open.run(());
                        }
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; text-align: left; transition: background 0.2s;",
                            button_bg, theme.ui_text_primary
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        {"⚙️ "}{move || t!(i18n, menu.settings)}
                    </button>

                    <button
                        on:click=move |_| {
                            on_statistics_open.run(());
                        }
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; text-align: left; transition: background 0.2s;",
                            button_bg, theme.ui_text_primary
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        {"📊 "}{move || t!(i18n, menu.statistics)}
                    </button>

                    <button
                        on:click=move |_| {
                            on_voting_open.run(());
                        }
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; text-align: left; transition: background 0.2s;",
                            button_bg, theme.ui_text_primary
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        {"🗳️ "}{move || t!(i18n, menu.voting)}
                    </button>
                </div>
            </div>
        </div>
    }
}
