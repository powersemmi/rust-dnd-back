use crate::components::scene_board::model::{
    MAX_TOKEN_SIZE_CELLS, MIN_TOKEN_SIZE_CELLS, clamp_token_dimension,
};

pub const DEFAULT_TOKEN_WIDTH_CELLS: u16 = 1;
pub const DEFAULT_TOKEN_HEIGHT_CELLS: u16 = 1;
pub const TOKEN_IMAGE_ACCEPT: &str = "image/png,image/jpeg,image/webp,image/gif";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenLibraryValidationError {
    EmptyName,
    MissingImage,
    InvalidDimensions,
}

pub fn validate_token_form(
    name: &str,
    width_cells: &str,
    height_cells: &str,
    has_image: bool,
) -> Result<(u16, u16), TokenLibraryValidationError> {
    if name.trim().is_empty() {
        return Err(TokenLibraryValidationError::EmptyName);
    }

    if !has_image {
        return Err(TokenLibraryValidationError::MissingImage);
    }

    let Ok(width_cells) = width_cells.parse::<u16>() else {
        return Err(TokenLibraryValidationError::InvalidDimensions);
    };
    let Ok(height_cells) = height_cells.parse::<u16>() else {
        return Err(TokenLibraryValidationError::InvalidDimensions);
    };

    if !(MIN_TOKEN_SIZE_CELLS..=MAX_TOKEN_SIZE_CELLS).contains(&width_cells)
        || !(MIN_TOKEN_SIZE_CELLS..=MAX_TOKEN_SIZE_CELLS).contains(&height_cells)
    {
        return Err(TokenLibraryValidationError::InvalidDimensions);
    }

    Ok((
        clamp_token_dimension(width_cells),
        clamp_token_dimension(height_cells),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_token_form_accepts_valid_values() {
        assert_eq!(validate_token_form("Goblin", "2", "3", true), Ok((2, 3)));
    }

    #[test]
    fn validate_token_form_rejects_empty_name() {
        assert_eq!(
            validate_token_form("   ", "1", "1", true),
            Err(TokenLibraryValidationError::EmptyName)
        );
    }

    #[test]
    fn validate_token_form_rejects_missing_image() {
        assert_eq!(
            validate_token_form("Goblin", "1", "1", false),
            Err(TokenLibraryValidationError::MissingImage)
        );
    }

    #[test]
    fn validate_token_form_rejects_out_of_range_dimensions() {
        assert_eq!(
            validate_token_form("Goblin", "99", "1", true),
            Err(TokenLibraryValidationError::InvalidDimensions)
        );
    }
}
