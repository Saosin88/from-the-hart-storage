mod logging;

use from_the_hart_storage::{config, routes, services};
use lambda_http::{run, Error};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    services::init_start_time();
    config::init_config();
    logging::init_logging();

    let app = routes::configure_routes();
    let app = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .service(app);
        
    info!(
        environment = %config::config().environment,
        "From The Hart Storage starting on Lambda"
    );
    
    run(app).await
}
