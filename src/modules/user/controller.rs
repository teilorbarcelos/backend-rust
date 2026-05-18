use crate::{
    core::query_parser::{PaginatedResponse, QueryValidator},
    errors::{AppError, AppJson},
    infra::cache::Cache,
    middleware::auth::CurrentUser,
    modules::user::schemas::{CreateUserRequest, UpdateUserRequest, UserResponse},
    modules::user::service::UserModuleService,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use sea_orm::DatabaseConnection;

/// HTTP GET: Retrieve paginated list of users with dynamic filters and role name searches
pub async fn list_users_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    uri: axum::http::Uri,
    Query(mut params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<UserResponse>>, AppError> {
    if uri.path().ends_with("/all") {
        params.insert("ignoreDefaultFilters".to_string(), "true".to_string());
    }

    // Strict schema search check: only allow searching on name, email, and Role.name columns
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["name", "email", "Role.name"],
        &[
            "name",
            "email",
            "active",
            "createdAt",
            "updatedAt",
            "Role.name",
        ],
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

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ToggleStatusRequest {
    pub active: bool,
}

/// HTTP PATCH: Modify active status of a user
pub async fn toggle_user_status_handler(
    State((db, cache, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<ToggleStatusRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // RBAC check: Action is "activate"
    crate::middleware::auth::authorize(&current_user.id, "user", "activate", &db).await?;

    let updated = UserModuleService::toggle_user_status(&id, payload.active, &db, &cache).await?;
    Ok(Json(updated))
}
