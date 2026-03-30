/// Pure domain data and validation for the room selection form.

#[derive(Clone, Debug, Default)]
pub struct RoomInput {
    pub room_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RoomValidationError {
    EmptyRoomId,
}

impl RoomInput {
    pub fn validate(&self) -> Result<(), RoomValidationError> {
        if self.room_id.is_empty() {
            return Err(RoomValidationError::EmptyRoomId);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_room_id_fails() {
        let input = RoomInput { room_id: "".into() };
        assert_eq!(input.validate(), Err(RoomValidationError::EmptyRoomId));
    }

    #[test]
    fn non_empty_room_id_passes() {
        let input = RoomInput {
            room_id: "my-room".into(),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn whitespace_only_room_id_is_technically_valid() {
        // Whitespace is allowed at the model level; the UI may strip it.
        let input = RoomInput {
            room_id: "   ".into(),
        };
        assert!(input.validate().is_ok());
    }
}
