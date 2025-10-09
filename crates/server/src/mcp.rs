use super::schemas::CreateAppInput;
use ottershipper_core::ApplicationService;
use rmcp::handler::server::{router::tool::ToolRouter, tool::Parameters, ServerHandler};
use rmcp::model::{CallToolResult, Content, ErrorCode, ErrorData as McpError, Implementation, InitializeResult, ProtocolVersion, ServerCapabilities};
use rmcp::{tool, tool_handler, tool_router};
use serde_json::json;
use std::{borrow::Cow, future::Future};
use tracing::info;

/// MCP Server for OtterShipper
#[derive(Clone)]
pub struct McpServer {
    service: ApplicationService,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl McpServer {
    /// Create a new MCP server with the given application service
    pub fn new(service: ApplicationService) -> Self {
        Self {
            service,
            tool_router: Self::tool_router(),
        }
    }

    /// Create a new application
    #[tool(description = "Create a new application in OtterShipper. Returns the application ID, name, and creation timestamp.")]
    async fn otter_create_app(
        &self,
        Parameters(input): Parameters<CreateAppInput>,
    ) -> Result<CallToolResult, McpError> {
        info!("Creating application: {}", input.name);

        match self.service.create_app(input.name.clone()).await {
            Ok(app) => {
                let response = json!({
                    "success": true,
                    "application": {
                        "id": app.id,
                        "name": app.name,
                        "created_at": app.created_at
                    },
                    "message": format!("Successfully created application '{}' with ID {}", app.name, app.id)
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => Err(McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(format!("Failed to create application: {}", e)),
                data: None,
            }),
        }
    }

    /// List all applications
    #[tool(description = "List all applications in OtterShipper. Returns an array of applications with their IDs, names, and creation timestamps.")]
    async fn otter_list_apps(&self) -> Result<CallToolResult, McpError> {
        info!("Listing all applications");

        match self.service.list_apps().await {
            Ok(apps) => {
                let response = json!({
                    "success": true,
                    "applications": apps.iter().map(|app| {
                        json!({
                            "id": app.id,
                            "name": app.name,
                            "created_at": app.created_at
                        })
                    }).collect::<Vec<_>>(),
                    "count": apps.len()
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => Err(McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(format!("Failed to list applications: {}", e)),
                data: None,
            }),
        }
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "ottershipper".to_string(),
                version: "0.1.0".to_string(),
            },
            instructions: None,
        }
    }
}
