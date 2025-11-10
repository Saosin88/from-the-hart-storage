mod config;
mod controllers;
mod logging;
mod models;
mod routes;
mod services;

use tokio::net::TcpListener;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    services::init_start_time();
    config::init_config();
    logging::init_logging();
    let app = routes::configure_routes().layer(TraceLayer::new_for_http());
    let server_config = config::config().server.as_ref().expect(
        "Server configuration (APP_SERVER_HOST, APP_SERVER_PORT) is required for local execution",
    );
    let bind_address = format!("{}:{}", server_config.host, server_config.port);
    info!(
        bind_address = %bind_address,
        environment = %config::config().environment,
        "From The Hart Storage starting"
    );
    let listener = TcpListener::bind(&bind_address).await?;

    // Configure graceful shutdown: the server will wait for inflight requests to complete
    // when a shutdown signal is received before terminating
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Listens for shutdown signals (SIGINT/Ctrl+C and SIGTERM) to trigger graceful shutdown.
/// This ensures that inflight requests can complete and resources are cleaned up properly
/// before the application terminates.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
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
