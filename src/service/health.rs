use crate::{
    models::{HealthData, HealthResponse},
    utils::time,
};

pub fn get_health_status() -> Result<HealthResponse, time::TimeError> {
    let start_time = time::START_TIME
        .get()
        .ok_or(time::TimeError::NotInitialized)?;
    let uptime = time::uptime_in_secs(*start_time).map_err(time::TimeError::Calculation)?;
    let timestamp = time::current_timestamp_millis().map_err(time::TimeError::Calculation)?;
    Ok(HealthResponse {
        data: HealthData {
            status: "ok".to_string(),
            uptime,
            timestamp,
        },
    })
}
