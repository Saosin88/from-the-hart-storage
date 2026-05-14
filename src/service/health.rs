use super::models::HealthStatus;
use crate::error::StorageError;
use crate::utils::time;

pub fn get_health_status(start_time: std::time::Instant) -> Result<HealthStatus, StorageError> {
    let uptime = start_time.elapsed().as_secs();

    let timestamp = time::current_timestamp_millis().map_err(|e| StorageError::Time {
        context: format!("Timestamp calculation failed: {e}"),
    })?;

    Ok(HealthStatus { uptime, timestamp })
}
