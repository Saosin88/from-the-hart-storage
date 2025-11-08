use crate::controllers;
use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/storage").route("/health", web::get().to(controllers::health)));
}
