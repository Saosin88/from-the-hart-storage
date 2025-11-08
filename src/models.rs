use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthData {
    pub status: String,
    pub uptime: u64,
    pub timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub data: HealthData,
}
