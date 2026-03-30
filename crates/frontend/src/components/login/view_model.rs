use super::model::{LoginFormInput, LoginValidationError};
use gloo_net::http::Request;
use leptos::prelude::*;
use shared::auth::{LoginRequest, LoginResponse};

/// Reactive state and business logic for the login form.
#[derive(Clone, Copy)]
pub struct LoginViewModel {
    pub username: RwSignal<String>,
    pub code: RwSignal<String>,
    pub error_message: RwSignal<Option<String>>,
    pub is_loading: RwSignal<bool>,
    pub back_url: &'static str,
    pub api_path: &'static str,
}

impl LoginViewModel {
    pub fn new(back_url: &'static str, api_path: &'static str) -> Self {
        Self {
            username: RwSignal::new(String::new()),
            code: RwSignal::new(String::new()),
            error_message: RwSignal::new(None),
            is_loading: RwSignal::new(false),
            back_url,
            api_path,
        }
    }

    /// Validates form inputs. Returns `Some(input)` on success,
    /// `None` if invalid (error_message signal is updated).
    pub fn validate(&self) -> Option<LoginFormInput> {
        let input = LoginFormInput {
            username: self.username.get_untracked(),
            code: self.code.get_untracked(),
        };
        match input.validate() {
            Ok(()) => Some(input),
            Err(LoginValidationError::EmptyFields) => {
                self.error_message
                    .set(Some("Username and code are required.".into()));
                None
            }
            Err(LoginValidationError::InvalidCodeLength) => {
                self.error_message
                    .set(Some("Code must be exactly 6 digits.".into()));
                None
            }
        }
    }

    /// Spawns an async HTTP login request. Calls `on_success` with the JWT token on success.
    pub fn submit(&self, on_success: Callback<String>) {
        let Some(input) = self.validate() else { return };

        self.is_loading.set(true);
        self.error_message.set(None);

        let back_url = self.back_url;
        let api_path = self.api_path;
        let is_loading = self.is_loading;
        let error_message = self.error_message;

        leptos::task::spawn_local(async move {
            let url = format!("{}{}/auth/login", back_url, api_path);
            let payload = LoginRequest {
                username: input.username.clone(),
                code: input.code,
            };

            let result = match Request::post(&url).json(&payload) {
                Ok(req) => req.send().await,
                Err(e) => {
                    error_message.set(Some(format!("Failed to serialize request: {}", e)));
                    is_loading.set(false);
                    return;
                }
            };

            match result {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<LoginResponse>().await {
                            Ok(data) => {
                                if let Some(window) = web_sys::window()
                                    && let Ok(Some(storage)) = window.local_storage()
                                {
                                    let _ = storage.set_item("jwt_token", &data.token);
                                    let _ = storage.set_item("username", &input.username);
                                }
                                on_success.run(data.token);
                            }
                            Err(e) => {
                                error_message.set(Some(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let status = response.status();
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        error_message.set(Some(format!("Error {}: {}", status, error_text)));
                    }
                }
                Err(e) => {
                    error_message.set(Some(format!("Network error: {}", e)));
                }
            }
            is_loading.set(false);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn validate_returns_none_on_empty_inputs() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = LoginViewModel::new("http://localhost", "/api");
            let result = vm.validate();
            assert!(result.is_none());
            assert!(vm.error_message.get_untracked().is_some());
        });
    }

    #[test]
    fn validate_returns_none_on_short_code() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = LoginViewModel::new("http://localhost", "/api");
            vm.username.set("alice".into());
            vm.code.set("123".into());
            let result = vm.validate();
            assert!(result.is_none());
            let msg = vm.error_message.get_untracked();
            assert!(msg.is_some());
            assert!(msg.unwrap().contains("6 digits"));
        });
    }

    #[test]
    fn validate_returns_input_on_valid_data() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = LoginViewModel::new("http://localhost", "/api");
            vm.username.set("alice".into());
            vm.code.set("123456".into());
            let result = vm.validate();
            assert!(result.is_some());
            let input = result.unwrap();
            assert_eq!(input.username, "alice");
            assert_eq!(input.code, "123456");
        });
    }
}
