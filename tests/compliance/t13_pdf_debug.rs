use crate::common::{read_body_string, TestClient, TestContext};
use axum::http::StatusCode;

pub async fn run(ctx: &TestContext) {
    println!("=== Running PDF Debug Tests ===");
    ctx.reset_rate_limiter().await;

    test_pdf_debug_endpoints(ctx).await;
}

async fn test_pdf_debug_endpoints(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    let (status_get, resp_get) = client.get("/v1/debug/pdf").await;
    assert_eq!(status_get, StatusCode::OK);

    let headers_get = resp_get.headers();
    assert_eq!(
        headers_get.get("Content-Type").unwrap().to_str().unwrap(),
        "application/pdf"
    );
    let disposition_get = headers_get
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        disposition_get.contains("inline"),
        "Expected inline disposition, got: {}",
        disposition_get
    );

    let body_get = read_body_string(resp_get).await;
    assert!(
        body_get.starts_with("%PDF-"),
        "Response is not a valid PDF file"
    );

    let (status_post, resp_post) = client
        .request("POST", "/v1/debug/pdf", axum::body::Body::empty(), None)
        .await;
    assert_eq!(status_post, StatusCode::OK);

    let headers_post = resp_post.headers();
    assert_eq!(
        headers_post.get("Content-Type").unwrap().to_str().unwrap(),
        "application/pdf"
    );
    let disposition_post = headers_post
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        disposition_post.contains("attachment"),
        "Expected attachment disposition, got: {}",
        disposition_post
    );

    let body_post = read_body_string(resp_post).await;
    assert!(
        body_post.starts_with("%PDF-"),
        "Response is not a valid PDF file"
    );

    let mut prod_config = ctx.config.clone();
    prod_config.environment = "production".to_string();
    let prod_router =
        backend_rust::modules::app_router(ctx.db.clone(), ctx.cache.clone(), prod_config);
    let mut prod_client = TestClient::new(prod_router);

    let (status_prod_get, _) = prod_client.get("/v1/debug/pdf").await;
    assert_eq!(status_prod_get, StatusCode::NOT_FOUND);

    let (status_prod_post, _) = prod_client
        .request("POST", "/v1/debug/pdf", axum::body::Body::empty(), None)
        .await;
    assert_eq!(status_prod_post, StatusCode::NOT_FOUND);
}
