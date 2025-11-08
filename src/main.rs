mod config;
mod controllers;
mod models;
mod routes;
mod services;

use log::info;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment = std::env::var("APP_ENVIRONMENT").ok();

    if environment.as_deref() != Some("local") {
        dotenvy::dotenv().ok();
    }

    services::init_start_time();
    env_logger::init();
    config::init_config();

    let app = routes::configure_routes().layer(TraceLayer::new_for_http());

    let server_config = config::config().server.as_ref().expect(
        "Server configuration (APP_SERVER_HOST, APP_SERVER_PORT) is required for local execution",
    );

    let bind_address = format!("{}:{}", server_config.host, server_config.port);
    info!(
        "From The Hart Storage starting on http://{} [environment: {}]",
        bind_address,
        config::config().environment
    );

    let listener = TcpListener::bind(&bind_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
