pub mod controller;
pub mod service;
pub mod routes;
pub mod schemas;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::list_audit_logs_handler,
    ),
    components(
        schemas(
            schemas::AuditLogResponse, schemas::PaginatedAuditLogResponse
        )
    )
)]
pub struct AuditApi;
