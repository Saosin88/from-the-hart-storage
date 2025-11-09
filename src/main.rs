mod config;
mod controllers;
mod models;
mod routes;
mod services;

use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment = std::env::var("APP_ENVIRONMENT").ok();

    if environment.as_deref() != Some("local") {
        dotenvy::dotenv().ok();
    }

    services::init_start_time();
    
    // Initialize tracing with error layer for better error context
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with(tracing_error::ErrorLayer::default())
        .init();
    
    config::init_config();

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
