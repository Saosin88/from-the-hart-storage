use aws_sdk_dynamodb::{
    types::{Put, TransactWriteItem},
    Client,
};
use base64::Engine;

use crate::{
    config::config,
    error::StorageError,
    service::models::{File, ViewLink},
};

use std::sync::Arc;

use super::utils::{file_to_dynamo_item, view_link_to_dynamo_item};

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
        let aws_config = super::aws_config::get_aws_config().await;
        Self {
            client: Arc::new(Client::new(aws_config)),
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

            let put_builder = Put::builder()
                .table_name(&self.table_name)
                .set_item(Some(view_item));

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
            .index_name("view-link-index")
            .key_condition_expression("GSI2PK = :pk")
            .expression_attribute_values(":pk", aws_sdk_dynamodb::types::AttributeValue::S(pk))
            .scan_index_forward(false)
            .limit(limit);

        if let Some(cursor_str) = cursor && !cursor_str.is_empty() {
            let decoded_bytes = base64::prelude::BASE64_STANDARD
                .decode(cursor_str)
                    .map_err(|e| StorageError::InvalidRequest {
                        context: "Invalid cursor format".to_string(),
                        source: e.into(),
                    })?;

            let json: serde_json::Value =
                serde_json::from_slice(&decoded_bytes).map_err(|e| {
                    StorageError::InvalidRequest {
                        context: "Invalid cursor JSON".to_string(),
                        source: e.into(),
                    }
                })?;

            let last_evaluated_key = super::utils::json_to_dynamo_key(&json).map_err(|e| {
                StorageError::InvalidRequest {
                    context: "Invalid cursor data".to_string(),
                    source: e,
                }
            })?;

            query = query.set_exclusive_start_key(Some(last_evaluated_key));
        }

        let output = query.send().await.map_err(|e| StorageError::DynamoDb {
            context: "Failed to query view links".to_string(),
            source: e.into(),
        })?;

        let items = output.items.unwrap_or_default();
        let view_links: Result<Vec<ViewLink>, _> = items
            .iter()
            .map(super::utils::dynamo_item_to_view_link)
            .collect();
        let view_links = view_links?;

        let next_cursor = if let Some(last_evaluated_key) = output.last_evaluated_key {
            if !last_evaluated_key.is_empty() {
                let json = super::utils::dynamo_key_to_json(&last_evaluated_key);
                let json_bytes =
                    serde_json::to_vec(&json).map_err(|e| StorageError::Serialization {
                        context: "Failed to serialize cursor".to_string(),
                        source: e.into(),
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

    async fn get_file(&self, user_id: &str, file_path: &str) -> Result<Option<File>, StorageError> {
        let pk = format!("USER#{}", user_id);
        let sk = format!("FILE#{}", file_path);

        let output = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", aws_sdk_dynamodb::types::AttributeValue::S(pk))
            .key("SK", aws_sdk_dynamodb::types::AttributeValue::S(sk))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDb {
                context: "Failed to get file".to_string(),
                source: e.into(),
            })?;

        if let Some(item) = output.item {
            let file = super::utils::dynamo_item_to_file(&item)?;
            Ok(Some(file))
        } else {
            Ok(None)
        }
    }

    async fn folder_exists(&self, user_id: &str, folder_path: &str) -> Result<bool, StorageError> {
        use crate::service::file::utils::{get_folder_name, get_parent_folder_path};

        let parent_path = get_parent_folder_path(folder_path);
        let folder_name = get_folder_name(folder_path);

        let gsi2_pk = format!("VIEWER#{}#FOLDER#{}", user_id, parent_path);
        let gsi2_sk_prefix = format!("TYPE#FOLDER#{}#", folder_name);

        let output = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("view-link-index")
            .key_condition_expression("GSI2PK = :pk AND begins_with(GSI2SK, :sk_prefix)")
            .expression_attribute_values(":pk", aws_sdk_dynamodb::types::AttributeValue::S(gsi2_pk))
            .expression_attribute_values(
                ":sk_prefix",
                aws_sdk_dynamodb::types::AttributeValue::S(gsi2_sk_prefix),
            )
            .limit(1)
            .send()
            .await
            .map_err(|e| StorageError::DynamoDb {
                context: "Failed to check folder existence".to_string(),
                source: e.into(),
            })?;

        Ok(output.items.is_some_and(|items| !items.is_empty()))
    }

    async fn create_folder(
        &self,
        user_id: &str,
        folder_path: &str,
    ) -> Result<ViewLink, StorageError> {
        use crate::service::file::utils::{get_folder_name, get_parent_folder_path};
        use crate::utils::time::now_as_unix_millis;

        let parent_path = get_parent_folder_path(folder_path);
        let folder_name = get_folder_name(folder_path);

        let view_link = ViewLink {
            viewer_id: user_id.into(),
            resource_id: crate::service::models::ResourceId::Folder(folder_path.to_string()),
            owner_id: user_id.into(),
            grant_id: "OWNER".into(),
            created_date: now_as_unix_millis(),
            folder_prefix: parent_path,
            name: folder_name,
            media_type: "Folder".into(),
            size_bytes: 0,
        };

        let item = super::utils::view_link_to_dynamo_item(&view_link);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDb {
                context: "Failed to create folder".to_string(),
                source: e.into(),
            })?;

        Ok(view_link)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::MockDynamoDbRepository;
    use crate::repository::DynamoDbRepositoryTrait;
    use crate::service::models::ResourceId;

    #[tokio::test]
    async fn test_folder_exists_returns_true_when_folder_found() {
        let mock_repo = MockDynamoDbRepository::new().with_folder_exists_response(Ok(true));

        let result = mock_repo.folder_exists("user123", "media/photos/").await;

        assert!(result.is_ok());
        assert!(result.unwrap());

        let calls = mock_repo.folder_exists_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "user123");
        assert_eq!(calls[0].1, "media/photos/");
    }

    #[tokio::test]
    async fn test_folder_exists_returns_false_when_folder_not_found() {
        let mock_repo = MockDynamoDbRepository::new().with_folder_exists_response(Ok(false));

        let result = mock_repo.folder_exists("user123", "media/photos/").await;

        assert!(result.is_ok());
        assert!(!result.unwrap());

        let calls = mock_repo.folder_exists_calls();
        assert_eq!(calls.len(), 1);
    }

    #[tokio::test]
    async fn test_folder_exists_handles_error() {
        let mock_repo = MockDynamoDbRepository::new().with_folder_exists_response(Err(
            StorageError::DynamoDb {
                context: "Query failed".to_string(),
                source: anyhow::anyhow!("Test error"),
            },
        ));

        let result = mock_repo.folder_exists("user123", "media/photos/").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::DynamoDb { context, .. } => {
                assert_eq!(context, "Query failed");
            }
            _ => panic!("Expected DynamoDb error"),
        }
    }

    #[tokio::test]
    async fn test_create_folder_success() {
        let expected_view_link = ViewLink {
            viewer_id: "user123".into(),
            resource_id: ResourceId::Folder("media/photos/".to_string()),
            owner_id: "user123".into(),
            grant_id: "OWNER".into(),
            created_date: 1234567890,
            folder_prefix: "media/".into(),
            name: "photos".into(),
            media_type: "Folder".into(),
            size_bytes: 0,
        };

        let mock_repo = MockDynamoDbRepository::new()
            .with_create_folder_response(Ok(expected_view_link.clone()));

        let result = mock_repo.create_folder("user123", "media/photos/").await;

        assert!(result.is_ok());
        let view_link = result.unwrap();
        assert_eq!(view_link.viewer_id.as_str(), "user123");
        assert_eq!(view_link.resource_id.as_str(), "media/photos/");
        assert!(view_link.is_folder());
        assert_eq!(view_link.folder_prefix.as_str(), "media/");
        assert_eq!(view_link.name.as_str(), "photos");
        assert_eq!(view_link.media_type.as_str(), "Folder");

        let calls = mock_repo.create_folder_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "user123");
        assert_eq!(calls[0].1, "media/photos/");
    }

    #[tokio::test]
    async fn test_create_folder_root_level() {
        let expected_view_link = ViewLink {
            viewer_id: "user123".into(),
            resource_id: ResourceId::Folder("media/".to_string()),
            owner_id: "user123".into(),
            grant_id: "OWNER".into(),
            created_date: 1234567890,
            folder_prefix: "".into(),
            name: "media".into(),
            media_type: "Folder".into(),
            size_bytes: 0,
        };

        let mock_repo = MockDynamoDbRepository::new()
            .with_create_folder_response(Ok(expected_view_link.clone()));

        let result = mock_repo.create_folder("user123", "media/").await;

        assert!(result.is_ok());
        let view_link = result.unwrap();
        assert_eq!(view_link.folder_prefix.as_str(), "");
        assert_eq!(view_link.name.as_str(), "media");
    }

    #[tokio::test]
    async fn test_create_folder_handles_error() {
        let mock_repo = MockDynamoDbRepository::new().with_create_folder_response(Err(
            StorageError::DynamoDb {
                context: "PutItem failed".to_string(),
                source: anyhow::anyhow!("Test error"),
            },
        ));

        let result = mock_repo.create_folder("user123", "media/photos/").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::DynamoDb { context, .. } => {
                assert_eq!(context, "PutItem failed");
            }
            _ => panic!("Expected DynamoDb error"),
        }
    }
}
