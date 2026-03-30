use super::model::locale_to_code;
use crate::i18n::i18n::Locale;
use leptos::prelude::*;

/// Reactive state for the language selector widget.
#[derive(Clone, Copy)]
pub struct LanguageSelectorViewModel {
    pub current_locale: RwSignal<Locale>,
}

impl LanguageSelectorViewModel {
    pub fn new(initial: Locale) -> Self {
        Self {
            current_locale: RwSignal::new(initial),
        }
    }

    /// Returns the current locale's string code ("en" or "ru").
    pub fn locale_code(&self) -> &'static str {
        locale_to_code(self.current_locale.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn initial_locale_is_reflected_in_code() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = LanguageSelectorViewModel::new(Locale::ru);
            assert_eq!(vm.locale_code(), "ru");
        });
    }

    #[test]
    fn updating_locale_changes_code() {
        let owner = Owner::new();
        owner.with(|| {
            let vm = LanguageSelectorViewModel::new(Locale::en);
            vm.current_locale.set(Locale::ru);
            assert_eq!(vm.locale_code(), "ru");
        });
    }
}
