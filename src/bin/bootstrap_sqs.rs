use from_the_hart_storage::{
    config,
    handler::sqs::worker,
    logging,
    repository::{dynamodb::DynamoDbRepository, s3::S3Repository},
    service::metadata::MetadataService,
};

use aws_lambda_events::event::sqs::SqsEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    logging::init_logging();

    config::init_config().map_err(|e| Error::from(format!("Failed to load config: {}", e)))?;

    info!(
        environment = %config::config().environment,
        timezone = ?config::config().timezone,
        "From The Hart Storage SQS Handler starting on Lambda"
    );

    let s3_repo = Arc::new(S3Repository::new().await);
    let ddb_repo = Arc::new(DynamoDbRepository::new().await);
    let metadata_service = Arc::new(MetadataService::new());

    let state = from_the_hart_storage::state::AppState::new(
        Some(s3_repo),
        ddb_repo,
        Some(metadata_service),
        #[cfg(feature = "http")]
        None,
        #[cfg(feature = "http")]
        None,
    );

    run(service_fn(move |event: LambdaEvent<SqsEvent>| {
        let state = state.clone();
        async move {
            worker::handle_sqs_event(event.payload, &state)
                .await
                .map_err(|e| Error::from(format!("Handler error: {}", e)))
        }
    }))
    .await
}
