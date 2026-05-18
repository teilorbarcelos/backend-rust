use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use crate::{
    errors::AppError,
    infra::cache::Cache,
    core::query_parser::{FilterParams, QueryValidator, PaginatedResponse},
    modules::audit::schemas::AuditLogResponse,
    modules::audit::service::AuditModuleService,
};

/// HTTP GET: Retrieve paginated list of logged database mutations (Audit Trail)
pub async fn list_audit_logs_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Query(params): Query<FilterParams>,
) -> Result<Json<PaginatedResponse<AuditLogResponse>>, AppError> {
    // Only allow searching by username / email
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["username"],
    )?;

    let logs = AuditModuleService::list_audit_logs(parsed_filters, &db).await?;
    Ok(Json(logs))
}
