use serde::{Deserialize, Serialize};

/// Input schema for `otter_create_app` tool
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct CreateAppInput {
    #[schemars(
        description = "Application name (alphanumeric, hyphens, underscores, max 255 chars). Must start with alphanumeric character."
    )]
    pub name: String,
}
