use crate::service::models::HealthStatus;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::DataResponse;

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

pub type HealthResponse = DataResponse<HealthData>;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_to_response() {
        let status = HealthStatus {
            uptime: 42,
            timestamp: 123456,
        };

        let response: HealthResponse = HealthResponse::from(status);

        assert_eq!(response.data.status, "ok");
        assert_eq!(response.data.uptime, 42);
        assert_eq!(response.data.timestamp, 123456);
    }
}
