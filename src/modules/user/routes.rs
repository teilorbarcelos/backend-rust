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
    modules::user::controller::{
        create_user_handler, delete_user_handler, get_user_handler, list_users_handler, update_user_handler, toggle_user_status_handler,
    },
};

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    // Secure user routes
    let secure_routes = Router::new()
        .route("/all", get(list_users_handler))
        .route("/", get(list_users_handler).post(create_user_handler))
        .route("/:id", get(get_user_handler))
        .route("/:id", put(update_user_handler))
        .route("/:id", delete(delete_user_handler))
        .route("/:id/status", patch(toggle_user_status_handler))
        .layer(from_fn_with_state((cache.clone(), config.clone()), auth_middleware))
        .with_state(state);

    Router::new().nest("/v1/user", secure_routes)
}
