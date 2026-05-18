use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use sea_orm::DatabaseConnection;
use crate::{
    errors::{AppError, AppJson},
    infra::cache::Cache,
    middleware::auth::CurrentUser,
    core::query_parser::{FilterParams, QueryValidator, PaginatedResponse},
    modules::product::schemas::{CreateProductRequest, ProductResponse, UpdateProductRequest},
    modules::product::service::ProductModuleService,
};

/// HTTP GET: Retrieve paginated list of products
pub async fn list_products_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<FilterParams>,
) -> Result<Json<PaginatedResponse<ProductResponse>>, AppError> {
    // RBAC check
    crate::middleware::auth::authorize(&current_user.id, "product", "view", &db).await?;

    // Only allow searching on product name
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["name"],
    )?;

    let products = ProductModuleService::list_products(parsed_filters, &db).await?;
    Ok(Json(products))
}

/// HTTP GET: Retrieve detailed information of a single product
pub async fn get_product_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    // RBAC check
    crate::middleware::auth::authorize(&current_user.id, "product", "view", &db).await?;

    let product = ProductModuleService::get_product_by_id(&id, &db).await?;
    Ok(Json(product))
}

/// HTTP POST: Create a new product record
pub async fn create_product_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
    AppJson(payload): AppJson<CreateProductRequest>,
) -> Result<impl IntoResponse, AppError> {
    // RBAC check
    crate::middleware::auth::authorize(&current_user.id, "product", "create", &db).await?;

    let created = ProductModuleService::create_product(payload, &db).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// HTTP PUT: Modify properties of a product
pub async fn update_product_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<UpdateProductRequest>,
) -> Result<Json<ProductResponse>, AppError> {
    // RBAC check
    crate::middleware::auth::authorize(&current_user.id, "product", "create", &db).await?;

    let updated = ProductModuleService::update_product(&id, payload, &db).await?;
    Ok(Json(updated))
}

/// HTTP DELETE: Mark product as soft deleted
pub async fn delete_product_handler(
    State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // RBAC check
    crate::middleware::auth::authorize(&current_user.id, "product", "delete", &db).await?;

    ProductModuleService::delete_product(&id, &db).await?;
    Ok(StatusCode::NO_CONTENT)
}
