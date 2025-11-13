use chrono::{DateTime, NaiveDateTime, Utc};
use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

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

pub fn parse_media_datetime(date_str: &str) -> Option<DateTime<Utc>> {
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y:%m:%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
    }

    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    None
}
