use crate::models::{HealthData, HealthResponse};
use std::time::{SystemTime, UNIX_EPOCH};

static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

#[derive(Debug)]
pub enum HealthError {
    TimeError(String),
}

impl std::fmt::Display for HealthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthError::TimeError(msg) => write!(f, "Time error: {}", msg),
        }
    }
}

impl std::error::Error for HealthError {}

pub fn init_start_time() {
    START_TIME.get_or_init(SystemTime::now);
}

pub fn get_health_status() -> Result<HealthResponse, HealthError> {
    let now = SystemTime::now();
    let start_time = START_TIME
        .get()
        .ok_or_else(|| HealthError::TimeError("Service start time not initialized".to_string()))?;
    let uptime = start_time
        .elapsed()
        .map_err(|e| HealthError::TimeError(format!("Failed to calculate uptime: {}", e)))?
        .as_secs();
    let timestamp = now
        .duration_since(UNIX_EPOCH)
        .map_err(|e| HealthError::TimeError(format!("Failed to get timestamp: {}", e)))?
        .as_millis();
    Ok(HealthResponse {
        data: HealthData {
            status: "ok".to_string(),
            uptime,
            timestamp,
        },
    })
}
