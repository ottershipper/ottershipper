mod error;
mod models;
mod repositories;

pub use error::{DbError, Result};
pub use models::Application;
pub use repositories::ApplicationRepository;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use tracing::info;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Enable `SQLite` write-ahead logging for better concurrency
    pub enable_wal: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_connections: 5,
            enable_wal: true,
        }
    }
}

/// Database connection pool
#[derive(Clone)]
pub struct Database {
    pub(crate) pool: SqlitePool,
}

impl Database {
    /// Create a new database connection with default config
    pub async fn new(database_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config(database_path, DatabaseConfig::default()).await
    }

    /// Create a new database connection with custom config
    pub async fn new_with_config(
        database_path: impl AsRef<Path>,
        config: DatabaseConfig,
    ) -> Result<Self> {
        let database_url = format!("sqlite:{}", database_path.as_ref().display());

        let mut options = SqliteConnectOptions::new()
            .filename(&database_path)
            .create_if_missing(true);

        // Enable WAL mode for better concurrency
        if config.enable_wal {
            options = options.pragma("journal_mode", "WAL");
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .connect_with(options)
            .await?;

        info!(
            "Connected to database at {} (max_connections: {}, wal: {})",
            database_url, config.max_connections, config.enable_wal
        );

        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        info!("Running database migrations...");

        // Create migrations tracking table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _migrations (
                id INTEGER PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                applied_at INTEGER NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;

        // Check if migration already applied
        let applied: Option<(String,)> =
            sqlx::query_as("SELECT name FROM _migrations WHERE name = ?")
                .bind("001_initial_schema")
                .fetch_optional(&self.pool)
                .await?;

        if applied.is_none() {
            // Run migration
            sqlx::query(include_str!("../migrations/001_initial_schema.sql"))
                .execute(&self.pool)
                .await?;

            // Record migration
            sqlx::query("INSERT INTO _migrations (name, applied_at) VALUES (?, ?)")
                .bind("001_initial_schema")
                .bind(chrono::Utc::now().timestamp_millis())
                .execute(&self.pool)
                .await?;

            info!("Applied migration: 001_initial_schema");
        } else {
            info!("Migration 001_initial_schema already applied, skipping");
        }

        info!("Database migrations completed");
        Ok(())
    }

    /// Get repository for application operations
    #[must_use]
    pub fn applications(&self) -> ApplicationRepository<'_> {
        ApplicationRepository::new(self)
    }
}
