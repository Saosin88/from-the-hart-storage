use crate::service::models::HealthStatus;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Health check data containing service status and uptime information")]
pub struct HealthData {
    #[schemars(
        description = "Current status of the service (e.g., 'ok', 'degraded', 'unhealthy')"
    )]
    pub status: String,
    #[schemars(description = "Service uptime in seconds")]
    pub uptime: u64,
    #[schemars(description = "Current timestamp in milliseconds since UNIX epoch")]
    pub timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Successful health check response")]
pub struct HealthResponse {
    #[schemars(description = "Health check data")]
    pub data: HealthData,
}

impl From<HealthStatus> for HealthResponse {
    fn from(status: HealthStatus) -> Self {
        Self {
            data: HealthData {
                status: "ok".to_string(),
                uptime: status.uptime,
                timestamp: status.timestamp,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Error response for failed health checks")]
pub struct ErrorResponse {
    #[schemars(description = "Error code indicating the type of error")]
    pub code: String,
    #[schemars(description = "Human-readable error message")]
    pub message: String,
    #[schemars(description = "Optional additional details about the error")]
    pub details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "A link to a file or folder")]
pub struct ViewLink {
    pub viewer_id: String,
    pub resource_id: String,
    pub owner_id: String,
    pub grant_id: String,
    pub created_date: i64,
    pub folder_prefix: String,
    pub name: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub is_folder: bool,
}

impl From<crate::service::models::ViewLink> for ViewLink {
    fn from(model: crate::service::models::ViewLink) -> Self {
        Self {
            viewer_id: model.viewer_id.to_string(),
            resource_id: model.resource_id.to_string(),
            owner_id: model.owner_id.to_string(),
            grant_id: model.grant_id.to_string(),
            created_date: model.created_date,
            folder_prefix: model.folder_prefix.to_string(),
            name: model.name.to_string(),
            media_type: model.media_type.to_string(),
            size_bytes: model.size_bytes,
            is_folder: model.is_folder,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Response containing a list of files and folders")]
pub struct StorageListResponse {
    #[schemars(description = "List of files and folders")]
    pub items: Vec<ViewLink>,
    #[schemars(description = "Cursor for pagination")]
    pub next_cursor: Option<String>,
}
