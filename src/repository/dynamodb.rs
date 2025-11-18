// use aws_sdk_dynamodb::Client;
// use std::sync::Arc;
// use tokio::sync::OnceCell;

// static DDB_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

// pub async fn get_dynamodb_client() -> Arc<Client> {
//     DDB_CLIENT
//         .get_or_init(|| async {
//             let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
//             Arc::new(Client::new(&config))
//         })
//         .await
//         .clone()
// }

// pub struct DynamoDbRepository {
//     client: Arc<Client>,
//     table_name: String,
// }

// impl DynamoDbRepository {
//     pub fn new(client: Arc<Client>, table_name: String) -> Self {
//         Self { client, table_name }
//     }

//     pub async fn new_with_global(table_name: String) -> Self {
//         Self {
//             client: get_dynamodb_client().await,
//             table_name,
//         }
//     }
// }
