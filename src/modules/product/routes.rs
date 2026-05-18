use axum::{
    middleware::from_fn_with_state,
    routing::{get, put, delete},
    Router,
};
use sea_orm::DatabaseConnection;
use crate::{
    infra::cache::Cache,
    config::AppConfig,
    middleware::auth::auth_middleware,
    modules::product::controller::{
        create_product_handler, delete_product_handler, get_product_handler, list_products_handler, update_product_handler,
    },
};

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    // Secure product routes
    let secure_routes = Router::new()
        .route("/", get(list_products_handler).post(create_product_handler))
        .route("/all", get(list_products_handler))
        .route("/:id", get(get_product_handler))
        .route("/:id", put(update_product_handler))
        .route("/:id", delete(delete_product_handler))
        .layer(from_fn_with_state((cache.clone(), config.clone()), auth_middleware))
        .with_state(state);

    Router::new().nest("/v1/product", secure_routes)
}
