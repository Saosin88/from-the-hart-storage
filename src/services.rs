use crate::models::{HealthData, HealthResponse};
use std::time::{SystemTime, UNIX_EPOCH};

static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

pub fn init_start_time() {
    START_TIME.get_or_init(SystemTime::now);
}

pub fn get_health_status() -> HealthResponse {
    let now = SystemTime::now();
    let start_time = START_TIME.get().unwrap_or(&now);
    let uptime = start_time.elapsed().unwrap_or_default().as_secs();

    let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_millis();

    HealthResponse {
        data: HealthData {
            status: "ok".to_string(),
            uptime,
            timestamp,
        },
    }
}
