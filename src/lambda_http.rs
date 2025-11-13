use from_the_hart_storage::{config, handlers::http::routes, logging, utils::time};

use lambda_http::Error;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    logging::init_logging();
    time::init_start_time();
    config::init_config();

    info!(
        environment = %config::config().environment,
        "From The Hart Storage HTTP Handler starting on Lambda"
    );

    let app = routes::configure_routes();
    let app = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .service(app);

    lambda_http::run(app).await
}
