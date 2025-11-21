use aws_sdk_dynamodb::{
    types::{Put, TransactWriteItem},
    Client,
};
use base64::Engine;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::{
    config::config,
    error::StorageError,
    service::{models::ViewLink, File},
};

use super::utils::{file_to_dynamo_item, view_link_to_dynamo_item};

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
}

#[async_trait::async_trait]
impl crate::repository::DynamoDbRepositoryTrait for DynamoDbRepository {
    async fn put_file_and_view_links(
        &self,
        file: &File,
        view_links: &[ViewLink],
    ) -> Result<(), StorageError> {
        let file_item = file_to_dynamo_item(file);

        let put_file = Put::builder()
            .table_name(&self.table_name)
            .set_item(Some(file_item))
            .build()
            .map_err(|e| StorageError::DynamoDb {
                context: "Failed to build Put for file".to_string(),
                source: e.into(),
            })?;

        let mut transact_items = vec![TransactWriteItem::builder().put(put_file).build()];

        for view_link in view_links {
            let view_item = view_link_to_dynamo_item(view_link);


            let mut put_builder = Put::builder()
                .table_name(&self.table_name)
                .set_item(Some(view_item));

            if view_link.is_folder {
                put_builder = put_builder
                    .condition_expression("attribute_not_exists(PK) AND attribute_not_exists(SK)");
            }

            let put_view = put_builder.build().map_err(|e| StorageError::DynamoDb {
                context: "Failed to build Put for view link".to_string(),
                source: e.into(),
            })?;

            transact_items.push(TransactWriteItem::builder().put(put_view).build());
        }

        for batch in transact_items.chunks(100) {
            match self
                .client
                .transact_write_items()
                .set_transact_items(Some(batch.to_vec()))
                .send()
                .await
            {
                Ok(_) => continue,
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("ConditionalCheckFailed") {
                        tracing::debug!("Folder marker already exists in batch, continuing");
                        continue;
                    } else {
                        return Err(StorageError::DynamoDb {
                            context: format!(
                                "Failed to execute DynamoDB transaction for batch (size: {})",
                                batch.len()
                            ),
                            source: e.into(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    async fn find_view_links_by_folder(
        &self,
        user_id: &str,
        folder_path: &str,
        limit: i32,
        cursor: Option<String>,
    ) -> Result<(Vec<ViewLink>, Option<String>), StorageError> {
        let pk = format!("VIEWER#{}#FOLDER#{}", user_id, folder_path);

        let mut query = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("GSI2")
            .key_condition_expression("GSI2PK = :pk")
            .expression_attribute_values(":pk", aws_sdk_dynamodb::types::AttributeValue::S(pk))
            .scan_index_forward(false)
            .limit(limit);

        if let Some(cursor_str) = cursor {
            if !cursor_str.is_empty() {
                let decoded_bytes = base64::prelude::BASE64_STANDARD
                    .decode(cursor_str)
                    .map_err(|e| StorageError::InvalidRequest {
                        context: "Invalid cursor format".to_string(),
                        source: e.into(),
                    })?;
                
                let json: serde_json::Value = serde_json::from_slice(&decoded_bytes).map_err(|e| StorageError::InvalidRequest {
                    context: "Invalid cursor JSON".to_string(),
                    source: e.into(),
                })?;

                let last_evaluated_key = super::utils::json_to_dynamo_key(&json).map_err(|e| StorageError::InvalidRequest {
                    context: "Invalid cursor data".to_string(),
                    source: e.into(),
                })?;

                query = query.set_exclusive_start_key(Some(last_evaluated_key));
            }
        }

        let output = query.send().await.map_err(|e| StorageError::DynamoDb {
            context: "Failed to query view links".to_string(),
            source: e.into(),
        })?;

        let items = output.items.unwrap_or_default();
        let view_links: Result<Vec<ViewLink>, _> = items
            .iter()
            .map(|item| super::utils::dynamo_item_to_view_link(item))
            .collect();
        let view_links = view_links?;

        let next_cursor = if let Some(last_evaluated_key) = output.last_evaluated_key {
            if !last_evaluated_key.is_empty() {
                let json = super::utils::dynamo_key_to_json(&last_evaluated_key);
                let json_bytes = serde_json::to_vec(&json).map_err(|e| {
                    StorageError::Serialization {
                        context: "Failed to serialize cursor".to_string(),
                        source: e.into(),
                    }
                })?;
                Some(base64::prelude::BASE64_STANDARD.encode(json_bytes))
            } else {
                None
            }
        } else {
            None
        };

        Ok((view_links, next_cursor))
    }
}
