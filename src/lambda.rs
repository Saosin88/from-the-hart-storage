use from_the_hart_storage::{config, routes, services};
use lambda_http::{run, Error};
use log::info;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Error> {
    services::init_start_time();
    env_logger::init();
    config::init_config();

    info!(
        "From The Hart Storage starting on Lambda [environment: {}]",
        config::config().environment
    );

    let app = routes::configure_routes().layer(TraceLayer::new_for_http());

    run(app).await
}
