use super::view_model::RegisterViewModel;
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;

#[component]
pub fn RegisterForm(
    on_registered: Callback<()>,
    on_switch_to_login: Callback<()>,
    back_url: &'static str,
    api_path: &'static str,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let vm = RegisterViewModel::new(back_url, api_path);

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        vm.submit();
    };

    view! {
        <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; padding: 1.25rem;">
            <div style=format!("background: {}; padding: 2.5rem; border-radius: 0.625rem; max-width: 31.25rem; width: 100%;", theme.ui_bg_primary)>
                <h1 style=format!("color: {}; text-align: center; margin-bottom: 1.875rem;", theme.ui_text_primary)>
                    {t!(i18n, auth.register.title)}
                </h1>

                // Form shown while QR code is not yet received
                <Show when=move || vm.qr_code_data.get().is_none()>
                    <form on:submit=on_submit style="display: flex; flex-direction: column; gap: 1.25rem;">
                        <div style="display: flex; flex-direction: column; gap: 0.5rem;">
                            <label style=format!("color: {};", theme.ui_text_primary)>{t!(i18n, auth.register.username)}</label>
                            <input
                                type="text"
                                value=move || vm.username.get()
                                on:input=move |ev| vm.username.set(event_target_value(&ev))
                                placeholder=move || t_string!(i18n, auth.register.username).to_string()
                                disabled=move || vm.is_loading.get()
                                style=format!(
                                    "padding: 0.75rem; border-radius: 0.3125rem; border: 0.0625rem solid {}; \
                                     background: {}; color: {}; font-size: 1rem;",
                                    theme.ui_border, theme.ui_bg_secondary, theme.ui_text_primary
                                )
                            />
                        </div>

                        <Show when=move || vm.error_message.get().is_some()>
                            <div style=format!(
                                "padding: 0.75rem; background: {}; border: 0.0625rem solid {}; \
                                 border-radius: 0.3125rem; color: {};",
                                theme.ui_bg_secondary, theme.ui_button_danger, theme.ui_button_danger
                            )>
                                {move || vm.error_message.get().unwrap_or_default()}
                            </div>
                        </Show>

                        <button
                            type="submit"
                            disabled=move || vm.is_loading.get()
                            style=format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; \
                                 border-radius: 0.3125rem; font-size: 1rem; cursor: pointer; font-weight: bold;",
                                theme.ui_button_primary, theme.ui_text_primary
                            )
                        >
                            {t!(i18n, auth.register.button)}
                        </button>

                        <button
                            type="button"
                            on:click=move |_| on_switch_to_login.run(())
                            style=format!(
                                "padding: 0.75rem; background: {}; color: {}; border: none; \
                                 border-radius: 0.3125rem; font-size: 1rem; cursor: pointer; font-weight: bold;",
                                theme.ui_bg_secondary, theme.ui_text_primary
                            )
                        >
                            {t!(i18n, auth.register.switch_to_login)}
                        </button>
                    </form>
                </Show>

                // QR code shown after successful registration
                <Show when=move || vm.qr_code_data.get().is_some()>
                    <div style="display: flex; flex-direction: column; align-items: center; gap: 1.25rem;">
                        <div style=format!(
                            "color: {}; text-align: center; padding: 0.75rem; background: {}; \
                             border: 0.0625rem solid {}; border-radius: 0.3125rem;",
                            theme.ui_success, theme.ui_bg_secondary, theme.ui_success
                        )>
                            {t!(i18n, auth.register.success)}
                        </div>

                        <div style="background: white; padding: 1.25rem; border-radius: 0.625rem;">
                            <img
                                src=move || format!(
                                    "data:image/png;base64,{}",
                                    vm.qr_code_data.get().unwrap_or_default()
                                )
                                alt="QR Code"
                                style="max-width: 18.75rem; display: block;"
                            />
                        </div>

                        <p style=format!("color: {}; text-align: center;", theme.ui_text_primary)>
                            {t!(i18n, auth.register.qr_instruction)}
                        </p>

                        <button
                            on:click=move |_| on_registered.run(())
                            style=format!(
                                "padding: 0.75rem 1.5rem; background: {}; color: {}; border: none; \
                                 border-radius: 0.3125rem; font-size: 1rem; cursor: pointer; font-weight: bold;",
                                theme.ui_button_primary, theme.ui_text_primary
                            )
                        >
                            {t!(i18n, auth.register.back)}
                        </button>
                    </div>
                </Show>
            </div>
        </div>
    }
}
