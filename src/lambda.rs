use from_the_hart_storage::{config, routes, services};
use lambda_http::{run, Error};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
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

    info!(
        environment = %config::config().environment,
        "From The Hart Storage starting on Lambda"
    );

    let app = routes::configure_routes();

    let app = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .service(app);

    run(app).await
}
