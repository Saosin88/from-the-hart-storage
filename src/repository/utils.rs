use aws_sdk_dynamodb::types::AttributeValue;
use std::collections::HashMap;

use crate::service::models::{File, ViewLink};

#[must_use] 
pub fn view_link_to_dynamo_item(view_link: &ViewLink) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    item.insert(
        "PK".to_string(),
        AttributeValue::S(format!("USER#{}", view_link.viewer_id)),
    );

    let (sk, item_type, resource_id_str) = match &view_link.resource_id {
        crate::service::models::ResourceId::Folder(folder_path) => {
            let sk = format!("VIEWLINK#{}#FOLDER#{}", view_link.owner_id, folder_path);
            (sk, "FOLDER_VIEW_LINK", format!("FOLDER#{}", folder_path))
        }
        crate::service::models::ResourceId::File(file_id) => {
            let sk = format!("VIEWLINK#{}#FILE#{}", view_link.owner_id, file_id);
            (sk, "FILE_VIEW_LINK", file_id.clone())
        }
    };

    item.insert("SK".to_string(), AttributeValue::S(sk));

    item.insert(
        "item_type".to_string(),
        AttributeValue::S(item_type.to_string()),
    );
    item.insert(
        "resource_id".to_string(),
        AttributeValue::S(resource_id_str),
    );
    item.insert(
        "owner_id".to_string(),
        AttributeValue::S(view_link.owner_id.to_string()),
    );
    item.insert(
        "grant_id".to_string(),
        AttributeValue::S(view_link.grant_id.to_string()),
    );
    item.insert(
        "created_date".to_string(),
        AttributeValue::N(view_link.created_date.to_string()),
    );
    item.insert(
        "folder_prefix".to_string(),
        AttributeValue::S(view_link.folder_prefix.to_string()),
    );
    item.insert(
        "name".to_string(),
        AttributeValue::S(view_link.name.to_string()),
    );
    item.insert(
        "media_type".to_string(),
        AttributeValue::S(view_link.media_type.to_string()),
    );
    item.insert(
        "size_bytes".to_string(),
        AttributeValue::N(view_link.size_bytes.to_string()),
    );

    item.insert(
        "GSI2PK".to_string(),
        AttributeValue::S(format!(
            "VIEWER#{}#FOLDER#{}",
            view_link.viewer_id, view_link.folder_prefix
        )),
    );

    let gsi2_sk = match &view_link.resource_id {
        crate::service::models::ResourceId::Folder(_) => {
            format!("TYPE#FOLDER#{}#{}", view_link.name, view_link.owner_id)
        }
        crate::service::models::ResourceId::File(file_id) => {
            format!(
                "TYPE#FILE#{}#{}#{}",
                view_link.created_date, view_link.media_type, file_id
            )
        }
    };

    item.insert("GSI2SK".to_string(), AttributeValue::S(gsi2_sk));

    item
}

#[must_use] 
pub fn file_to_dynamo_item(file: &File) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    item.insert(
        "PK".to_string(),
        AttributeValue::S(format!("USER#{}", file.owner_id)),
    );
    item.insert(
        "SK".to_string(),
        AttributeValue::S(format!("FILE#{}", file.file_path)),
    );
    item.insert(
        "item_type".to_string(),
        AttributeValue::S("FILE".to_string()),
    );
    item.insert(
        "owner_id".to_string(),
        AttributeValue::S(file.owner_id.to_string()),
    );
    item.insert(
        "resource_id".to_string(),
        AttributeValue::S(file.file_id.to_string()),
    );
    item.insert(
        "file_name".to_string(),
        AttributeValue::S(file.file_name.to_string()),
    );
    item.insert(
        "file_path".to_string(),
        AttributeValue::S(file.file_path.to_string()),
    );
    item.insert(
        "folder_prefix".to_string(),
        AttributeValue::S(file.folder_prefix.to_string()),
    );
    item.insert(
        "media_type".to_string(),
        AttributeValue::S(file.media_type.to_string()),
    );
    item.insert(
        "content_type".to_string(),
        AttributeValue::S(file.content_type.to_string()),
    );
    item.insert(
        "size_bytes".to_string(),
        AttributeValue::N(file.size_bytes.to_string()),
    );
    item.insert(
        "created_date".to_string(),
        AttributeValue::N(file.created_date.to_string()),
    );
    item.insert(
        "bucket_key".to_string(),
        AttributeValue::S(file.bucket_key.to_string()),
    );
    item.insert(
        "bucket".to_string(),
        AttributeValue::S(file.bucket.to_string()),
    );

    if let Some(metadata) = &file.media_metadata
        && let Ok(meta_json) = serde_json::to_string(metadata)
    {
        item.insert("MediaMetadata".to_string(), AttributeValue::S(meta_json));
    }

    item
}

