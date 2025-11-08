mod config;
mod controllers;
mod models;
mod routes;
mod services;

use actix_web::App;
use lambda_web::{LambdaError, run_actix_on_lambda};
use log::info;

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    services::init_start_time();
    dotenvy::dotenv().ok();
    env_logger::init();
    config::init_config();

    info!(
        "From The Hart Media starting on Lambda [environment: {}]",
        config::config().environment
    );

    let app = move || App::new().configure(routes::configure_routes);
    run_actix_on_lambda(app).await
}
