use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::DatabaseConnection;
use crate::{
    errors::{AppError, AppJson},
    infra::cache::Cache,
    core::query_parser::{FilterParams, QueryValidator, PaginatedResponse},
    modules::user::schemas::{CreateUserRequest, UpdateUserRequest, UserResponse},
    modules::user::service::UserModuleService,
};

/// HTTP GET: Retrieve paginated list of users with dynamic filters and role name searches
pub async fn list_users_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Query(params): Query<FilterParams>,
) -> Result<Json<PaginatedResponse<UserResponse>>, AppError> {
    // Strict schema search check: only allow searching on name, email, and Role.name columns
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["name", "email", "Role.name"],
    )?;

    let users = UserModuleService::list_users(parsed_filters, &db).await?;
    Ok(Json(users))
}

/// HTTP GET: Retrieve detailed profile of a single user
pub async fn get_user_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, AppError> {
    let user = UserModuleService::get_user_by_id(&id, &db).await?;
    Ok(Json(user))
}

/// HTTP POST: Register a new user and credential pair
pub async fn create_user_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    AppJson(payload): AppJson<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let created = UserModuleService::create_user(payload, &db).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// HTTP PUT: Modify profile details of a user, invalidating existing caches
pub async fn update_user_handler(
    State((db, cache, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let updated = UserModuleService::update_user(&id, payload, &db, &cache).await?;
    Ok(Json(updated))
}

/// HTTP DELETE: Perform soft-delete and LGPD scrubbing on user record
pub async fn delete_user_handler(
    State((db, cache, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    UserModuleService::delete_user(&id, &db, &cache).await?;
    Ok(StatusCode::NO_CONTENT)
}