pub fn dynamo_item_to_view_link(
    item: &HashMap<String, AttributeValue>,
) -> Result<ViewLink, crate::error::StorageError> {
    let viewer_id = item
        .get("PK")
        .and_then(|v| v.as_s().ok())
        .and_then(|s| s.strip_prefix("USER#"))
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing or invalid PK".to_string(),
            source: anyhow::anyhow!("Missing PK"),
        })?
        .to_string();

    let resource_id = item
        .get("resource_id")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing resource_id".to_string(),
            source: anyhow::anyhow!("Missing resource_id"),
        })?
        .to_string();

    let owner_id = item
        .get("owner_id")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing owner_id".to_string(),
            source: anyhow::anyhow!("Missing owner_id"),
        })?
        .to_string();

    let grant_id = item
        .get("grant_id")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing grant_id".to_string(),
            source: anyhow::anyhow!("Missing grant_id"),
        })?
        .to_string();

    let created_date = item
        .get("created_date")
        .and_then(|v| v.as_n().ok())
        .and_then(|n| n.parse::<i64>().ok())
        .unwrap_or(0);

    let folder_prefix = item
        .get("folder_prefix")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&"".to_string())
        .to_string();

    let name = item
        .get("name")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&"".to_string())
        .to_string();

    let media_type = item
        .get("media_type")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&"".to_string())
        .to_string();

    let size_bytes = item
        .get("size_bytes")
        .and_then(|v| v.as_n().ok())
        .and_then(|n| n.parse::<i64>().ok())
        .unwrap_or(0);

    let item_type = item
        .get("item_type")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&"".to_string())
        .to_string();

    let resource_id = if item_type == "FOLDER_VIEW_LINK" {
        let path = resource_id
            .strip_prefix("FOLDER#")
            .unwrap_or(&resource_id)
            .to_string();
        crate::service::models::ResourceId::Folder(path)
    } else {
        crate::service::models::ResourceId::File(resource_id)
    };

    Ok(ViewLink {
        viewer_id,
        resource_id,
        owner_id,
        grant_id,
        created_date,
        folder_prefix,
        name,
        media_type,
        size_bytes,
    })
}

#[must_use] 
pub fn dynamo_key_to_json(key: &HashMap<String, AttributeValue>) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in key {
        map.insert(k.clone(), attribute_value_to_json(v));
    }
    serde_json::Value::Object(map)
}

pub fn json_to_dynamo_key(
    json: &serde_json::Value,
) -> Result<HashMap<String, AttributeValue>, anyhow::Error> {
    if let serde_json::Value::Object(map) = json {
        let mut key = HashMap::new();
        for (k, v) in map {
            key.insert(k.clone(), json_to_attribute_value(v)?);
        }
        Ok(key)
    } else {
        Err(anyhow::anyhow!("Invalid cursor JSON format"))
    }
}

