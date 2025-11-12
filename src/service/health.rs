use crate::{
    models::{HealthData, HealthResponse},
    utils::time,
};

use crate::error::AppError;

pub fn get_health_status() -> Result<HealthResponse, AppError> {
    let start_time = time::START_TIME.get().ok_or(AppError::Internal(
        "Service start time not initialized".to_string(),
    ))?;
    let uptime = time::uptime_in_secs(*start_time)
        .map_err(|e| AppError::Internal(format!("Uptime error: {e}")))?;
    let timestamp = time::current_timestamp_millis()
        .map_err(|e| AppError::Internal(format!("Timestamp error: {e}")))?;
    Ok(HealthResponse {
        data: HealthData {
            status: "ok".to_string(),
            uptime,
            timestamp,
        },
    })
}
