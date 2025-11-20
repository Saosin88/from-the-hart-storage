use aws_sdk_dynamodb::{
    types::{Put, TransactWriteItem},
    Client,
};
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
}