fn attribute_value_to_json(av: &AttributeValue) -> serde_json::Value {
    match av {
        AttributeValue::S(s) => serde_json::Value::String(s.clone()),
        AttributeValue::N(n) => serde_json::Value::String(n.clone()), // Keep numbers as strings to avoid precision loss
        AttributeValue::Bool(b) => serde_json::Value::Bool(*b),
        AttributeValue::Null(_) => serde_json::Value::Null,
        AttributeValue::M(m) => {
            let mut map = serde_json::Map::new();
            for (k, v) in m {
                map.insert(k.clone(), attribute_value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        AttributeValue::L(l) => {
            serde_json::Value::Array(l.iter().map(attribute_value_to_json).collect())
        }
        _ => serde_json::Value::Null,
    }
}

fn json_to_attribute_value(json: &serde_json::Value) -> Result<AttributeValue, anyhow::Error> {
    match json {
        serde_json::Value::String(s) => Ok(AttributeValue::S(s.clone())),
        serde_json::Value::Bool(b) => Ok(AttributeValue::Bool(*b)),
        serde_json::Value::Null => Ok(AttributeValue::Null(true)),
        serde_json::Value::Object(m) => {
            let mut map = HashMap::new();
            for (k, v) in m {
                map.insert(k.clone(), json_to_attribute_value(v)?);
            }
            Ok(AttributeValue::M(map))
        }
        serde_json::Value::Array(l) => {
            let mut list = Vec::new();
            for v in l {
                list.push(json_to_attribute_value(v)?);
            }
            Ok(AttributeValue::L(list))
        }
        serde_json::Value::Number(n) => Ok(AttributeValue::N(n.to_string())),
    }
}

pub fn dynamo_item_to_file(
    item: &HashMap<String, AttributeValue>,
) -> Result<File, crate::error::StorageError> {
    let bucket_key = item
        .get("bucket_key")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing bucket_key".to_string(),
            source: anyhow::anyhow!("Missing bucket_key"),
        })?
        .to_string();

    let bucket = item
        .get("bucket")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing bucket".to_string(),
            source: anyhow::anyhow!("Missing bucket"),
        })?
        .to_string();

    let owner_id = item
        .get("owner_id")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing owner_id".to_string(),
            source: anyhow::anyhow!("Missing owner_id"),
        })?
        .to_string();

    let file_id = item
        .get("resource_id")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing resource_id".to_string(),
            source: anyhow::anyhow!("Missing resource_id"),
        })?
        .to_string();

    let file_name = item
        .get("file_name")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing file_name".to_string(),
            source: anyhow::anyhow!("Missing file_name"),
        })?
        .to_string();

    let file_path = item
        .get("file_path")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| crate::error::StorageError::DynamoDb {
            context: "Missing file_path".to_string(),
            source: anyhow::anyhow!("Missing file_path"),
        })?
        .to_string();

    let folder_prefix = item
        .get("folder_prefix")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&"".to_string())
        .to_string();

    let created_date = item
        .get("created_date")
        .and_then(|v| v.as_n().ok())
        .and_then(|n| n.parse::<i64>().ok())
        .unwrap_or(0);

    let size_bytes = item
        .get("size_bytes")
        .and_then(|v| v.as_n().ok())
        .and_then(|n| n.parse::<i64>().ok())
        .unwrap_or(0);

    let content_type = item
        .get("content_type")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&"application/octet-stream".to_string())
        .to_string();

    let media_type_str = item
        .get("media_type")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&"Unknown".to_string())
        .to_string();

    let media_type = match media_type_str.as_str() {
        "Image" => crate::service::models::MediaType::Image,
        "Video" => crate::service::models::MediaType::Video,
        "Audio" => crate::service::models::MediaType::Audio,
        "Document" => crate::service::models::MediaType::Document,
        _ => crate::service::models::MediaType::Unknown,
    };

    let media_metadata = if let Some(meta_av) = item.get("MediaMetadata") {
        if let Ok(meta_str) = meta_av.as_s() {
            serde_json::from_str(meta_str).ok()
        } else {
            None
        }
    } else {
        None
    };

    Ok(File {
        bucket_key,
        bucket,
        owner_id,
        file_id,
        file_name,
        file_path,
        folder_prefix,
        created_date,
        size_bytes,
        content_type,
        media_type,
        media_metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::models::{
        File, GpsCoordinates, ImageMetadata, MediaMetadata, MediaType, ResourceId, ViewLink,
    };
    use std::collections::HashMap;

    #[test]
    fn test_file_to_dynamo_and_back() {
        let mut exif = HashMap::new();
        exif.insert("Make".to_string(), "Canon".to_string());

        let file = File {
            bucket_key: "user123/photos/vacation.jpg".into(),
            bucket: "test-bucket".into(),
            owner_id: "user123".into(),
            file_id: "sha256-hash-of-bucket+key".into(),
            file_name: "vacation.jpg".into(),
            file_path: "photos/vacation.jpg".into(),
            folder_prefix: "photos/".into(),
            created_date: 1700000000000,
            size_bytes: 1048576,
            content_type: "image/jpeg".into(),
            media_type: MediaType::Image,
            media_metadata: Some(MediaMetadata::Image(ImageMetadata {
                width: 1920,
                height: 1080,
                exif: Some(exif),
                gps: Some(GpsCoordinates {
                    latitude: 37.7749,
                    longitude: -122.4194,
                    altitude: Some(30.0),
                }),
            })),
        };

        let item = file_to_dynamo_item(&file);
        let result = dynamo_item_to_file(&item).unwrap();

        assert_eq!(result, file);
    }

    #[test]
    fn test_view_link_file_to_dynamo_and_back() {
        let view_link = ViewLink {
            viewer_id: "user123".into(),
            resource_id: ResourceId::File("some-file-id".to_string()),
            owner_id: "user123".into(),
            grant_id: "OWNER".into(),
            created_date: 1700000000000,
            folder_prefix: "photos/".into(),
            name: "photo.jpg".into(),
            media_type: "Image".into(),
            size_bytes: 1048576,
        };

        let item = view_link_to_dynamo_item(&view_link);
        let result = dynamo_item_to_view_link(&item).unwrap();

        assert_eq!(result, view_link);
    }

    #[test]
    fn test_view_link_folder_to_dynamo_and_back() {
        let view_link = ViewLink {
            viewer_id: "user123".into(),
            resource_id: ResourceId::Folder("some/path/".to_string()),
            owner_id: "user123".into(),
            grant_id: "OWNER".into(),
            created_date: 1700000000000,
            folder_prefix: "some/".into(),
            name: "path".into(),
            media_type: "Folder".into(),
            size_bytes: 0,
        };

        let item = view_link_to_dynamo_item(&view_link);
        let result = dynamo_item_to_view_link(&item).unwrap();

        assert_eq!(result, view_link);
    }

    #[test]
    fn test_dynamo_key_json_roundtrip() {
        let mut key = HashMap::new();
        key.insert(
            "PK".to_string(),
            AttributeValue::S("USER#user123".to_string()),
        );
        key.insert(
            "SK".to_string(),
            AttributeValue::S("FILE#photos/vacation.jpg".to_string()),
        );
        key.insert("is_public".to_string(), AttributeValue::Bool(true));
        key.insert("version".to_string(), AttributeValue::N("42".to_string()));

        let json = dynamo_key_to_json(&key);
        let result = json_to_dynamo_key(&json).unwrap();

        // DynamoDB numbers are preserved as strings in JSON (to avoid precision loss),
        // so N round-trips as S. Compare string representations.
        assert_eq!(key.len(), result.len());
        for (k, v) in &key {
            let rv = result.get(k).unwrap();
            match (v, rv) {
                (AttributeValue::S(a), AttributeValue::S(b)) => assert_eq!(a, b),
                (AttributeValue::N(a), AttributeValue::S(b)) => assert_eq!(a, b),
                (AttributeValue::Bool(a), AttributeValue::Bool(b)) => assert_eq!(a, b),
                _ => panic!("Mismatched attribute types for key: {}", k),
            }
        }
    }
}
