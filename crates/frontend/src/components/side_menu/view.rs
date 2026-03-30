use super::view_model::SideMenuViewModel;
use crate::config::Theme;
use crate::i18n::i18n::{t_string, use_i18n};
use leptos::prelude::*;

const MENU_BUTTON_FONT_SIZE: &str = "clamp(0.875rem, 0.84rem + 0.16vw, 1rem)";
const MENU_META_FONT_SIZE: &str = "clamp(0.72rem, 0.7rem + 0.12vw, 0.82rem)";
const MENU_TOGGLE_FONT_SIZE: &str = "clamp(1rem, 0.95rem + 0.28vw, 1.2rem)";

#[component]
pub fn SideMenu(
    #[prop(into)] is_open: RwSignal<bool>,
    on_chat_open: Callback<()>,
    on_notes_open: Callback<()>,
    on_scenes_open: Callback<()>,
    on_tokens_open: Callback<()>,
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
    let vm = SideMenuViewModel::new(is_open);

    let button_bg = theme.ui_button_primary;
    let button_hover = "#1d4ed8";
    let notification_color = theme.ui_notification;
    let form_bg = theme.ui_bg_primary;
    let menu_width_open = "15.625rem";
    let menu_width_closed = "3.75rem";

    view! {
        <div>
            // Hamburger toggle button
            <button
                on:click=move |_| vm.toggle()
                style=format!(
                    "position: fixed; top: 1.25rem; left: 1.25rem; z-index: 1000; \
                     padding: 0.625rem 0.9375rem; background: {}; color: {}; border: none; \
                     border-radius: 0.625rem; cursor: pointer; font-size: {};",
                    button_bg, theme.ui_text_primary, MENU_TOGGLE_FONT_SIZE
                )
            >
                "☰"
            </button>

            // Slide-in menu panel
            <div
                style=move || {
                    let left = if vm.is_open.get() {
                        "0".to_string()
                    } else {
                        format!("calc(-{} + {})", menu_width_open, menu_width_closed)
                    };
                    let (pr, pl) = if vm.is_open.get() { ("1.25rem", "1.25rem") } else { ("0.5rem", "0.5rem") };
                    format!(
                        "position: fixed; top: 0; left: {}; width: {}; height: 100vh; \
                         background: {}; box-shadow: 0.125rem 0 0.625rem rgba(0,0,0,0.3); \
                         transition: all 0.3s ease; z-index: 999; \
                         padding: 4.375rem {} 1.25rem {}; overflow: hidden;",
                        left, menu_width_open, form_bg, pr, pl
                    )
                }
            >
                <div style="display: flex; flex-direction: column; gap: 0.625rem;">

                    // Chat button
                    <button
                        on:click=move |_| on_chat_open.run(())
                        style=move || {
                            let bg = if has_chat_notification.get() { notification_color } else { button_bg };
                            format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; \
                                 cursor: pointer; display: flex; justify-content: space-between; align-items: center; \
                                 transition: background 0.2s; min-width: 0; font-size: {};",
                                bg, theme.ui_text_primary, MENU_BUTTON_FONT_SIZE
                            )
                        }
                        onmouseover=move || if has_chat_notification.get() {
                            format!("this.style.background='{}'", notification_color)
                        } else {
                            format!("this.style.background='{}'", button_hover)
                        }
                        onmouseout=move || if has_chat_notification.get() {
                            format!("this.style.background='{}'", notification_color)
                        } else {
                            format!("this.style.background='{}'", button_bg)
                        }
                    >
                        <span style=move || if vm.is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"💬 "}
                            {move || if vm.is_open.get() { t_string!(i18n, menu.chat) } else { "" }}
                        </span>
                        {move || if !vm.is_open.get() {
                            view! {
                                <span style=format!(
                                    "color: {}; font-size: {}; margin-left: 0.5rem;",
                                    theme.ui_text_secondary, MENU_META_FONT_SIZE
                                )>
                                    {if has_chat_notification.get() && chat_notification_count.get() > 0 {
                                        chat_notification_count.get().to_string()
                                    } else {
                                        t_string!(i18n, menu.hotkey_chat).to_string()
                                    }}
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </button>

                    // Notes button
                    <button
                        on:click=move |_| on_notes_open.run(())
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; \
                             cursor: pointer; display: flex; justify-content: space-between; align-items: center; \
                             transition: background 0.2s; min-width: 0; font-size: {};",
                            button_bg, theme.ui_text_primary, MENU_BUTTON_FONT_SIZE
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        <span style=move || if vm.is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"📝 "}
                            {move || if vm.is_open.get() { t_string!(i18n, menu.notes) } else { "" }}
                        </span>
                        {move || if !vm.is_open.get() {
                            view! {
                                <span style=format!(
                                    "color: {}; font-size: {}; margin-left: 0.5rem;",
                                    theme.ui_text_secondary, MENU_META_FONT_SIZE
                                )>
                                    {t_string!(i18n, menu.hotkey_notes)}
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </button>

                    // Scenes button
                    <button
                        on:click=move |_| on_scenes_open.run(())
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; \
                             cursor: pointer; display: flex; justify-content: space-between; align-items: center; \
                             transition: background 0.2s; min-width: 0; font-size: {};",
                            button_bg, theme.ui_text_primary, MENU_BUTTON_FONT_SIZE
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        <span style=move || if vm.is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"🗺️ "}
                            {move || if vm.is_open.get() { t_string!(i18n, menu.scenes) } else { "" }}
                        </span>
                        {move || if !vm.is_open.get() {
                            view! {
                                <span style=format!(
                                    "color: {}; font-size: {}; margin-left: 0.5rem;",
                                    theme.ui_text_secondary, MENU_META_FONT_SIZE
                                )>
                                    {t_string!(i18n, menu.hotkey_scenes)}
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </button>

                    // Tokens button
                    <button
                        on:click=move |_| on_tokens_open.run(())
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; \
                             cursor: pointer; display: flex; justify-content: space-between; align-items: center; \
                             transition: background 0.2s; min-width: 0; font-size: {};",
                            button_bg, theme.ui_text_primary, MENU_BUTTON_FONT_SIZE
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        <span style=move || if vm.is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"🧿 "}
                            {move || if vm.is_open.get() { t_string!(i18n, menu.tokens) } else { "" }}
                        </span>
                        {move || if !vm.is_open.get() {
                            view! {
                                <span style=format!(
                                    "color: {}; font-size: {}; margin-left: 0.5rem;",
                                    theme.ui_text_secondary, MENU_META_FONT_SIZE
                                )>
                                    {t_string!(i18n, menu.hotkey_tokens)}
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </button>

                    // Voting button
                    <button
                        on:click=move |_| on_voting_open.run(())
                        style=move || {
                            let bg = if has_statistics_notification.get() { notification_color } else { button_bg };
                            format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; \
                                 cursor: pointer; display: flex; justify-content: space-between; align-items: center; \
                                 transition: background 0.2s; min-width: 0; font-size: {};",
                                bg, theme.ui_text_primary, MENU_BUTTON_FONT_SIZE
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
                        <span style=move || if vm.is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"🗳️ "}
                            {move || if vm.is_open.get() { t_string!(i18n, menu.voting) } else { "" }}
                        </span>
                        {move || if !vm.is_open.get() {
                            view! {
                                <span style=format!(
                                    "color: {}; font-size: {}; margin-left: 0.5rem;",
                                    theme.ui_text_secondary, MENU_META_FONT_SIZE
                                )>
                                    {if has_statistics_notification.get() && notification_count.get() > 0 {
                                        notification_count.get().to_string()
                                    } else {
                                        t_string!(i18n, menu.hotkey_voting).to_string()
                                    }}
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </button>

                    // Statistics button
                    <button
                        on:click=move |_| on_statistics_open.run(())
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; \
                             cursor: pointer; display: flex; justify-content: space-between; align-items: center; \
                             transition: background 0.2s; min-width: 0; font-size: {};",
                            button_bg, theme.ui_text_primary, MENU_BUTTON_FONT_SIZE
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        <span style=move || if vm.is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"📊 "}
                            {move || if vm.is_open.get() { t_string!(i18n, menu.statistics) } else { "" }}
                        </span>
                        {move || if !vm.is_open.get() {
                            view! {
                                <span style=format!(
                                    "color: {}; font-size: {}; margin-left: 0.5rem;",
                                    theme.ui_text_secondary, MENU_META_FONT_SIZE
                                )>
                                    {t_string!(i18n, menu.hotkey_statistics)}
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </button>

                    // Settings button
                    <button
                        on:click=move |_| on_settings_open.run(())
                        style=format!(
                            "padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; \
                             cursor: pointer; display: flex; justify-content: space-between; align-items: center; \
                             transition: background 0.2s; min-width: 0; font-size: {};",
                            button_bg, theme.ui_text_primary, MENU_BUTTON_FONT_SIZE
                        )
                        onmouseover=format!("this.style.background='{}'", button_hover)
                        onmouseout=format!("this.style.background='{}'", button_bg)
                    >
                        <span style=move || if vm.is_open.get() {
                            "white-space: nowrap;".to_string()
                        } else {
                            "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;".to_string()
                        }>
                            {"⚙️ "}
                            {move || if vm.is_open.get() { t_string!(i18n, menu.settings) } else { "" }}
                        </span>
                        {move || if !vm.is_open.get() {
                            view! {
                                <span style=format!("color: {}; font-size: 0.75rem; margin-left: 0.5rem;", theme.ui_text_secondary)>
                                    {t_string!(i18n, menu.hotkey_settings)}
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </button>

                </div>
            </div>
        </div>
    }
}
