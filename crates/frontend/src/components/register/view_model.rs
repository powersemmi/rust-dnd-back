use super::model::{RegisterFormInput, RegisterValidationError};
use gloo_net::http::Request;
use leptos::prelude::*;
use shared::auth::{RegisterRequest, RegisterResponse};

/// Reactive state and business logic for the registration form.
#[derive(Clone, Copy)]
pub struct RegisterViewModel {
    pub username: RwSignal<String>,
    pub error_message: RwSignal<Option<String>>,
    pub qr_code_data: RwSignal<Option<String>>,
    pub is_loading: RwSignal<bool>,
    pub back_url: &'static str,
    pub api_path: &'static str,
}

impl RegisterViewModel {
    pub fn new(back_url: &'static str, api_path: &'static str) -> Self {
        Self {
            username: RwSignal::new(String::new()),
            error_message: RwSignal::new(None),
            qr_code_data: RwSignal::new(None),
            is_loading: RwSignal::new(false),
            back_url,
            api_path,
        }
    }

    /// Validates the form. Returns `Some(input)` if valid.
    pub fn validate(&self) -> Option<RegisterFormInput> {
        let input = RegisterFormInput {
            username: self.username.get_untracked(),
        };
        match input.validate() {
            Ok(()) => Some(input),
            Err(RegisterValidationError::EmptyUsername) => {
                self.error_message.set(Some("Username is required.".into()));
                None
            }
        }
    }

    /// Spawns the async HTTP registration request.
    /// On success, sets `qr_code_data` with the base64 QR image.
    pub fn submit(&self) {
        let Some(input) = self.validate() else { return };

        self.is_loading.set(true);
        self.error_message.set(None);

        let back_url = self.back_url;
        let api_path = self.api_path;
        let is_loading = self.is_loading;
        let error_message = self.error_message;
        let qr_code_data = self.qr_code_data;

        leptos::task::spawn_local(async move {
            let url = format!("{}{}/auth/register", back_url, api_path);
            let payload = RegisterRequest {
                username: input.username,
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
                        match response.json::<RegisterResponse>().await {
                            Ok(data) => {
                                qr_code_data.set(Some(data.qr_code_base64));
                                error_message.set(None);
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
    fn validate_sets_error_on_empty_username() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = RegisterViewModel::new("http://localhost", "/api");
            assert!(vm.validate().is_none());
            assert!(vm.error_message.get_untracked().is_some());
        });
    }

    #[test]
    fn validate_succeeds_with_username() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = RegisterViewModel::new("http://localhost", "/api");
            vm.username.set("bob".into());
            let result = vm.validate();
            assert!(result.is_some());
            assert_eq!(result.unwrap().username, "bob");
        });
    }

    #[test]
    fn qr_code_starts_as_none() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = RegisterViewModel::new("http://localhost", "/api");
            assert!(vm.qr_code_data.get_untracked().is_none());
        });
    }
}
