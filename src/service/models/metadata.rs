use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MediaMetadata {
    Image(ImageMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub exif: Option<HashMap<String, String>>,
    pub gps: Option<GpsCoordinates>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpsCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

impl GpsCoordinates {
    #[must_use]
    pub fn new(latitude: f64, longitude: f64, altitude: Option<f64>) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
        }
    }
}
