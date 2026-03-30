/// Pure domain data and validation for the login form.
/// No signals, no Leptos imports.

#[derive(Clone, Debug, Default)]
pub struct LoginFormInput {
    pub username: String,
    pub code: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoginValidationError {
    EmptyFields,
    InvalidCodeLength,
}

impl LoginFormInput {
    pub fn validate(&self) -> Result<(), LoginValidationError> {
        if self.username.is_empty() || self.code.is_empty() {
            return Err(LoginValidationError::EmptyFields);
        }
        if self.code.len() != 6 {
            return Err(LoginValidationError::InvalidCodeLength);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_username_fails() {
        let input = LoginFormInput {
            username: "".into(),
            code: "123456".into(),
        };
        assert_eq!(input.validate(), Err(LoginValidationError::EmptyFields));
    }

    #[test]
    fn empty_code_fails() {
        let input = LoginFormInput {
            username: "alice".into(),
            code: "".into(),
        };
        assert_eq!(input.validate(), Err(LoginValidationError::EmptyFields));
    }

    #[test]
    fn short_code_fails() {
        let input = LoginFormInput {
            username: "alice".into(),
            code: "123".into(),
        };
        assert_eq!(
            input.validate(),
            Err(LoginValidationError::InvalidCodeLength)
        );
    }

    #[test]
    fn long_code_fails() {
        let input = LoginFormInput {
            username: "alice".into(),
            code: "1234567".into(),
        };
        assert_eq!(
            input.validate(),
            Err(LoginValidationError::InvalidCodeLength)
        );
    }

    #[test]
    fn valid_input_passes() {
        let input = LoginFormInput {
            username: "alice".into(),
            code: "123456".into(),
        };
        assert!(input.validate().is_ok());
    }
}
