use crate::{config::AppConfig, errors::AppError, infra::cache::Cache};
use axum::{extract::State, response::IntoResponse, response::Response};
use sea_orm::DatabaseConnection;

#[utoipa::path(
    post,
    path = "/v1/debug/pdf",
    responses(
        (status = 200, description = "Mock PDF file stream response", body = Vec<u8>, content_type = "application/pdf"),
        (status = 404, description = "Not found (if in production environment)")
    ),
    tag = "Debug"
)]
pub async fn trigger_pdf_post_handler(
    State(state): State<(DatabaseConnection, Cache, AppConfig)>,
) -> Result<impl IntoResponse, AppError> {
    let (_, _, config) = state;
    if config.environment.to_lowercase() == "production" {
        return Err(AppError::NotFound("Rota não encontrada".to_string()));
    }

    let pdf_bytes = get_mock_pdf_bytes();

    let response = Response::builder()
        .header("Content-Type", "application/pdf")
        .header("Content-Disposition", "attachment; filename=\"test.pdf\"")
        .body(axum::body::Body::from(pdf_bytes))
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

#[utoipa::path(
    get,
    path = "/v1/debug/pdf",
    responses(
        (status = 200, description = "Mock PDF file inline stream response", body = Vec<u8>, content_type = "application/pdf"),
        (status = 404, description = "Not found (if in production environment)")
    ),
    tag = "Debug"
)]
pub async fn trigger_pdf_get_handler(
    State(state): State<(DatabaseConnection, Cache, AppConfig)>,
) -> Result<impl IntoResponse, AppError> {
    let (_, _, config) = state;
    if config.environment.to_lowercase() == "production" {
        return Err(AppError::NotFound("Rota não encontrada".to_string()));
    }

    let pdf_bytes = get_mock_pdf_bytes();

    let response = Response::builder()
        .header("Content-Type", "application/pdf")
        .header("Content-Disposition", "inline; filename=\"test.pdf\"")
        .body(axum::body::Body::from(pdf_bytes))
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

fn get_mock_pdf_bytes() -> Vec<u8> {
    let mock_pdf = "%PDF-1.4\n\
1 0 obj\n\
<< /Type /Catalog /Pages 2 0 R >>\n\
endobj\n\
2 0 obj\n\
<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n\
endobj\n\
3 0 obj\n\
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R >>\n\
endobj\n\
4 0 obj\n\
<< /Length 51 >>\n\
stream\n\
BT\n\
/F1 12 Tf\n\
72 712 Td\n\
(Mock PDF Content) Tj\n\
ET\n\
endstream\n\
endobj\n\
xref\n\
0 5\n\
0000000000 65535 f \n\
0000000009 00000 n \n\
0000000056 00000 n \n\
0000000111 00000 n \n\
0000000212 00000 n \n\
trailer\n\
<< /Size 5 /Root 1 0 R >>\n\
startxref\n\
311\n\
%%EOF";
    mock_pdf.as_bytes().to_vec()
}
