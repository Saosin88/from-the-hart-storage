pub mod file;
pub mod health;
pub mod metadata;
pub mod view_link;

pub use file::{File, MediaType};
pub use health::HealthStatus;
pub use metadata::{GpsCoordinates, ImageMetadata, MediaMetadata};
pub use view_link::{ResourceId, ViewLink};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_id_enum_serde_roundtrip() {
        // File variant
        let file_id = ResourceId::File("abc123".to_string());
        let json = serde_json::to_value(&file_id).unwrap();
        assert_eq!(json, serde_json::json!({"type": "File", "value": "abc123"}));
        let deserialized: ResourceId = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, file_id);

        // Folder variant
        let folder_id = ResourceId::Folder("media/photos/".to_string());
        let json = serde_json::to_value(&folder_id).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "Folder", "value": "media/photos/"})
        );
        let deserialized: ResourceId = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, folder_id);
    }
}
