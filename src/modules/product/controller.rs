use crate::{
    core::query_parser::{PaginatedResponse, QueryValidator},
    errors::{AppError, AppJson},
    infra::cache::Cache,
    modules::product::schemas::{CreateProductRequest, ProductResponse, UpdateProductRequest},
    modules::product::service::ProductModuleService,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::DatabaseConnection;

/// HTTP GET: Retrieve paginated list of products
#[utoipa::path(
    get,
    path = "/v1/product",
    params(
        ("page" = Option<u64>, Query, description = "Page number"),
        ("size" = Option<u64>, Query, description = "Page size"),
        ("searchWord" = Option<String>, Query, description = "Search query word"),
        ("searchFields" = Option<String>, Query, description = "Comma-separated fields to search in"),
        ("orderBy" = Option<String>, Query, description = "Field to order by"),
        ("orderDirection" = Option<String>, Query, description = "Order direction (asc/desc)"),
        ("active" = Option<bool>, Query, description = "Filter by active status"),
    ),
    responses(
        (status = 200, description = "List of products retrieved successfully", body = PaginatedProductResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Product"
)]
pub async fn list_products_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    uri: axum::http::Uri,
    Query(mut params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<ProductResponse>>, AppError> {
    let (db, _, _) = state;

    if uri.path().ends_with("/all") {
        params.insert("ignoreDefaultFilters".to_string(), "true".to_string());
    }

    // Only allow searching by name, sku, and category
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["name", "sku", "category"],
        &[
            "name",
            "sku",
            "category",
            "active",
            "createdAt",
            "updatedAt",
        ],
    )?;

    let products = ProductModuleService::list_products(parsed_filters, &db).await?;
    Ok(Json(products))
}

/// HTTP GET: Retrieve detailed information of a single product
#[utoipa::path(
    get,
    path = "/v1/product/{id}",
    params(
        ("id" = String, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product retrieved successfully", body = ProductResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Product"
)]
pub async fn get_product_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    let (db, _, _) = state;

    let product = ProductModuleService::get_product_by_id(&id, &db).await?;
    Ok(Json(product))
}

/// HTTP POST: Create a new product record
#[utoipa::path(
    post,
    path = "/v1/product",
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Product created successfully", body = ProductResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Product"
)]
pub async fn create_product_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    AppJson(payload): AppJson<CreateProductRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (db, _, _) = state;

    let created = ProductModuleService::create_product(payload, &db).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// HTTP PUT: Modify properties of a product
#[utoipa::path(
    put,
    path = "/v1/product/{id}",
    params(
        ("id" = String, Path, description = "Product ID")
    ),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Product updated successfully", body = ProductResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Product"
)]
pub async fn update_product_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<UpdateProductRequest>,
) -> Result<Json<ProductResponse>, AppError> {
    let (db, _, _) = state;

    let updated = ProductModuleService::update_product(&id, payload, &db).await?;
    Ok(Json(updated))
}

/// HTTP DELETE: Mark product as soft deleted
#[utoipa::path(
    delete,
    path = "/v1/product/{id}",
    params(
        ("id" = String, Path, description = "Product ID")
    ),
    responses(
        (status = 204, description = "Product deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Product"
)]
pub async fn delete_product_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let (db, _, _) = state;

    ProductModuleService::delete_product(&id, &db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ToggleStatusRequest {
    pub active: bool,
}

/// HTTP PATCH: Modify active status of a product
#[utoipa::path(
    patch,
    path = "/v1/product/{id}/status",
    params(
        ("id" = String, Path, description = "Product ID")
    ),
    request_body = ToggleStatusRequest,
    responses(
        (status = 200, description = "Product status toggled successfully", body = ProductResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Product"
)]
pub async fn toggle_product_status_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<ToggleStatusRequest>,
) -> Result<Json<ProductResponse>, AppError> {
    let (db, _, _) = state;

    let updated = ProductModuleService::toggle_product_status(&id, payload.active, &db).await?;
    Ok(Json(updated))
}
