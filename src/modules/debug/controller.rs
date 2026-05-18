use axum::{
    extract::State,
    response::Response,
    response::IntoResponse,
};
use crate::{
    errors::AppError,
    infra::cache::Cache,
    config::AppConfig,
};
use sea_orm::DatabaseConnection;

pub async fn trigger_pdf_post_handler(
    State((_, _, config)): State<(DatabaseConnection, Cache, AppConfig)>,
) -> Result<impl IntoResponse, AppError> {
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

pub async fn trigger_pdf_get_handler(
    State((_, _, config)): State<(DatabaseConnection, Cache, AppConfig)>,
) -> Result<impl IntoResponse, AppError> {
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
