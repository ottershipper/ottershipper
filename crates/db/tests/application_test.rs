use ottershipper_db::{Database, DbError};
use tempfile::tempdir;

#[tokio::test]
async fn test_create_and_get_application() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary database
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Create application
    let app = db.applications().create("test-app").await?;
    assert_eq!(app.name, "test-app");
    assert!(!app.id.is_empty());

    // Get application by ID
    let fetched = db.applications().get(&app.id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, app.id);
    assert_eq!(fetched.name, "test-app");

    Ok(())
}

#[tokio::test]
async fn test_get_application_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Create application
    let app = db.applications().create("my-service").await?;

    // Get by name
    let fetched = db.applications().get_by_name("my-service").await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, app.id);
    assert_eq!(fetched.name, "my-service");

    // Non-existent app
    let not_found = db.applications().get_by_name("does-not-exist").await?;
    assert!(not_found.is_none());

    Ok(())
}

#[tokio::test]
async fn test_list_applications() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Initially empty
    let apps = db.applications().list().await?;
    assert_eq!(apps.len(), 0);

    // Create multiple applications
    db.applications().create("app-1").await?;
    db.applications().create("app-2").await?;
    db.applications().create("app-3").await?;

    // List all
    let apps = db.applications().list().await?;
    assert_eq!(apps.len(), 3);

    // Verify all apps are present (order may vary due to same timestamp)
    let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"app-1"));
    assert!(names.contains(&"app-2"));
    assert!(names.contains(&"app-3"));

    Ok(())
}

#[tokio::test]
async fn test_delete_application() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Create application
    let app = db.applications().create("to-delete").await?;

    // Verify it exists
    let exists = db.applications().get(&app.id).await?;
    assert!(exists.is_some());

    // Delete it
    let deleted = db.applications().delete(&app.id).await?;
    assert!(deleted);

    // Verify it's gone
    let not_found = db.applications().get(&app.id).await?;
    assert!(not_found.is_none());

    // Delete non-existent (should return false)
    let not_deleted = db.applications().delete("fake-id").await?;
    assert!(!not_deleted);

    Ok(())
}

#[tokio::test]
async fn test_duplicate_name_fails() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Create first application
    db.applications().create("duplicate").await?;

    // Try to create with same name (should return DuplicateName error)
    let result = db.applications().create("duplicate").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DbError::DuplicateName(_)));

    Ok(())
}

// Edge case tests

#[tokio::test]
async fn test_empty_name_validation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Empty name should fail
    let result = db.applications().create("").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DbError::InvalidName(_)));

    Ok(())
}

#[tokio::test]
async fn test_invalid_name_validation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Name with spaces
    let result = db.applications().create("my app").await;
    assert!(matches!(result.unwrap_err(), DbError::InvalidName(_)));

    // Name starting with hyphen
    let result = db.applications().create("-myapp").await;
    assert!(matches!(result.unwrap_err(), DbError::InvalidName(_)));

    // Name with special characters
    let result = db.applications().create("my@app").await;
    assert!(matches!(result.unwrap_err(), DbError::InvalidName(_)));

    // Name too long
    let result = db.applications().create(&"a".repeat(256)).await;
    assert!(matches!(result.unwrap_err(), DbError::InvalidName(_)));

    Ok(())
}

#[tokio::test]
async fn test_migration_idempotency() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;

    // Run migration multiple times
    db.migrate().await?;
    db.migrate().await?;
    db.migrate().await?;

    // Should still work
    let app = db.applications().create("test").await?;
    assert_eq!(app.name, "test");

    Ok(())
}

#[tokio::test]
async fn test_concurrent_creates() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    // Create multiple applications concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let db_clone = db.clone();
            tokio::spawn(async move { db_clone.applications().create(&format!("app-{}", i)).await })
        })
        .collect();

    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // All should succeed
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(success_count, 10);

    // Verify all 10 apps exist
    let apps = db.applications().list().await?;
    assert_eq!(apps.len(), 10);

    Ok(())
}
