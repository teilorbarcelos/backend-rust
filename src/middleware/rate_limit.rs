use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::{
    errors::AppError,
    infra::cache::Cache,
};

/// Rate Limiting middleware using sliding-window inside Redis cache.
pub async fn rate_limit_middleware(
    State(cache): State<Cache>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract Client IP address safely, checking proxy headers first
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next()) // Take first IP if comma-separated proxy list
        .map(|v| v.trim().to_string())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Config: 1000 requests per minute per IP to accommodate fast E2E test suites
    let limit = 1000;
    let window_sec = 60;

    let (allowed, remaining, max_limit) = cache.check_rate_limit(&client_ip, limit, window_sec).await?;

    if !allowed {
        let response = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-ratelimit-limit", max_limit.to_string())
            .header("x-ratelimit-remaining", "0")
            .body(axum::body::Body::from(
                r#"{"status":false,"message":"Muitas requisições. Por favor, tente novamente mais tarde.","error":"Too Many Requests"}"#,
            ))
            .unwrap();
            
        return Ok(response);
    }

    // Call the rest of the app handlers
    let mut response = next.run(req).await;

    // Attach compliant rate-limit headers to response
    response.headers_mut().insert(
        header::HeaderName::from_static("x-ratelimit-limit"),
        header::HeaderValue::from_str(&max_limit.to_string()).unwrap(),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-ratelimit-remaining"),
        header::HeaderValue::from_str(&remaining.to_string()).unwrap(),
    );

    Ok(response)
}
