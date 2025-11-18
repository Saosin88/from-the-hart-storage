use aws_sdk_dynamodb::{types::AttributeValue, Client};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::OnceCell;

use crate::{error::StorageError, service::File};

static DDB_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

async fn get_dynamodb_client() -> Arc<Client> {
    DDB_CLIENT
        .get_or_init(|| async {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Arc::new(Client::new(&config))
        })
        .await
        .clone()
}

pub struct DynamoDbRepository {
    client: Arc<Client>,
    table_name: String,
}

impl DynamoDbRepository {
    pub async fn new(table_name: String) -> Self {
        Self {
            client: get_dynamodb_client().await,
            table_name,
        }
    }

    pub async fn put_file(&self, file: &File) -> Result<(), StorageError> {
        let mut item: HashMap<String, AttributeValue> = HashMap::new();

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
            AttributeValue::S(file.owner_id.clone()),
        );
        item.insert(
            "resource_id".to_string(),
            AttributeValue::S(file.file_id.clone()),
        );
        item.insert(
            "file_name".to_string(),
            AttributeValue::S(file.file_name.clone()),
        );
        item.insert(
            "file_path".to_string(),
            AttributeValue::S(file.file_path.clone()),
        );
        item.insert(
            "folder_prefix".to_string(),
            AttributeValue::S(file.folder_prefix.clone()),
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

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| {
                StorageError::DynamoDb(format!("Failed to put file item into DynamoDB: {}", e))
            })?;

        Ok(())
    }
}
