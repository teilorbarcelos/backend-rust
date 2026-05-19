use crate::{
    config::AppConfig,
    infra::cache::Cache,
    modules::debug::controller::{trigger_pdf_get_handler, trigger_pdf_post_handler},
};
use axum::{routing::get, Router};
use sea_orm::DatabaseConnection;

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    let public_routes = Router::new()
        .route(
            "/pdf",
            get(trigger_pdf_get_handler).post(trigger_pdf_post_handler),
        )
        .with_state(state);

    Router::new().nest("/v1/debug", public_routes)
}
