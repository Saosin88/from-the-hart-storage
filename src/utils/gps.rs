pub fn parse_coordinate_with_ref(coord_str: &str, reference: Option<&str>) -> Option<f64> {
    let mut value = parse_coordinate(coord_str)?;

    if let Some(ref_str) = reference {
        if ref_str.to_uppercase().starts_with('S') || ref_str.to_uppercase().starts_with('W') {
            value = -value;
        }
    }

    Some(value)
}

pub fn parse_altitude_with_ref(altitude_str: &str, reference: Option<&str>) -> Option<f64> {
    let mut altitude = parse_decimal_value(altitude_str)?;

    if let Some(ref_str) = reference {
        let ref_lower = ref_str.to_lowercase();
        if ref_lower.contains("below") || ref_str == "1" {
            altitude = -altitude;
        }
    }

    Some(altitude)
}

pub fn parse_coordinate(coord_str: &str) -> Option<f64> {
    let trimmed = coord_str.trim();

    if trimmed.contains('°') || trimmed.contains('\'') || trimmed.contains('"') {
        return parse_dms_with_symbols(trimmed);
    }

    if trimmed.contains("deg") {
        return parse_dms_format(trimmed);
    }

    if trimmed.contains('/') && trimmed.contains(' ') {
        return parse_fractional_dms(trimmed);
    }

    if let Some(result) = parse_packed_ddm(trimmed) {
        return Some(result);
    }

    if let Some(result) = parse_space_separated_dms(trimmed) {
        return Some(result);
    }

    parse_decimal_value(trimmed)
}

fn parse_dms_with_symbols(s: &str) -> Option<f64> {
    let clean = s
        .replace(['N', 'S', 'E', 'W', 'n', 's', 'e', 'w'], "")
        .trim()
        .to_string();

    let parts: Vec<&str> = clean.split('°').collect();
    if parts.len() != 2 {
        return None;
    }

    let degrees: f64 = parts[0].trim().parse().ok()?;
    let rest = parts[1].trim();

    if rest.contains('"') {
        let min_sec: Vec<&str> = rest.split('\'').collect();
        if min_sec.len() != 2 {
            return None;
        }

        let minutes: f64 = min_sec[0].trim().parse().ok()?;
        let seconds_str = min_sec[1].trim().trim_end_matches('"').trim();
        let seconds: f64 = seconds_str.parse().ok()?;

        Some(dms_to_dd(degrees, minutes, seconds))
    } else if rest.contains('\'') {
        let minutes_str = rest.trim_end_matches('\'').trim();
        let minutes: f64 = minutes_str.parse().ok()?;

        Some(ddm_to_dd(degrees, minutes))
    } else {
        Some(degrees)
    }
}

fn parse_space_separated_dms(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split_whitespace().collect();

    match parts.len() {
        2 => {
            let degrees: f64 = parts[0].parse().ok()?;
            let minutes: f64 = parts[1].parse().ok()?;
            Some(ddm_to_dd(degrees, minutes))
        }
        3 => {
            let degrees: f64 = parts[0].parse().ok()?;
            let minutes: f64 = parts[1].parse().ok()?;
            let seconds: f64 = parts[2].parse().ok()?;
            Some(dms_to_dd(degrees, minutes, seconds))
        }
        _ => None,
    }
}

fn parse_packed_ddm(s: &str) -> Option<f64> {
    if !s.contains('.') {
        return None;
    }

    if s.contains(' ') {
        return None;
    }

    let value: f64 = s.parse().ok()?;

    if value < 100.0 {
        return None;
    }

    let int_part = value.floor() as i32;

    let deg = int_part / 100;
    let min = int_part % 100;
    let decimal_part = value - value.floor();
    let (degrees, minutes) = (deg as f64, min as f64 + decimal_part);

    if minutes >= 60.0 {
        return None;
    }

    if degrees > 180.0 {
        return None;
    }

    Some(ddm_to_dd(degrees, minutes))
}

pub fn dms_to_dd(degrees: f64, minutes: f64, seconds: f64) -> f64 {
    degrees + (minutes / 60.0) + (seconds / 3600.0)
}

pub fn ddm_to_dd(degrees: f64, decimal_minutes: f64) -> f64 {
    degrees + (decimal_minutes / 60.0)
}

fn parse_dms_format(dms_str: &str) -> Option<f64> {
    let parts: Vec<&str> = dms_str
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() < 6 {
        return None;
    }

    let degrees: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[2].parse().ok()?;
    let seconds: f64 = parts[4].parse().ok()?;

    Some(dms_to_dd(degrees, minutes, seconds))
}

fn parse_fractional_dms(frac_str: &str) -> Option<f64> {
    let parts: Vec<&str> = frac_str.split_whitespace().collect();

    if parts.len() < 3 {
        return None;
    }

    let degrees = parse_fraction(parts[0])?;
    let minutes = parse_fraction(parts[1])?;
    let seconds = parse_fraction(parts[2])?;

    Some(dms_to_dd(degrees, minutes, seconds))
}

fn parse_fraction(frac_str: &str) -> Option<f64> {
    let parts: Vec<&str> = frac_str.split('/').collect();

    if parts.len() != 2 {
        return frac_str.parse().ok();
    }

    let numerator: f64 = parts[0].parse().ok()?;
    let denominator: f64 = parts[1].parse().ok()?;

    if denominator == 0.0 {
        return None;
    }

    Some(numerator / denominator)
}

