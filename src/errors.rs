use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
    extract::FromRequest,
    extract::rejection::JsonRejection,
    extract::Request,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub status: bool,
    pub message: String,
    pub error: String,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    Internal(String),
}

impl AppError {
    pub fn message(&self) -> String {
        match self {
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Unauthorized(msg) => msg.clone(),
            AppError::Forbidden(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::Internal(msg) => msg.clone(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, error_name, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BadRequestError", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UnauthorizedError", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "ForbiddenError", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NotFoundError", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "ConflictError", msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "InternalServerError", msg),
        };

        // Write audit log error to console for easier debugging
        if status_code == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("Internal AppError: {}", message);
        }

        let body = Json(ErrorResponse {
            status: false,
            message,
            error: error_name.to_string(),
        });

        (status_code, body).into_response()
    }
}

// Convert SeaORM Database Errors
impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        AppError::Internal(format!("Erro no banco de dados: {}", err))
    }
}

// Convert Bcrypt Hashing Errors
impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        AppError::Internal(format!("Erro de criptografia: {}", err))
    }
}

// Convert JWT Session Errors
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::Unauthorized(format!("Token JWT inválido ou expirado: {}", err))
    }
}

// Custom JSON Extractor to intercept deserialization rejections (e.g. missing required fields)
// and return a premium, compliant JSON error response (Bad Request 400).
pub struct AppJson<T>(pub T);

#[axum::async_trait]
impl<S, T> FromRequest<S> for AppJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => {
                let msg = match rejection {
                    JsonRejection::MissingJsonContentType(_) => "Cabeçalho Content-Type ausente ou inválido".to_string(),
                    JsonRejection::BytesRejection(e) => format!("Falha ao ler o corpo da requisição: {}", e),
                    JsonRejection::JsonDataError(e) => format!("Erro de validação do JSON: {}", e),
                    JsonRejection::JsonSyntaxError(e) => format!("Erro de sintaxe no JSON: {}", e),
                    _ => "Falha ao desserializar o corpo da requisição".to_string(),
                };
                Err(AppError::BadRequest(msg))
            }
        }
    }
}
