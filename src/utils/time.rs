use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimeError {
    #[error("Time not initialized")]
    NotInitialized,
    #[error("Time calculation error: {0}")]
    Calculation(String),
}

pub static START_TIME: OnceLock<SystemTime> = OnceLock::new();

pub fn init_start_time() {
    START_TIME.get_or_init(SystemTime::now);
}

pub fn current_timestamp_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .map_err(|e| format!("Failed to get timestamp: {}", e))
}

pub fn uptime_in_secs(start_time: SystemTime) -> Result<u64, String> {
    start_time
        .elapsed()
        .map(|d| d.as_secs())
        .map_err(|e| format!("Failed to calculate uptime: {}", e))
}
