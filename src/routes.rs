use crate::controllers;
use axum::{routing::get, Router};

pub fn configure_routes() -> Router {
    Router::new().nest(
        "/storage",
        Router::new().route("/health", get(controllers::health)),
    )
}
