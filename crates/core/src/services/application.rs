use ottershipper_db::{Application, Database, DbError};

/// Service for application-related business logic
///
/// This service wraps the database repository and provides
/// a clean interface for application operations with validation
/// and business logic.
#[derive(Clone)]
pub struct ApplicationService {
    db: Database,
}

impl ApplicationService {
    /// Create a new `ApplicationService`
    #[must_use]
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create a new application
    ///
    /// # Arguments
    /// * `name` - Application name (alphanumeric, hyphens, underscores, max 255 chars)
    ///
    /// # Returns
    /// * `Ok(Application)` - Successfully created application with id and timestamp
    /// * `Err(DbError::InvalidName)` - Name validation failed
    /// * `Err(DbError::DuplicateName)` - Application with this name already exists
    ///
    /// # Examples
    /// ```ignore
    /// let service = ApplicationService::new(db);
    /// let app = service.create_app("my-app".to_string()).await?;
    /// println!("Created app: {} with id {}", app.name, app.id);
    /// ```
    pub async fn create_app(&self, name: String) -> Result<Application, DbError> {
        // Validation and creation is handled by the repository
        self.db.applications().create(&name).await
    }

    /// Get application by ID
    pub async fn get_app(&self, id: &str) -> Result<Option<Application>, DbError> {
        self.db.applications().get(id).await
    }

    /// Get application by name
    pub async fn get_app_by_name(&self, name: &str) -> Result<Option<Application>, DbError> {
        self.db.applications().get_by_name(name).await
    }

    /// List all applications
    pub async fn list_apps(&self) -> Result<Vec<Application>, DbError> {
        self.db.applications().list().await
    }

    /// Delete application by ID
    pub async fn delete_app(&self, id: &str) -> Result<bool, DbError> {
        self.db.applications().delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn setup_test_service() -> Result<ApplicationService, Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).await?;
        db.migrate().await?;
        Ok(ApplicationService::new(db))
    }

    /// Test that ApplicationService correctly integrates with Database layer
    /// This verifies the service layer properly delegates to DB and returns results
    #[tokio::test]
    async fn test_service_integration() -> Result<(), Box<dyn std::error::Error>> {
        let service = setup_test_service().await?;

        // Test create
        let app = service.create_app("integration-test".to_string()).await?;
        assert_eq!(app.name, "integration-test");
        assert!(!app.id.is_empty());

        // Test get by id
        let fetched = service.get_app(&app.id).await?;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "integration-test");

        // Test get by name
        let by_name = service.get_app_by_name("integration-test").await?;
        assert!(by_name.is_some());

        // Test list
        let apps = service.list_apps().await?;
        assert_eq!(apps.len(), 1);

        // Test delete
        let deleted = service.delete_app(&app.id).await?;
        assert!(deleted);
        assert!(service.get_app(&app.id).await?.is_none());

        Ok(())
    }

    /// Test that errors from DB layer are properly propagated
    #[tokio::test]
    async fn test_service_error_propagation() -> Result<(), Box<dyn std::error::Error>> {
        let service = setup_test_service().await?;

        // Test validation error propagation
        let result = service.create_app("invalid name".to_string()).await;
        assert!(matches!(result, Err(DbError::InvalidName(_))));

        // Test duplicate name error propagation
        service.create_app("duplicate".to_string()).await?;
        let result = service.create_app("duplicate".to_string()).await;
        assert!(matches!(result, Err(DbError::DuplicateName(_))));

        Ok(())
    }
}
