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
    modules::role::schemas::{CreateRoleRequest, RoleResponse, UpdateRoleRequest},
    modules::role::service::RoleModuleService,
};

/// HTTP GET: Retrieve paginated list of active roles
pub async fn list_roles_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Query(params): Query<FilterParams>,
) -> Result<Json<PaginatedResponse<RoleResponse>>, AppError> {
    // Only allow searching by name
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["name"],
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
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, AppError> {
    let updated = RoleModuleService::update_role(&id, payload, &db).await?;
    Ok(Json(updated))
}

/// HTTP DELETE: Mark role profile as soft deleted
pub async fn delete_role_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    RoleModuleService::delete_role(&id, &db).await?;
    Ok(StatusCode::NO_CONTENT)
}
