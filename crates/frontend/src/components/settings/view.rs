use super::view_model::{SettingsViewModel, apply_language_change};
use crate::config::Theme;
use crate::i18n::i18n::{Locale, t, use_i18n};
use leptos::prelude::*;

#[component]
pub fn Settings(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] show_workspace_hint: RwSignal<bool>,
    #[prop(into)] show_inactive_scene_contents: RwSignal<bool>,
    on_clear_room_local_state: Callback<()>,
    current_room: ReadSignal<String>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let vm = SettingsViewModel::new(is_open);
    let current_locale = i18n.get_locale();

    view! {
        <Show when=move || vm.is_open.get()>
            <div
                style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; \
                       background: rgba(0, 0, 0, 0.7); display: flex; align-items: center; \
                       justify-content: center; z-index: 2000;"
                on:click=move |_| vm.close()
            >
                <div
                    style=format!(
                        "background: {}; padding: 2.5rem; border-radius: 0.625rem; \
                         max-width: 31.25rem; width: 100%; position: relative;",
                        theme.ui_bg_primary
                    )
                    on:click=|ev| ev.stop_propagation()
                >
                    <h2 style=format!("color: {}; margin-bottom: 1.875rem;", theme.ui_text_primary)>
                        {t!(i18n, settings.title)}
                    </h2>

                    <div style="display: flex; flex-direction: column; gap: 1.25rem;">
                        <div style="display: flex; flex-direction: column; gap: 0.5rem;">
                            <label style=format!("color: {};", theme.ui_text_primary)>
                                {t!(i18n, settings.language)}
                            </label>
                            <select
                                on:change=move |ev| {
                                    apply_language_change(&event_target_value(&ev), i18n);
                                }
                                style=format!(
                                    "padding: 0.75rem; border-radius: 0.3125rem; border: 0.0625rem solid {}; \
                                     background: {}; color: {}; font-size: 1rem; cursor: pointer;",
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

                        <label style=format!(
                            "display: flex; gap: 0.75rem; align-items: flex-start; padding: 0.9rem 1rem; \
                             border-radius: 0.625rem; background: {}; color: {}; cursor: pointer;",
                            theme.ui_bg_secondary, theme.ui_text_primary
                        )>
                            <input
                                type="checkbox"
                                prop:checked=move || show_workspace_hint.get()
                                on:change=move |ev| show_workspace_hint.set(event_target_checked(&ev))
                                style="margin-top: 0.2rem; width: 1rem; height: 1rem;"
                            />
                            <span style="display: flex; flex-direction: column; gap: 0.3rem;">
                                <span>{t!(i18n, settings.show_workspace_hint)}</span>
                                <span style=format!("color: {}; font-size: 0.82rem; line-height: 1.45;", theme.ui_text_secondary)>
                                    {t!(i18n, settings.show_workspace_hint_hint)}
                                </span>
                            </span>
                        </label>

                        <label style=format!(
                            "display: flex; gap: 0.75rem; align-items: flex-start; padding: 0.9rem 1rem; \
                             border-radius: 0.625rem; background: {}; color: {}; cursor: pointer;",
                            theme.ui_bg_secondary, theme.ui_text_primary
                        )>
                            <input
                                type="checkbox"
                                prop:checked=move || show_inactive_scene_contents.get()
                                on:change=move |ev| show_inactive_scene_contents.set(event_target_checked(&ev))
                                style="margin-top: 0.2rem; width: 1rem; height: 1rem;"
                            />
                            <span style="display: flex; flex-direction: column; gap: 0.3rem;">
                                <span>{t!(i18n, settings.show_inactive_scene_contents)}</span>
                                <span style=format!("color: {}; font-size: 0.82rem; line-height: 1.45;", theme.ui_text_secondary)>
                                    {t!(i18n, settings.show_inactive_scene_contents_hint)}
                                </span>
                            </span>
                        </label>

                        <div style=format!(
                            "display: flex; flex-direction: column; gap: 0.55rem; padding: 0.9rem 1rem; \
                             border-radius: 0.625rem; background: {}; color: {};",
                            theme.ui_bg_secondary, theme.ui_text_primary
                        )>
                            <span>{t!(i18n, settings.clear_room_local_state)}</span>
                            <span style=format!("color: {}; font-size: 0.82rem; line-height: 1.45;", theme.ui_text_secondary)>
                                {t!(i18n, settings.clear_room_local_state_hint)}
                            </span>
                            <button
                                on:click=move |_| on_clear_room_local_state.run(())
                                prop:disabled=move || current_room.get().is_empty()
                                style=move || format!(
                                    "padding: 0.75rem; background: rgba(220,38,38,0.14); color: {}; border: none; \
                                     border-radius: 0.3125rem; font-size: 1rem; cursor: {}; font-weight: 700; opacity: {};",
                                    theme.ui_button_danger,
                                    if current_room.get().is_empty() { "not-allowed" } else { "pointer" },
                                    if current_room.get().is_empty() { "0.55" } else { "1" },
                                )
                            >
                                {t!(i18n, settings.clear_room_local_state_button)}
                            </button>
                        </div>

                        <button
                            on:click=move |_| vm.close()
                            style=format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; \
                                 border-radius: 0.3125rem; font-size: 1rem; cursor: pointer; font-weight: bold;",
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
