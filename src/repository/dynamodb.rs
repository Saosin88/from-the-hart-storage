use aws_sdk_dynamodb::{
    types::{AttributeValue, Put, TransactWriteItem},
    Client,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::OnceCell;

use crate::{
    config::config,
    error::StorageError,
    service::{models::ViewLink, File},
};

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
    pub async fn new() -> Self {
        let dynamo_db_config = config()
            .dynamodb
            .as_ref()
            .expect("DynamoDB configuration is required");
        Self {
            client: get_dynamodb_client().await,
            table_name: dynamo_db_config.table.clone(),
        }
    }

    pub async fn put_file_and_view_link(
        &self,
        file: &File,
        view_link: &ViewLink,
    ) -> Result<(), StorageError> {
        let mut file_item: HashMap<String, AttributeValue> = HashMap::new();
        file_item.insert(
            "PK".to_string(),
            AttributeValue::S(format!("USER#{}", file.owner_id)),
        );
        file_item.insert(
            "SK".to_string(),
            AttributeValue::S(format!("FILE#{}", file.file_path)),
        );
        file_item.insert(
            "item_type".to_string(),
            AttributeValue::S("FILE".to_string()),
        );
        file_item.insert(
            "owner_id".to_string(),
            AttributeValue::S(file.owner_id.clone()),
        );
        file_item.insert(
            "resource_id".to_string(),
            AttributeValue::S(file.file_id.clone()),
        );
        file_item.insert(
            "file_name".to_string(),
            AttributeValue::S(file.file_name.clone()),
        );
        file_item.insert(
            "file_path".to_string(),
            AttributeValue::S(file.file_path.clone()),
        );
        file_item.insert(
            "folder_prefix".to_string(),
            AttributeValue::S(file.folder_prefix.clone()),
        );
        file_item.insert(
            "media_type".to_string(),
            AttributeValue::S(file.media_type.to_string()),
        );
        file_item.insert(
            "content_type".to_string(),
            AttributeValue::S(file.content_type.to_string()),
        );
        file_item.insert(
            "size_bytes".to_string(),
            AttributeValue::N(file.size_bytes.to_string()),
        );
        file_item.insert(
            "created_date".to_string(),
            AttributeValue::N(file.created_date.to_string()),
        );
        file_item.insert(
            "bucket_key".to_string(),
            AttributeValue::S(file.bucket_key.to_string()),
        );
        file_item.insert(
            "bucket".to_string(),
            AttributeValue::S(file.bucket.to_string()),
        );
        if let Some(metadata) = &file.media_metadata {
            if let Ok(meta_json) = serde_json::to_string(metadata) {
                file_item.insert("MediaMetadata".to_string(), AttributeValue::S(meta_json));
            }
        }

        let mut view_item: HashMap<String, AttributeValue> = HashMap::new();
        view_item.insert(
            "PK".to_string(),
            AttributeValue::S(format!("USER#{}", view_link.viewer_id)),
        );
        view_item.insert(
            "SK".to_string(),
            AttributeValue::S(format!(
                "VIEWLINK#{}#{}",
                view_link.owner_id, view_link.resource_id
            )),
        );
        view_item.insert(
            "resource_id".to_string(),
            AttributeValue::S(view_link.resource_id.clone()),
        );
        view_item.insert(
            "owner_id".to_string(),
            AttributeValue::S(view_link.owner_id.clone()),
        );
        view_item.insert(
            "grant_id".to_string(),
            AttributeValue::S(view_link.grant_id.clone()),
        );
        view_item.insert(
            "created_date".to_string(),
            AttributeValue::N(view_link.created_date.to_string()),
        );
        view_item.insert(
            "folder_prefix".to_string(),
            AttributeValue::S(view_link.folder_prefix.clone()),
        );
        view_item.insert(
            "file_name".to_string(),
            AttributeValue::S(view_link.file_name.clone()),
        );
        view_item.insert(
            "media_type".to_string(),
            AttributeValue::S(view_link.media_type.clone()),
        );
        view_item.insert(
            "size_bytes".to_string(),
            AttributeValue::N(view_link.size_bytes.to_string()),
        );

        view_item.insert(
            "GSI2-PK".to_string(),
            AttributeValue::S(format!(
                "VIEWER#{}#FOLDER#{}",
                view_link.viewer_id, view_link.folder_prefix
            )),
        );
        view_item.insert(
            "GSI2-SK".to_string(),
            AttributeValue::S(format!(
                "TYPE#FILE#{}#{}#{}",
                view_link.created_date, view_link.media_type, view_link.resource_id
            )),
        );

        let put_file = Put::builder()
            .table_name(&self.table_name)
            .set_item(Some(file_item))
            .build()
            .map_err(|e| StorageError::DynamoDb(format!("Failed to build Put for file: {}", e)))?;

        let put_view = Put::builder()
            .table_name(&self.table_name)
            .set_item(Some(view_item))
            .build()
            .map_err(|e| {
                StorageError::DynamoDb(format!("Failed to build Put for view link: {}", e))
            })?;

        let file = TransactWriteItem::builder().put(put_file).build();
        let view_link = TransactWriteItem::builder().put(put_view).build();

        self.client
            .transact_write_items()
            .set_transact_items(Some(vec![file, view_link]))
            .send()
            .await
            .map_err(|e| {
                StorageError::DynamoDb(format!("Failed to execute DynamoDB transaction: {}", e))
            })?;

        Ok(())
    }
}
