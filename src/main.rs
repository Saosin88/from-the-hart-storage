use from_the_hart_storage::{config, logging, routes, utils::time};

use tokio::{net::TcpListener, signal, signal::unix};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    time::init_start_time();
    config::init_config();
    logging::init_logging();

    let app = routes::configure_routes().layer(TraceLayer::new_for_http());

    let server_config = config::config().server.as_ref().expect(
        "Server configuration (APP_SERVER_HOST, APP_SERVER_PORT) is required for local execution",
    );
    let bind_address = format!("{}:{}", server_config.host, server_config.port);
    let listener = TcpListener::bind(&bind_address).await?;

    info!(
        bind_address = %bind_address,
        environment = %config::config().environment,
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
