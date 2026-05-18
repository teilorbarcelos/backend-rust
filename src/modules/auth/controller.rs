use axum::{
    extract::State,
    Extension, Json,
};
use sea_orm::DatabaseConnection;
use crate::{
    errors::{AppError, AppJson},
    infra::cache::Cache,
    config::AppConfig,
    middleware::auth::CurrentUser,
    modules::auth::schemas::{AuthResponse, LoginRequest, RefreshRequest, SimpleStatusResponse, UserMeResponse},
    modules::auth::service::AuthModuleService,
};

/// HTTP POST: Authenticates a user and returns a session token
pub async fn login_handler(
    State((db, cache, config)): State<(DatabaseConnection, Cache, AppConfig)>,
    AppJson(payload): AppJson<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let auth_data = AuthModuleService::login(payload, &db, &cache, &config).await?;
    Ok(Json(auth_data))
}

/// HTTP GET: Returns details of the currently authenticated user
pub async fn get_me_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<UserMeResponse>, AppError> {
    let me_data = AuthModuleService::get_me(&current_user.id, &db).await?;
    Ok(Json(me_data))
}

/// HTTP POST: Revokes all active session tokens for the current user (Logout)
pub async fn logout_handler(
    State((_, cache, _)): State<(DatabaseConnection, Cache, AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<SimpleStatusResponse>, AppError> {
    let response = AuthModuleService::logout(&current_user.id, &cache).await?;
    Ok(Json(response))
}

/// HTTP POST: Refreshes an expired JWT session token
pub async fn refresh_handler(
    State((db, cache, config)): State<(DatabaseConnection, Cache, AppConfig)>,
    AppJson(payload): AppJson<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let auth_data = AuthModuleService::refresh(&payload.refresh_token, &db, &cache, &config).await?;
    Ok(Json(auth_data))
}
