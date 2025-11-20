use aws_sdk_dynamodb::types::AttributeValue;
use std::collections::HashMap;

use crate::service::{models::ViewLink, File};

pub fn view_link_to_dynamo_item(view_link: &ViewLink) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    item.insert(
        "PK".to_string(),
        AttributeValue::S(format!("USER#{}", view_link.viewer_id)),
    );

    let sk = if view_link.is_folder_marker() {
        let folder_path = view_link
            .resource_id
            .strip_prefix("FOLDER#")
            .unwrap_or(&view_link.resource_id);
        format!("VIEWLINK#{}#FOLDER#{}", view_link.owner_id, folder_path)
    } else {
        format!("VIEWLINK#{}#FILE#{}", view_link.owner_id, view_link.resource_id)
    };

    item.insert("SK".to_string(), AttributeValue::S(sk));

    let item_type = if view_link.is_folder_marker() {
        "FOLDER_VIEW_LINK"
    } else {
        "FILE_VIEW_LINK"
    };

    item.insert(
        "item_type".to_string(),
        AttributeValue::S(item_type.to_string()),
    );
    item.insert(
        "resource_id".to_string(),
        AttributeValue::S(view_link.resource_id.to_string()),
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

    let gsi2_sk = if view_link.is_folder_marker() {
        format!("TYPE#FOLDER#{}#{}", view_link.name, view_link.owner_id)
    } else {
        format!(
            "TYPE#FILE#{}#{}#{}",
            view_link.created_date, view_link.media_type, view_link.resource_id
        )
    };

    item.insert("GSI2SK".to_string(), AttributeValue::S(gsi2_sk));

    item
}

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

    if let Some(metadata) = &file.media_metadata {
        if let Ok(meta_json) = serde_json::to_string(metadata) {
            item.insert("MediaMetadata".to_string(), AttributeValue::S(meta_json));
        }
    }

    item
}
