mod config;
mod controllers;
mod models;
mod routes;
mod services;

use actix_web::{App, HttpServer, middleware};
use log::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let environment = std::env::var("APP_ENVIRONMENT").ok();

    if environment.as_deref() != Some("local") {
        dotenvy::dotenv().ok();
    }

    services::init_start_time();
    env_logger::init();
    config::init_config();

    let app = move || {
        App::new()
            .wrap(middleware::Logger::default())
            .configure(routes::configure_routes)
    };

    let server_config = config::config().server.as_ref().expect(
        "Server configuration (APP_SERVER_HOST, APP_SERVER_PORT) is required for local execution",
    );

    let bind_address = (server_config.host.as_str(), server_config.port);
    info!(
        "From The Hart Storage starting on http://{}:{} [environment: {}]",
        server_config.host,
        server_config.port,
        config::config().environment
    );

    HttpServer::new(app).bind(bind_address)?.run().await
}
