use from_the_hart_storage::{
    config,
    handler::http::routes,
    logging,
    repository::{dynamodb::DynamoDbRepository, ssm::SsmRepository},
    service::access::CloudFrontSigner,
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

    let ddb_repo = Arc::new(DynamoDbRepository::new().await);
    let ssm_repo = SsmRepository::new().await;
    let cloudfront_signer = CloudFrontSigner::from_ssm_config(&ssm_repo).await;

    let state =
        from_the_hart_storage::state::AppState::new(None, ddb_repo, None, cloudfront_signer);

    let app = routes::configure_routes(state);
    let app = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .service(app);

    lambda_http::run(app).await
}
