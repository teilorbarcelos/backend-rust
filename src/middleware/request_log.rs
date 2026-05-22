use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::time::Instant;

pub async fn request_logging_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();

    if path == "/health"
        || path == "/metrics"
        || path == "/liveness"
        || path.starts_with("/v1/docs")
    {
        return next.run(req).await;
    }

    let start = Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed().as_millis();
    let status = response.status().as_u16();

    if status >= 500 {
        tracing::error!("{} {}{} → {} ({}ms)", method, path, query, status, duration);
    } else if status >= 400 {
        tracing::warn!("{} {}{} → {} ({}ms)", method, path, query, status, duration);
    } else {
        tracing::info!("{} {}{} → {} ({}ms)", method, path, query, status, duration);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn handle_ok() -> impl IntoResponse {
        StatusCode::OK
    }

    async fn handle_bad_request() -> impl IntoResponse {
        StatusCode::BAD_REQUEST
    }

    async fn handle_internal_error() -> impl IntoResponse {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    #[tokio::test]
    async fn test_request_logging_middleware() {
        let app = Router::new()
            .route("/ok", get(handle_ok))
            .route("/bad", get(handle_bad_request))
            .route("/err", get(handle_internal_error))
            .route("/health", get(handle_ok))
            .layer(axum::middleware::from_fn(request_logging_middleware));

        let req = Request::builder().uri("/ok").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let req = Request::builder().uri("/bad").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let req = Request::builder().uri("/err").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
