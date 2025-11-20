use super::models::HealthStatus;
use crate::{error::StorageError, utils::time};

pub fn get_health_status() -> Result<HealthStatus, StorageError> {
    let start_time = time::START_TIME.get()        .ok_or_else(|| StorageError::NotInitialized {
            context: "Service start time not initialized".to_string(),
        })?;

    let uptime = time::uptime_in_secs(*start_time)
        .map_err(|e| StorageError::Time {
            context: format!("Uptime calculation failed: {e}"),
        })?;

    let timestamp = time::current_timestamp_millis()
        .map_err(|e| StorageError::Time {
            context: format!("Timestamp calculation failed: {e}"),
        })?;

    Ok(HealthStatus { uptime, timestamp })
}
