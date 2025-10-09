use crate::error::{validate_app_name, DbError, Result};
use crate::models::Application;
use crate::Database;

/// Repository for application-related database operations
pub struct ApplicationRepository<'a> {
    db: &'a Database,
}

impl<'a> ApplicationRepository<'a> {
    /// Create a new `ApplicationRepository`
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new application
    pub async fn create(&self, name: &str) -> Result<Application> {
        // Validate name
        validate_app_name(name)?;

        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp_millis();

        sqlx::query_as::<_, Application>(
            "INSERT INTO applications (id, name, created_at) VALUES (?, ?, ?) RETURNING *",
        )
        .bind(&id)
        .bind(name)
        .bind(created_at)
        .fetch_one(&self.db.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                // Check for UNIQUE constraint violation (SQLITE_CONSTRAINT_UNIQUE = 2067)
                if let Some(code) = db_err.code() {
                    if code == "2067" {
                        return DbError::DuplicateName(name.to_string());
                    }
                }
            }
            DbError::DatabaseError(e)
        })
    }

    /// Get application by ID
    pub async fn get(&self, id: &str) -> Result<Option<Application>> {
        sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await
            .map_err(Into::into)
    }

    /// Get application by name
    pub async fn get_by_name(&self, name: &str) -> Result<Option<Application>> {
        sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.db.pool)
            .await
            .map_err(Into::into)
    }

    /// List all applications
    pub async fn list(&self) -> Result<Vec<Application>> {
        sqlx::query_as::<_, Application>(
            "SELECT * FROM applications ORDER BY created_at DESC, name ASC",
        )
        .fetch_all(&self.db.pool)
        .await
        .map_err(Into::into)
    }

    /// Delete application by ID
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM applications WHERE id = ?")
            .bind(id)
            .execute(&self.db.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
