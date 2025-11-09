mod config;
mod controllers;
mod logging;
mod models;
mod routes;
mod services;

use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

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
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
