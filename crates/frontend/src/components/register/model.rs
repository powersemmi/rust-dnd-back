/// Pure domain data and validation for the registration form.

#[derive(Clone, Debug, Default)]
pub struct RegisterFormInput {
    pub username: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegisterValidationError {
    EmptyUsername,
}

impl RegisterFormInput {
    pub fn validate(&self) -> Result<(), RegisterValidationError> {
        if self.username.is_empty() {
            return Err(RegisterValidationError::EmptyUsername);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_username_fails() {
        let input = RegisterFormInput {
            username: "".into(),
        };
        assert_eq!(
            input.validate(),
            Err(RegisterValidationError::EmptyUsername)
        );
    }

    #[test]
    fn non_empty_username_passes() {
        let input = RegisterFormInput {
            username: "bob".into(),
        };
        assert!(input.validate().is_ok());
    }
}
