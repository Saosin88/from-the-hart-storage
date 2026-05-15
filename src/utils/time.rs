use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .map_err(|e| format!("Failed to get timestamp: {}", e))
}

#[must_use] 
#[allow(clippy::cast_possible_truncation)]
// Unix epoch millis won't overflow i64 for millions of years
pub fn now_as_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before Unix epoch")
        .as_millis() as i64
}

#[must_use] 
pub fn parse_media_datetime_with_offset(date_str: &str, offset: Option<&str>, timezone: Option<Tz>) -> Option<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.timestamp_millis());
    }

    let naive_dt = parse_naive_datetime(date_str)?;

    if let Some(offset_str) = offset
        && let Some(timestamp) = parse_datetime_with_offset(date_str, offset_str)
    {
        return Some(timestamp);
    }

    let tz = timezone.unwrap_or(chrono_tz::UTC);

    match tz.from_local_datetime(&naive_dt).single() {
        Some(local_dt) => {
            let utc_dt = local_dt.with_timezone(&Utc);
            Some(utc_dt.timestamp_millis())
        }
        None => None,
    }
}

fn parse_datetime_with_offset(date_str: &str, offset: &str) -> Option<i64> {
    let naive_dt = parse_naive_datetime(date_str)?;

    let datetime_with_offset = format!("{}{}", naive_dt.format("%Y-%m-%dT%H:%M:%S"), offset);

    DateTime::parse_from_rfc3339(&datetime_with_offset)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn parse_naive_datetime(date_str: &str) -> Option<NaiveDateTime> {
    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y:%m:%d %H:%M:%S") {
        return Some(dt);
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }

    None
}


