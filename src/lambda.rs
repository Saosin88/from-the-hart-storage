use actix_web::{App, middleware};
use from_the_hart_storage::{config, routes, services}; // Import from your lib
use lambda_web::{LambdaError, is_running_on_lambda, run_actix_on_lambda};
use log::info;

#[actix_web::main]
async fn main() -> Result<(), LambdaError> {
    services::init_start_time();
    env_logger::init();
    config::init_config();

    info!(
        "From The Hart Media starting on Lambda [environment: {}]",
        config::config().environment
    );

    let app = move || {
        App::new()
            .wrap(middleware::Logger::default())
            .configure(routes::configure_routes)
    };
    if is_running_on_lambda() {
        info!("Running on AWS Lambda");
        run_actix_on_lambda(app).await?;
    } else {
        info!("Running locally (not on Lambda)");
    }

    Ok(())
}
