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
    async fn put_file_and_view_link(
        &self,
        file: &File,
        view_link: &ViewLink,
    ) -> Result<(), StorageError> {
        let file_item = file.to_dynamo_item();
        let view_item = view_link.to_dynamo_item();

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
