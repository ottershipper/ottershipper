use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Invalid name: {0}")]
    InvalidName(String),

    #[error("Name '{0}' already exists")]
    DuplicateName(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Validate application name
pub fn validate_app_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(DbError::InvalidName("name cannot be empty".to_string()));
    }

    if name.len() > 255 {
        return Err(DbError::InvalidName(
            "name cannot exceed 255 characters".to_string(),
        ));
    }

    // Must start with alphanumeric
    if !name.chars().next().unwrap().is_alphanumeric() {
        return Err(DbError::InvalidName(
            "name must start with alphanumeric character".to_string(),
        ));
    }

    // Only allow alphanumeric, hyphens, and underscores
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(DbError::InvalidName(
            "name can only contain alphanumeric characters, hyphens, and underscores".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_app_name() {
        // Valid names
        assert!(validate_app_name("my-app").is_ok());
        assert!(validate_app_name("my_app").is_ok());
        assert!(validate_app_name("app123").is_ok());
        assert!(validate_app_name("MyApp").is_ok());

        // Invalid names
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("-app").is_err());
        assert!(validate_app_name("_app").is_err());
        assert!(validate_app_name("my app").is_err());
        assert!(validate_app_name("my@app").is_err());
        assert!(validate_app_name(&"a".repeat(256)).is_err());
    }
}
