use crate::{error::StorageError, utils::time};

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub uptime: u64,
    pub timestamp: u128,
}

pub fn get_health_status() -> Result<HealthStatus, StorageError> {
    let start_time = time::START_TIME.get().ok_or_else(|| {
        StorageError::NotInitialized("Service start time not initialized".to_string())
    })?;

    let uptime = time::uptime_in_secs(*start_time)
        .map_err(|e| StorageError::Time(format!("Uptime calculation failed: {e}")))?;

    let timestamp = time::current_timestamp_millis()
        .map_err(|e| StorageError::Time(format!("Timestamp calculation failed: {e}")))?;

    Ok(HealthStatus { uptime, timestamp })
}
