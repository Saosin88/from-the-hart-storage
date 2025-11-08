use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Health check data containing service status and uptime information")]
pub struct HealthData {
    #[schemars(description = "Current status of the service")]
    pub status: String,
    #[schemars(description = "Service uptime in seconds")]
    pub uptime: u64,
    #[schemars(description = "Current timestamp in milliseconds since UNIX epoch")]
    pub timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Health check response wrapper")]
pub struct HealthResponse {
    #[schemars(description = "Health check data")]
    pub data: HealthData,
}
