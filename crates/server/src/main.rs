use anyhow::Result;
use std::path::PathBuf;

fn get_database_path() -> PathBuf {
    // Check environment variable first
    if let Ok(path) = std::env::var("OTTERSHIPPER_DB_PATH") {
        return PathBuf::from(path);
    }

    // Development vs Production
    if cfg!(debug_assertions) {
        // Development: use local directory
        PathBuf::from("./ottershipper.db")
    } else {
        // Production: use standard Linux location
        PathBuf::from("/var/lib/ottershipper/ottershipper.db")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with INFO level by default
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let db_path = get_database_path();
    tracing::info!("Using database at: {}", db_path.display());

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Initialize database
    let db = ottershipper_db::Database::new(&db_path).await?;
    db.migrate().await?;

    tracing::info!("OtterShipper server starting...");
    tracing::info!("Database initialized successfully");

    // TODO: Start MCP server here

    Ok(())
}
