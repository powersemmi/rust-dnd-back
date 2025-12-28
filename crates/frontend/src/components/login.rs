use crate::config::Theme;
use gloo_net::http::Request;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use shared::auth::{LoginRequest, LoginResponse};

#[component]
pub fn LoginForm(
    on_login_success: Callback<String>, // Передаем JWT токен и username
    on_switch_to_register: Callback<()>,
    back_url: &'static str,
    api_path: &'static str,
    theme: Theme,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (code, set_code) = signal(String::new());
    let (error_message, set_error_message) = signal(Option::<String>::None);
    let (is_loading, set_is_loading) = signal(false);

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();

        let username_val = username.get();
        let code_val = code.get();

        if username_val.is_empty() {
            set_error_message.set(Some("Username cannot be empty".to_string()));
            return;
        }

        if code_val.is_empty() {
            set_error_message.set(Some("Code cannot be empty".to_string()));
            return;
        }

        if code_val.len() != 6 {
            set_error_message.set(Some("Code must be 6 digits".to_string()));
            return;
        }

        set_is_loading.set(true);
        set_error_message.set(None);

        leptos::task::spawn_local(async move {
            let url = format!("{}{}/auth/login", back_url, api_path);
            let payload = LoginRequest {
                username: username_val.clone(),
                code: code_val,
            };

            let result = Request::post(&url).json(&payload).unwrap().send().await;

            match result {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<LoginResponse>().await {
                            Ok(data) => {
                                // Сохраняем токен в localStorage
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        let _ = storage.set_item("jwt_token", &data.token);
                                        let _ = storage.set_item("username", &username_val);
                                    }
                                }
                                on_login_success.run(data.token);
                            }
                            Err(e) => {
                                set_error_message
                                    .set(Some(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let status = response.status();
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        set_error_message.set(Some(format!("Error {}: {}", status, error_text)));
                    }
                }
                Err(e) => {
                    set_error_message.set(Some(format!("Network error: {}", e)));
                }
            }
            set_is_loading.set(false);
        });
    };

    let form_bg = theme.auth_form_bg;
    let input_bg = theme.auth_input_bg;
    let input_border = theme.auth_input_border;
    let input_text = theme.auth_input_text;
    let error_bg = theme.auth_error_bg;
    let error_border = theme.auth_error_border;
    let error_text = theme.auth_error_text;
    let button_color = theme.auth_button_login;

    view! {
        <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; padding: 20px;">
            <div style=format!("background: {}; padding: 40px; border-radius: 10px; max-width: 400px; width: 100%;", form_bg)>
                <h1 style="color: white; text-align: center; margin-bottom: 30px;">"Login"</h1>

                <form on:submit=on_submit style="display: flex; flex-direction: column; gap: 20px;">
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        <label style="color: #ccc;">"Username"</label>
                        <input
                            type="text"
                            value=move || username.get()
                            on:input=move |ev| set_username.set(event_target_value(&ev))
                            placeholder="Enter your username"
                            disabled=move || is_loading.get()
                            style=format!("padding: 12px; border-radius: 5px; border: 1px solid {}; background: {}; color: {}; font-size: 16px;", input_border, input_bg, input_text)
                        />
                    </div>

                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        <label style="color: #ccc;">"Authenticator Code"</label>
                        <input
                            type="text"
                            value=move || code.get()
                            on:input=move |ev| set_code.set(event_target_value(&ev))
                            placeholder="000000"
                            maxlength="6"
                            disabled=move || is_loading.get()
                            style=format!("padding: 12px; border-radius: 5px; border: 1px solid {}; background: {}; color: {}; font-size: 16px; letter-spacing: 4px; text-align: center;", input_border, input_bg, input_text)
                        />
                    </div>

                    <Show when=move || error_message.get().is_some()>
                        <div style=format!("padding: 12px; background: {}; border: 1px solid {}; border-radius: 5px; color: {};", error_bg, error_border, error_text)>
                            {move || error_message.get().unwrap_or_default()}
                        </div>
                    </Show>

                    <button
                        type="submit"
                        disabled=move || is_loading.get()
                        style=format!("padding: 12px; background: {}; color: white; border: none; border-radius: 5px; font-size: 16px; cursor: pointer; font-weight: bold;", button_color)
                    >
                        {move || if is_loading.get() { "Logging in..." } else { "Login" }}
                    </button>

                    <button
                        type="button"
                        on:click=move |_| on_switch_to_register.run(())
                        style=format!("padding: 12px; background: transparent; color: {}; border: 1px solid {}; border-radius: 5px; font-size: 16px; cursor: pointer;", button_color, button_color)
                    >
                        "Don't have an account? Register"
                    </button>
                </form>
            </div>
        </div>
    }
}
