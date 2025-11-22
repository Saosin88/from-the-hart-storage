use from_the_hart_storage::{
    config,
    handler::http::routes,
    logging,
    repository::{dynamodb::DynamoDbRepository, s3::S3Repository},
    service::metadata::MetadataService,
    utils::time,
};

use lambda_http::Error;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    time::init_start_time();
    logging::init_logging();

    config::init_config().map_err(|e| Error::from(format!("Failed to load config: {}", e)))?;

    info!(
        environment = %config::config().environment,
        timezone = ?config::config().timezone,
        "From The Hart Storage HTTP Handler starting on Lambda"
    );

    let s3_repo = Arc::new(S3Repository::new().await);
    let ddb_repo = Arc::new(DynamoDbRepository::new().await);
    let metadata_service = Arc::new(MetadataService::new());

    let state = from_the_hart_storage::state::AppState::new(
        s3_repo,
        ddb_repo,
        metadata_service,
    );

    let app = routes::configure_routes(state);
    let app = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .service(app);

    lambda_http::run(app).await
}
