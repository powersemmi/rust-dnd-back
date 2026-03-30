use crate::i18n::i18n::Locale;

/// Parse a locale string from a select element value.
pub fn parse_locale(value: &str) -> Locale {
    if value == "ru" {
        Locale::ru
    } else {
        Locale::en
    }
}

/// Persist locale to localStorage.
pub fn save_locale_to_storage(value: &str) {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let _ = storage.set_item("locale", value);
    }
}

/// Convert a Locale to its string code.
pub fn locale_to_code(locale: Locale) -> &'static str {
    match locale {
        Locale::en => "en",
        Locale::ru => "ru",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ru_returns_ru_locale() {
        assert!(matches!(parse_locale("ru"), Locale::ru));
    }

    #[test]
    fn parse_en_returns_en_locale() {
        assert!(matches!(parse_locale("en"), Locale::en));
    }

    #[test]
    fn parse_unknown_falls_back_to_en() {
        assert!(matches!(parse_locale("zz"), Locale::en));
    }

    #[test]
    fn locale_to_code_roundtrip() {
        assert_eq!(locale_to_code(Locale::en), "en");
        assert_eq!(locale_to_code(Locale::ru), "ru");
    }
}
