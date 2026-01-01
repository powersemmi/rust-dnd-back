use crate::config::Theme;
use crate::i18n::i18n::{t_string, use_i18n};
use leptos::prelude::*;

#[component]
pub fn SideMenu(
    #[prop(into)] is_open: RwSignal<bool>,
    on_chat_open: Callback<()>,
    on_settings_open: Callback<()>,
    on_statistics_open: Callback<()>,
    on_voting_open: Callback<()>,
    #[prop(into)] has_statistics_notification: Signal<bool>,
    #[prop(into)] notification_count: Signal<u32>,
    #[prop(into)] has_chat_notification: Signal<bool>,
    #[prop(into)] chat_notification_count: Signal<u32>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let toggle_menu = move |_| {
        is_open.update(|open| *open = !*open);
    };

    let form_bg = theme.ui_bg_primary;
    let button_bg = theme.ui_button_primary;
    let button_hover = "#1d4ed8"; // Darker blue for hover
    let notification_color = theme.ui_notification;

    // Ширина меню
    let menu_width_open = "15.625rem";
    let menu_width_closed = "3.75rem"; // Торчащая часть

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
                style=move || {
                    let left = if is_open.get() {
                        "0".to_string()
                    } else {
                        format!("calc(-{} + {})", menu_width_open, menu_width_closed)
                    };
                    let padding_left = if is_open.get() { "1.25rem" } else { "0.5rem" };
                    let padding_right = if is_open.get() { "1.25rem" } else { "0.5rem" };
                    format!(
                        "position: fixed; top: 0; left: {}; width: {}; height: 100vh; background: {}; box-shadow: 0.125rem 0 0.625rem rgba(0,0,0,0.3); transition: all 0.3s ease; z-index: 999; padding: 4.375rem {} 1.25rem {}; overflow: hidden;",
                        left, menu_width_open, form_bg, padding_right, padding_left
                    )
                }
            >
                <div style="display: flex; flex-direction: column; gap: 0.625rem;">
                    <button
                        on:click=move |_| {
                            on_chat_open.run(());
                        }
                        style=move || {
                            let bg = if has_chat_notification.get() {
                                &theme.ui_notification
                            } else {
                                button_bg
                            };
                            format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; display: flex; justify-content: space-between; align-items: center; transition: background 0.2s; min-width: 0;",
                                bg, theme.ui_text_primary
                            )
                        }
                        onmouseover=move || {
                            if has_chat_notification.get() {
                                format!("this.style.background='{}'", theme.ui_notification)
                            } else {
                                format!("this.style.background='{}'", button_hover)
                            }
                        }
                        onmouseout=move || {
                            if has_chat_notification.get() {
                                format!("this.style.background='{}'", theme.ui_notification)
                            } else {
                                format!("this.style.background='{}'", button_bg)
                            }
                        }
                    >
                        <span style=move || if is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"💬 "}
                            {move || if is_open.get() { t_string!(i18n, menu.chat) } else { "" }}
                        </span>
                        {move || if !is_open.get() {
                            view! {
                                <span style=format!("color: {}; font-size: 0.75rem; margin-left: 0.5rem;", theme.ui_text_secondary)>
                                    {if has_chat_notification.get() && chat_notification_count.get() > 0 {
                                        chat_notification_count.get().to_string()
                                    } else {
                                        t_string!(i18n, menu.hotkey_chat).to_string()
                                    }}
                                </span>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </button>

                    <button
                        on:click=move |_| {
                            on_voting_open.run(());
                        }
                        style=move || {
                            let bg = if has_statistics_notification.get() {
                                notification_color
                            } else {
                                button_bg
                            };
                            format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; display: flex; justify-content: space-between; align-items: center; transition: background 0.2s; min-width: 0;",
                                bg, theme.ui_text_primary
                            )
                        }
                        onmouseover=move || if has_statistics_notification.get() {
                            format!("this.style.background='{}'", notification_color)
                        } else {
                            format!("this.style.background='{}'", button_hover)
                        }
                        onmouseout=move || if has_statistics_notification.get() {
                            format!("this.style.background='{}'", notification_color)
                        } else {
                            format!("this.style.background='{}'", button_bg)
                        }
                    >
                        <span style=move || if is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"🗳️ "}
                            {move || if is_open.get() { t_string!(i18n, menu.voting) } else { "" }}
                        </span>
                        {move || if !is_open.get() {
                            view! {
                                <span style=format!("color: {}; font-size: 0.75rem; margin-left: 0.5rem;", theme.ui_text_secondary)>
                                    {if has_statistics_notification.get() && notification_count.get() > 0 {
                                        notification_count.get().to_string()
                                    } else {
                                        t_string!(i18n, menu.hotkey_voting).to_string()
                                    }}
                                </span>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </button>

                    <button
                        on:click=move |_| {
                            on_statistics_open.run(());
                        }
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; display: flex; justify-content: space-between; align-items: center; transition: background 0.2s; min-width: 0;",
                            button_bg, theme.ui_text_primary
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        <span style=move || if is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"📊 "}
                            {move || if is_open.get() { t_string!(i18n, menu.statistics) } else { "" }}
                        </span>
                        {move || if !is_open.get() {
                            view! {
                                <span style=format!("color: {}; font-size: 0.75rem; margin-left: 0.5rem;", theme.ui_text_secondary)>
                                    {t_string!(i18n, menu.hotkey_statistics)}
                                </span>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </button>

                    <button
                        on:click=move |_| {
                            on_settings_open.run(());
                        }
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; cursor: pointer; display: flex; justify-content: space-between; align-items: center; transition: background 0.2s; min-width: 0;",
                            button_bg, theme.ui_text_primary
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        <span style=move || if is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"⚙️ "}
                            {move || if is_open.get() { t_string!(i18n, menu.settings) } else { "" }}
                        </span>
                        {move || if !is_open.get() {
                            view! {
                                <span style=format!("color: {}; font-size: 0.75rem; margin-left: 0.5rem;", theme.ui_text_secondary)>
                                    {t_string!(i18n, menu.hotkey_settings)}
                                </span>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}
