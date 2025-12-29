use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;

#[component]
pub fn SideMenu(
    #[prop(into)] is_open: RwSignal<bool>,
    on_chat_open: Callback<()>,
    on_settings_open: Callback<()>,
    on_statistics_open: Callback<()>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let toggle_menu = move |_| {
        is_open.update(|open| *open = !*open);
    };

    let form_bg = theme.auth_form_bg;
    let button_bg = "#444";
    let button_hover = "#555";

    view! {
        <div>
            // Кнопка открытия меню
            <button
                on:click=toggle_menu
                style=format!(
                    "position: fixed; top: 20px; left: 20px; z-index: 1000; padding: 10px 15px; background: {}; color: white; border: none; border-radius: 5px; cursor: pointer; font-size: 18px;",
                    button_bg
                )
            >
                "☰"
            </button>

            // Боковое меню
            <div
                style=move || format!(
                    "position: fixed; top: 0; left: {}; width: 250px; height: 100vh; background: {}; box-shadow: 2px 0 10px rgba(0,0,0,0.3); transition: left 0.3s ease; z-index: 999; padding: 70px 20px 20px 20px;",
                    if is_open.get() { "0" } else { "-250px" },
                    form_bg
                )
            >
                <div style="display: flex; flex-direction: column; gap: 10px;">
                    <button
                        on:click=move |_| {
                            on_chat_open.run(());
                        }
                        style=format!(
                            "padding: 12px; background: {}; color: white; border: none; border-radius: 5px; cursor: pointer; text-align: left; transition: background 0.2s;",
                            button_bg
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
                            "padding: 12px; background: {}; color: white; border: none; border-radius: 5px; cursor: pointer; text-align: left; transition: background 0.2s;",
                            button_bg
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
                            "padding: 12px; background: {}; color: white; border: none; border-radius: 5px; cursor: pointer; text-align: left; transition: background 0.2s;",
                            button_bg
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        {"📊 "}{move || t!(i18n, menu.statistics)}
                    </button>
                </div>
            </div>
        </div>
    }
}
