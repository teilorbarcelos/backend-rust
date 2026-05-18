use axum::{
    middleware::from_fn_with_state,
    routing::{get, put, delete, patch},
    Router,
};
use sea_orm::DatabaseConnection;
use crate::{
    infra::cache::Cache,
    config::AppConfig,
    middleware::auth::auth_middleware,
    modules::role::controller::{
        create_role_handler, delete_role_handler, get_role_handler, list_roles_handler, update_role_handler, toggle_role_status_handler, list_features_handler,
    },
};

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    // Secure role routes
    let secure_routes = Router::new()
        .route("/features", get(list_features_handler))
        .route("/all", get(list_roles_handler))
        .route("/", get(list_roles_handler).post(create_role_handler))
        .route("/:id", get(get_role_handler))
        .route("/:id", put(update_role_handler))
        .route("/:id", delete(delete_role_handler))
        .route("/:id/status", patch(toggle_role_status_handler))
        .layer(from_fn_with_state((cache.clone(), config.clone()), auth_middleware))
        .with_state(state);

    Router::new().nest("/v1/role", secure_routes)
}
