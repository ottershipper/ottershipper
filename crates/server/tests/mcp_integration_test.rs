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
    assert!(response_text.text.contains("Successfully created application"));
    assert!(response_text.text.contains("test-app"));

    // Verify app exists in database
    let apps = db.applications().list().await?;
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "test-app");

    client.cancel().await?;
    server_handle.await??;

    Ok(())
}
