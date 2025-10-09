use ottershipper_core::ApplicationService;
use ottershipper_db::Database;
use ottershipper_server::McpServer;
use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::{ClientHandler, RoleClient, ServiceExt};
use tempfile::tempdir;

/// Test client handler
#[derive(Clone)]
struct TestClient;

impl ClientHandler for TestClient {}

/// Setup test environment with MCP server and client
async fn setup_mcp_test() -> Result<
    (
        Database,
        RunningService<RoleClient, TestClient>,
        tokio::task::JoinHandle<anyhow::Result<()>>,
    ),
    Box<dyn std::error::Error>,
> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;
    db.migrate().await?;

    let service = ApplicationService::new(db.clone());
    let mcp_server = McpServer::new(service);

    // Create duplex channel for server-client communication
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    // Spawn server in background
    let server_handle = tokio::spawn(async move {
        let server = mcp_server.serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    // Start client (automatically initializes)
    let client = TestClient.serve(client_transport).await?;

    Ok((db, client, server_handle))
}

/// Test end-to-end MCP tool call: create application
/// This tests the full stack: MCP protocol → service layer → database
#[tokio::test]
async fn test_mcp_create_app_e2e() -> Result<(), Box<dyn std::error::Error>> {
    let (db, client, server_handle) = setup_mcp_test().await?;

    // Call otter_create_app tool
    let result = client
        .call_tool(CallToolRequestParam {
            name: "otter_create_app".into(),
            arguments: serde_json::json!({
                "name": "test-app"
            })
            .as_object()
            .cloned(),
        })
        .await?;

    // Verify response indicates success
    assert!(!result.content.is_empty());
    let response_text = result.content[0].as_text().unwrap();
    assert!(response_text
        .text
        .contains("Successfully created application"));
    assert!(response_text.text.contains("test-app"));

    // Verify app exists in database
    let apps = db.applications().list().await?;
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "test-app");

    client.cancel().await?;
    server_handle.await??;

    Ok(())
}

/// Test end-to-end MCP tool call: list applications
/// This tests the full stack: MCP protocol → service layer → database
#[tokio::test]
async fn test_mcp_list_apps_e2e() -> Result<(), Box<dyn std::error::Error>> {
    let (_db, client, server_handle) = setup_mcp_test().await?;

    // Create test applications via MCP tool (better end-to-end testing)
    for name in ["app-one", "app-two", "app-three"] {
        client
            .call_tool(CallToolRequestParam {
                name: "otter_create_app".into(),
                arguments: serde_json::json!({ "name": name }).as_object().cloned(),
            })
            .await?;
    }

    // Call otter_list_apps tool
    let result = client
        .call_tool(CallToolRequestParam {
            name: "otter_list_apps".into(),
            arguments: None,
        })
        .await?;

    // Verify response format
    assert!(!result.content.is_empty());
    let response_text = result.content[0].as_text().unwrap();
    let response: serde_json::Value = serde_json::from_str(&response_text.text)?;

    // Verify success flag
    assert_eq!(response["success"], true);

    // Verify count
    assert_eq!(response["count"], 3);

    // Verify all apps are present
    let apps = response["applications"].as_array().unwrap();
    assert_eq!(apps.len(), 3);

    // Verify app names
    let app_names: Vec<String> = apps
        .iter()
        .map(|app| app["name"].as_str().unwrap().to_string())
        .collect();
    assert!(app_names.contains(&"app-one".to_string()));
    assert!(app_names.contains(&"app-two".to_string()));
    assert!(app_names.contains(&"app-three".to_string()));

    // Verify each app has required fields
    for app in apps {
        assert!(app["id"].is_string());
        assert!(app["name"].is_string());
        assert!(app["created_at"].is_number());
    }

    client.cancel().await?;
    server_handle.await??;

    Ok(())
}

/// Test listing applications when no apps exist
#[tokio::test]
async fn test_mcp_list_apps_empty() -> Result<(), Box<dyn std::error::Error>> {
    let (_db, client, server_handle) = setup_mcp_test().await?;

    // Call otter_list_apps tool on empty database
    let result = client
        .call_tool(CallToolRequestParam {
            name: "otter_list_apps".into(),
            arguments: None,
        })
        .await?;

    // Verify response format
    assert!(!result.content.is_empty());
    let response_text = result.content[0].as_text().unwrap();
    let response: serde_json::Value = serde_json::from_str(&response_text.text)?;

    // Verify success flag
    assert_eq!(response["success"], true);

    // Verify empty list
    assert_eq!(response["count"], 0);
    assert_eq!(response["applications"].as_array().unwrap().len(), 0);

    client.cancel().await?;
    server_handle.await??;

    Ok(())
}
