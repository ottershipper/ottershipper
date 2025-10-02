use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Application model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}
