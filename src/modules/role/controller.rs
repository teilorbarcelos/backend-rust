use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use sea_orm::DatabaseConnection;
use crate::{
    errors::{AppError, AppJson},
    infra::cache::Cache,
    middleware::auth::CurrentUser,
    core::query_parser::{QueryValidator, PaginatedResponse},
    modules::role::schemas::{CreateRoleRequest, RoleResponse, UpdateRoleRequest, FeatureResponse},
    modules::role::service::RoleModuleService,
};

/// HTTP GET: Retrieve paginated list of active roles
pub async fn list_roles_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    uri: axum::http::Uri,
    Query(mut params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<RoleResponse>>, AppError> {
    if uri.path().ends_with("/all") {
        params.insert("ignoreDefaultFilters".to_string(), "true".to_string());
    }

    // Only allow searching by name and description
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["name", "description"],
        &["name", "description", "active", "createdAt", "updatedAt"],
    )?;

    let roles = RoleModuleService::list_roles(parsed_filters, &db).await?;
    Ok(Json(roles))
}

/// HTTP GET: Retrieve a single role and its permissions
pub async fn get_role_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<Json<RoleResponse>, AppError> {
    let role = RoleModuleService::get_role_by_id(&id, &db).await?;
    Ok(Json(role))
}

/// HTTP POST: Register a new profile role with nested permissions mappings
pub async fn create_role_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    AppJson(payload): AppJson<CreateRoleRequest>,
) -> Result<impl IntoResponse, AppError> {
    let created = RoleModuleService::create_role(payload, &db).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// HTTP PUT: Modify profile properties and update permissions cascades
pub async fn update_role_handler(
    State((db, cache, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, AppError> {
    let updated = RoleModuleService::update_role(&id, payload, &db, &cache).await?;
    Ok(Json(updated))
}

/// HTTP DELETE: Mark role profile as soft deleted
pub async fn delete_role_handler(
    State((db, cache, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    RoleModuleService::delete_role(&id, &db, &cache).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ToggleStatusRequest {
    pub active: bool,
}

/// HTTP PATCH: Modify active status of a role
pub async fn toggle_role_status_handler(
    State((db, cache, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<ToggleStatusRequest>,
) -> Result<Json<RoleResponse>, AppError> {
    // RBAC check: Action is "activate"
    crate::middleware::auth::authorize(&current_user.id, "role", "activate", &db).await?;

    let updated = RoleModuleService::toggle_role_status(&id, payload.active, &db, &cache).await?;
    Ok(Json(updated))
}

/// HTTP GET: Retrieve all active system features
pub async fn list_features_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<Vec<FeatureResponse>>, AppError> {
    // RBAC check: feature is "role", action is "view"
    crate::middleware::auth::authorize(&current_user.id, "role", "view", &db).await?;

    let features = RoleModuleService::list_features(&db).await?;
    Ok(Json(features))
}
