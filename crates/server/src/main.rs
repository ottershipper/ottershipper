use anyhow::Result;
use ottershipper_server::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with INFO level by default
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration
    let config = Config::load_default()?;

    tracing::info!("OtterShipper server starting...");
    tracing::info!("Transport: {}", config.server.transport);
    tracing::info!("Database: {}", config.database.path.display());

    // Create parent directory for database if it doesn't exist
    if let Some(parent) = config.database.path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Initialize database
    let db = ottershipper_db::Database::new(&config.database.path).await?;
    db.migrate().await?;
    tracing::info!("Database initialized successfully");

    // Initialize application service
    let app_service = ottershipper_core::ApplicationService::new(db);

    // Create MCP server
    let mcp_server = ottershipper_server::McpServer::new(app_service);

    match config.server.transport.as_str() {
        "http" => {
            tracing::info!("MCP server initialized successfully");
            tracing::info!(
                "OtterShipper ready to accept MCP requests via HTTP on {}:{}",
                config.server.bind_address,
                config.server.port
            );
            tracing::info!(
                "MCP endpoints: http://localhost:{}/sse (SSE), http://localhost:{}/message (POST)",
                config.server.port,
                config.server.port
            );

            // Run HTTP server with SSE transport
            use rmcp::transport::sse_server::SseServer;
            use rmcp::ServiceExt;

            let bind_addr = format!("{}:{}", config.server.bind_address, config.server.port).parse()?;
            let mut sse_server = SseServer::serve(bind_addr).await?;

            // Process incoming SSE transports
            while let Some(transport) = sse_server.next_transport().await {
                let server = mcp_server.clone();
                tokio::spawn(async move {
                    match server.serve(transport).await {
                        Ok(service) => {
                            if let Err(e) = service.waiting().await {
                                tracing::error!("Service error: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to serve transport: {}", e);
                        }
                    }
                });
            }
        }
        "stdio" => {
            tracing::info!("MCP server initialized successfully");
            tracing::info!("OtterShipper ready to accept MCP requests via stdio (for local Claude Code)");

            // Run the MCP server (stdio transport for local Claude Code)
            use rmcp::{transport::stdio, ServiceExt};
            let service = mcp_server.serve(stdio()).await?;
            service.waiting().await?;
        }
        other => {
            anyhow::bail!("Invalid transport type: {}. Must be 'stdio' or 'http'", other);
        }
    }

    Ok(())
}
