pub mod auth;
pub mod user;
pub mod role;
pub mod product;
pub mod audit;
pub mod observability;
pub mod debug;

use axum::Router;
use sea_orm::DatabaseConnection;
use crate::{infra::cache::Cache, config::AppConfig};

pub fn app_router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    Router::new()
        .merge(auth::router(db.clone(), cache.clone(), config.clone()))
        .merge(user::router(db.clone(), cache.clone(), config.clone()))
        .merge(role::router(db.clone(), cache.clone(), config.clone()))
        .merge(product::router(db.clone(), cache.clone(), config.clone()))
        .merge(audit::router(db.clone(), cache.clone(), config.clone()))
        .merge(debug::router(db.clone(), cache.clone(), config.clone()))
}
