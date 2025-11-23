use from_the_hart_storage::{
    config,
    handler::http::routes,
    logging,
    repository::{dynamodb::DynamoDbRepository, s3::S3Repository},
    utils::time,
};

#[cfg(feature = "sqs")]
use from_the_hart_storage::service::metadata::MetadataService;

use std::sync::Arc;
use tokio::{net::TcpListener, signal, signal::unix};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    time::init_start_time();
    logging::init_logging();
    config::init_config()?;

    let s3_repo = Arc::new(S3Repository::new().await);
    let ddb_repo = Arc::new(DynamoDbRepository::new().await);

    #[cfg(feature = "sqs")]
    let state = {
        let metadata_service = Arc::new(MetadataService::new());
        from_the_hart_storage::state::AppState::new(s3_repo, ddb_repo, metadata_service)
    };

    #[cfg(not(feature = "sqs"))]
    let state = from_the_hart_storage::state::AppState::new(s3_repo, ddb_repo);

    let app = routes::configure_routes(state).layer(TraceLayer::new_for_http());

    let server_config = config::config().server.as_ref().expect(
        "Server configuration (APP_SERVER_HOST, APP_SERVER_PORT) is required for local execution",
    );
    let bind_address = format!("{}:{}", server_config.host, server_config.port);
    let listener = TcpListener::bind(&bind_address).await?;

    info!(
        bind_address = %bind_address,
        environment = %config::config().environment,
        timezone = ?config::config().timezone,
        "From The Hart Storage starting"
    );

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        unix::signal(unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
        },
        _ = terminate => {
            warn!("Received SIGTERM, initiating graceful shutdown");
        },
    }
}
