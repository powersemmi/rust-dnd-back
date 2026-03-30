use crate::i18n::i18n::Locale;
use leptos::prelude::*;

/// Reactive state and logic for the settings modal.
#[derive(Clone, Copy)]
pub struct SettingsViewModel {
    pub is_open: RwSignal<bool>,
}

impl SettingsViewModel {
    pub fn new(is_open: RwSignal<bool>) -> Self {
        Self { is_open }
    }

    pub fn close(&self) {
        self.is_open.set(false);
    }
}

/// Persist locale selection and update i18n context.
pub fn apply_language_change(
    value: &str,
    i18n: leptos_i18n::I18nContext<crate::i18n::i18n::Locale>,
) {
    let locale = if value == "ru" {
        Locale::ru
    } else {
        Locale::en
    };
    i18n.set_locale(locale);
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let _ = storage.set_item("locale", value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn close_sets_is_open_to_false() {
        let owner = Owner::new();
        owner.with(|| {
            let is_open = RwSignal::new(true);
            let vm = SettingsViewModel::new(is_open);
            vm.close();
            assert!(!vm.is_open.get_untracked());
        });
    }
}
