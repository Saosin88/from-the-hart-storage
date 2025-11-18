use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::OnceCell;

static DDB_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

pub async fn get_dynamodb_client() -> Arc<Client> {
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
    pub fn new(client: Arc<Client>, table_name: String) -> Self {
        Self { client, table_name }
    }

    /// Put a single item into the configured table.
    /// Returns Ok(()) on success or the underlying AWS SDK error.
    pub async fn put_item(
        &self,
        item: HashMap<String, AttributeValue>,
    ) -> Result<(), aws_sdk_dynamodb::Error> {
        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    /// Convenience async constructor that uses the shared global client.
    pub async fn new_with_global(table_name: String) -> Self {
        Self {
            client: get_dynamodb_client().await,
            table_name,
        }
    }
}
