mod config;
mod controllers;
mod models;
mod routes;
mod services;

use actix_web::{App, HttpServer};
use log::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let environment = std::env::var("ENVIRONMENT").ok();

    if environment.as_deref() == Some("local") || environment.is_none() {
        dotenvy::dotenv().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                if environment.is_none() {
                    "No ENVIRONMENT variable specified so assuming the environment is \"local\", but can't find a \".env\" file for local configuration"
                } else {
                    "ENVIRONMENT is set to \"local\", but can't find a \".env\" file for local configuration"
                }
            )
        })?;
    }

    services::init_start_time();
    env_logger::init();
    config::init_config();

    let app = move || App::new().configure(routes::configure_routes);

    let bind_address = (
        config::config().server.host.as_str(),
        config::config().server.port,
    );
    info!(
        "From The Hart Media starting on http://{}:{} [environment: {}]",
        config::config().server.host,
        config::config().server.port,
        config::config().environment
    );

    HttpServer::new(app).bind(bind_address)?.run().await
}