fn parse_decimal_value(val_str: &str) -> Option<f64> {
    if val_str.contains('/') {
        parse_fraction(val_str)
    } else {
        val_str.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::service::models::GpsCoordinates;

    use super::*;

    #[test]
    fn test_dms_to_dd() {
        let result = dms_to_dd(43.0, 28.0, 2.814);
        assert!((result - 43.467448).abs() < 0.000001);
    }

    #[test]
    fn test_parse_dms_format() {
        let result = parse_dms_format("43 deg 28 min 2.814 sec");
        assert!(result.is_some());
        assert!((result.unwrap() - 43.467448).abs() < 0.000001);
    }

    #[test]
    fn test_parse_fractional_dms() {
        let result = parse_fractional_dms("43/1 28/1 2814/1000");
        assert!(result.is_some());
        assert!((result.unwrap() - 43.467448).abs() < 0.000001);
    }

    #[test]
    fn test_parse_fraction() {
        assert_eq!(parse_fraction("2814/1000"), Some(2.814));
        assert_eq!(parse_fraction("43/1"), Some(43.0));
        assert_eq!(parse_fraction("123"), Some(123.0));
    }

    #[test]
    fn test_parse_coordinate_with_ref() {
        let lat = parse_coordinate_with_ref("43 deg 28 min 2.814 sec", Some("N"));
        assert!(lat.is_some());
        assert!((lat.unwrap() - 43.467448).abs() < 0.000001);

        let lat_s = parse_coordinate_with_ref("33 deg 55 min 11.82 sec", Some("S"));
        assert!(lat_s.is_some());
        assert!(lat_s.unwrap() < 0.0);

        let lon = parse_coordinate_with_ref("11 deg 53 min 6.45599999 sec", Some("E"));
        assert!(lon.is_some());
        assert!((lon.unwrap() - 11.885127).abs() < 0.000001);

        let lon_w = parse_coordinate_with_ref("79 deg 58 min 56 sec", Some("W"));
        assert!(lon_w.is_some());
        assert!(lon_w.unwrap() < 0.0);
    }

    #[test]
    fn test_parse_altitude_with_ref() {
        let alt = parse_altitude_with_ref("100/1", Some("above sea level"));
        assert_eq!(alt, Some(100.0));

        let alt_0 = parse_altitude_with_ref("100", Some("0"));
        assert_eq!(alt_0, Some(100.0));

        let alt_below = parse_altitude_with_ref("50", Some("below sea level"));
        assert_eq!(alt_below, Some(-50.0));

        let alt_1 = parse_altitude_with_ref("50", Some("1"));
        assert_eq!(alt_1, Some(-50.0));

        let alt_none = parse_altitude_with_ref("100", None);
        assert_eq!(alt_none, Some(100.0));
    }

    #[test]
    fn test_gps_coordinates_construction() {
        let coords = GpsCoordinates::new(43.467448, 11.885127, 100.0);

        assert!((coords.latitude - 43.467448).abs() < 0.000001);
        assert!((coords.longitude - 11.885127).abs() < 0.000001);
        assert_eq!(coords.altitude, 100.0);
    }

    #[test]
    fn test_dms_symbols_hemisphere_last() {
        let result = parse_coordinate("40° 26' 46\" N");
        assert!(result.is_some());
        let value = result.unwrap();
        assert!((value - 40.446111).abs() < 0.000001);
    }

    #[test]
    fn test_dms_symbols_no_hemisphere() {
        let result = parse_coordinate("40° 26' 46\"");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446111).abs() < 0.000001);
    }

    #[test]
    fn test_dms_spaces_only() {
        let result = parse_coordinate("40 26 46");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446111).abs() < 0.000001);
    }

    #[test]
    fn test_dms_spaces_with_decimals() {
        let result = parse_coordinate("40 26 46.5");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446250).abs() < 0.000001);
    }

    #[test]
    fn test_ddm_symbols_hemisphere_last() {
        let result = parse_coordinate("40° 26.767' N");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446117).abs() < 0.000001);
    }

    #[test]
    fn test_ddm_symbols_no_hemisphere() {
        let result = parse_coordinate("40° 26.767'");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446117).abs() < 0.000001);
    }

    #[test]
    fn test_ddm_spaces_only() {
        let result = parse_coordinate("40 26.767");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446117).abs() < 0.000001);
    }

    #[test]
    fn test_ddm_packed_numeric_latitude() {
        let result = parse_coordinate("4026.767");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446117).abs() < 0.000001);
    }

    #[test]
    fn test_ddm_packed_numeric_longitude() {
        let result = parse_coordinate("07958.933");
        assert!(result.is_some());
        assert!((result.unwrap() - 79.982217).abs() < 0.000001);
    }

    #[test]
    fn test_ddm_to_dd() {
        let result = ddm_to_dd(40.0, 26.767);
        assert!((result - 40.446117).abs() < 0.000001);
    }

    #[test]
    fn test_decimal_degrees() {
        let result = parse_coordinate("40.446111");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.446111).abs() < 0.000001);
    }

    #[test]
    fn test_negative_decimal_degrees() {
        let result = parse_coordinate("-79.982217");
        assert!(result.is_some());
        assert!((result.unwrap() - (-79.982217)).abs() < 0.000001);
    }

    #[test]
    fn test_zero_coordinates() {
        let result = parse_coordinate("0 0 0");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 0.0);
    }

    #[test]
    fn test_max_latitude() {
        let result = parse_coordinate("90 0 0");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 90.0);
    }

    #[test]
    fn test_max_longitude() {
        let result = parse_coordinate("180 0 0");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 180.0);
    }

    #[test]
    fn test_fractional_minutes_boundary() {
        let result = parse_coordinate("40 59.999");
        assert!(result.is_some());
        assert!((result.unwrap() - 40.999983).abs() < 0.000001);
    }
}
