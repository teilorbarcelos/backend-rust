use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sea_orm::prelude::Decimal;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: Decimal,
    pub stock: i32,
    pub description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: Decimal,
    pub stock: i32,
    pub description: String,
    pub active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProductResponse {
    pub id: String,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: Decimal,
    pub stock: i32,
    pub description: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedProductResponse {
    pub items: Vec<ProductResponse>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}
