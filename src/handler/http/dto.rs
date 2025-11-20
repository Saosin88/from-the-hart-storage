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
