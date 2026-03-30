use super::{AppState, model::ActiveWindow};
use leptos::prelude::*;

/// Central reactive state for the application shell.
///
/// Owns visibility signals for all floating windows and the active-window
/// tracker used for keyboard shortcuts and z-ordering.
#[derive(Clone, Copy)]
pub struct AppViewModel {
    pub app_state: RwSignal<AppState>,

    // Window visibility
    pub is_menu_open: RwSignal<bool>,
    pub is_chat_open: RwSignal<bool>,
    pub is_scenes_open: RwSignal<bool>,
    pub is_tokens_open: RwSignal<bool>,
    pub is_settings_open: RwSignal<bool>,
    pub is_statistics_open: RwSignal<bool>,
    pub is_voting_open: RwSignal<bool>,

    // Focus tracking
    pub active_window: RwSignal<ActiveWindow>,

    // Notification badges
    pub has_statistics_notification: RwSignal<bool>,
    pub notification_count: RwSignal<u32>,
    pub has_chat_notification: RwSignal<bool>,
    pub chat_notification_count: RwSignal<u32>,
}

impl AppViewModel {
    pub fn new(initial_state: AppState) -> Self {
        Self {
            app_state: RwSignal::new(initial_state),
            is_menu_open: RwSignal::new(false),
            is_chat_open: RwSignal::new(false),
            is_scenes_open: RwSignal::new(false),
            is_tokens_open: RwSignal::new(false),
            is_settings_open: RwSignal::new(false),
            is_statistics_open: RwSignal::new(false),
            is_voting_open: RwSignal::new(false),
            active_window: RwSignal::new(ActiveWindow::None),
            has_statistics_notification: RwSignal::new(false),
            notification_count: RwSignal::new(0),
            has_chat_notification: RwSignal::new(false),
            chat_notification_count: RwSignal::new(0),
        }
    }

    // --- Window open helpers ---

    pub fn open_chat(&self) {
        self.is_chat_open.set(true);
        self.active_window.set(ActiveWindow::Chat);
        self.has_chat_notification.set(false);
        self.chat_notification_count.set(0);
    }

    pub fn open_scenes(&self) {
        self.is_scenes_open.set(true);
        self.active_window.set(ActiveWindow::Scenes);
    }

    pub fn open_tokens(&self) {
        self.is_tokens_open.set(true);
        self.active_window.set(ActiveWindow::Tokens);
    }

    pub fn open_settings(&self) {
        self.is_settings_open.set(true);
        self.active_window.set(ActiveWindow::Settings);
    }

    pub fn open_statistics(&self) {
        self.is_statistics_open.set(true);
        self.active_window.set(ActiveWindow::Statistics);
    }

    pub fn open_voting(&self) {
        self.is_voting_open.set(true);
        self.active_window.set(ActiveWindow::Voting);
        self.has_statistics_notification.set(false);
        self.notification_count.set(0);
    }

    /// Closes the currently active floating window (ESC key behavior).
    pub fn close_active_window(&self) {
        match self.active_window.get_untracked() {
            ActiveWindow::Chat => self.is_chat_open.set(false),
            ActiveWindow::Scenes => self.is_scenes_open.set(false),
            ActiveWindow::Tokens => self.is_tokens_open.set(false),
            ActiveWindow::Settings => self.is_settings_open.set(false),
            ActiveWindow::Voting => self.is_voting_open.set(false),
            ActiveWindow::Statistics => self.is_statistics_open.set(false),
            ActiveWindow::None => {}
        }
    }

    /// Handles a keyboard shortcut by key code.
    /// Returns `true` if the key was handled (prevents default browser action).
    pub fn handle_hotkey(&self, code: &str) -> bool {
        match code {
            "Escape" => {
                self.close_active_window();
                true
            }
            "KeyC" => {
                self.open_chat();
                true
            }
            "KeyG" => {
                self.open_scenes();
                true
            }
            "KeyT" => {
                self.open_tokens();
                true
            }
            "KeyS" => {
                self.open_settings();
                true
            }
            "KeyV" => {
                self.open_voting();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    fn make_vm() -> AppViewModel {
        AppViewModel::new(AppState::Login)
    }

    #[test]
    fn open_chat_sets_active_window_and_clears_notification() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.has_chat_notification.set(true);
            vm.chat_notification_count.set(3);
            vm.open_chat();
            assert!(vm.is_chat_open.get_untracked());
            assert_eq!(vm.active_window.get_untracked(), ActiveWindow::Chat);
            assert!(!vm.has_chat_notification.get_untracked());
            assert_eq!(vm.chat_notification_count.get_untracked(), 0);
        });
    }

    #[test]
    fn open_voting_clears_statistics_notification() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.has_statistics_notification.set(true);
            vm.notification_count.set(5);
            vm.open_voting();
            assert!(vm.is_voting_open.get_untracked());
            assert!(!vm.has_statistics_notification.get_untracked());
            assert_eq!(vm.notification_count.get_untracked(), 0);
        });
    }

    #[test]
    fn handle_hotkey_key_c_opens_chat() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            let handled = vm.handle_hotkey("KeyC");
            assert!(handled);
            assert!(vm.is_chat_open.get_untracked());
        });
    }

    #[test]
    fn handle_hotkey_escape_closes_active_chat() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.open_chat();
            let handled = vm.handle_hotkey("Escape");
            assert!(handled);
            assert!(!vm.is_chat_open.get_untracked());
        });
    }

    #[test]
    fn handle_hotkey_unknown_key_returns_false() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            let handled = vm.handle_hotkey("KeyZ");
            assert!(!handled);
        });
    }

    #[test]
    fn handle_hotkey_key_t_opens_tokens() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            let handled = vm.handle_hotkey("KeyT");
            assert!(handled);
            assert!(vm.is_tokens_open.get_untracked());
            assert_eq!(vm.active_window.get_untracked(), ActiveWindow::Tokens);
        });
    }

    #[test]
    fn close_active_when_none_is_noop() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = make_vm();
            vm.close_active_window(); // should not panic
            assert!(!vm.is_chat_open.get_untracked());
        });
    }
}
