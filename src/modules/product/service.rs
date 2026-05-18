use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, Order,
    PaginatorTrait, Condition,
};
use uuid::Uuid;
use crate::{
    errors::AppError,
    core::query_parser::{ParsedFilters, PaginatedResponse},
    models::product,
    modules::product::schemas::{CreateProductRequest, ProductResponse, UpdateProductRequest},
};

pub struct ProductModuleService;

impl ProductModuleService {
    /// Paginated list of products, filtering out soft-deleted ones by default
    pub async fn list_products(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<ProductResponse>, AppError> {
        let mut query = product::Entity::find()
            .filter(product::Column::IsDeleted.ne(true));

        if let Some(word) = filters.search_word {
            use sea_orm::sea_query::{Expr, Func};
            let lower_word = word.to_lowercase();
            let mut or_cond = Condition::any();
            for field in filters.search_fields {
                if field == "name" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col(product::Column::Name))).like(format!("%{}%", lower_word)));
                } else if field == "sku" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col(product::Column::Sku))).like(format!("%{}%", lower_word)));
                } else if field == "category" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col(product::Column::Category))).like(format!("%{}%", lower_word)));
                }
            }
            query = query.filter(or_cond);
        }

        // Apply active status and date filters using generic macro
        query = crate::apply_common_filters!(query, filters, product::Column::Active, product::Column::CreatedAt, product::Column::UpdatedAt);

        // Apply explicit column filters
        if let Some(ref name_val) = filters.name {
            use sea_orm::sea_query::{Expr, Func};
            query = query.filter(Expr::expr(Func::lower(Expr::col(product::Column::Name))).like(format!("%{}%", name_val.to_lowercase())));
        }
        if let Some(ref sku_val) = filters.sku {
            query = query.filter(product::Column::Sku.eq(sku_val.clone()));
        }
        if let Some(ref cat_val) = filters.category {
            query = query.filter(product::Column::Category.eq(cat_val.clone()));
        }

        let total = query.clone().paginate(db, 1).num_items().await?;

        // Apply sorting
        let dir = if filters.order_direction.to_lowercase() == "desc" {
            Order::Desc
        } else {
            Order::Asc
        };
        query = query.order_by(product::Column::CreatedAt, dir);

        // Apply paging
        let offset = filters.page * filters.size;
        let records = query
            .limit(filters.size)
            .offset(offset)
            .all(db)
            .await?;

        let items = records
            .into_iter()
            .map(|p| ProductResponse {
                id: p.id,
                name: p.name,
                sku: p.sku,
                category: p.category,
                price: p.price,
                stock: p.stock,
                description: p.description,
                active: p.active,
                created_at: p.created_at.to_rfc3339(),
                updated_at: p.updated_at.to_rfc3339(),
            })
            .collect();

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    /// Fetches a single product by ID
    pub async fn get_product_by_id(id: &str, db: &DatabaseConnection) -> Result<ProductResponse, AppError> {
        let p = product::Entity::find_by_id(id.to_string())
            .filter(product::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Produto não encontrado".to_string()))?;

        Ok(ProductResponse {
            id: p.id,
            name: p.name,
            sku: p.sku,
            category: p.category,
            price: p.price,
            stock: p.stock,
            description: p.description,
            active: p.active,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        })
    }

    /// Creates a product and validates SKU uniqueness
    pub async fn create_product(
        payload: CreateProductRequest,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let exists = product::Entity::find()
            .filter(product::Column::Sku.eq(&payload.sku))
            .filter(product::Column::IsDeleted.ne(true))
            .one(db)
            .await?;

        if exists.is_some() {
            return Err(AppError::Conflict("SKU já cadastrado no sistema".to_string()));
        }

        let product_id = Uuid::new_v4().to_string();
        let active_prod = product::ActiveModel {
            id: Set(product_id),
            name: Set(payload.name),
            sku: Set(payload.sku),
            category: Set(payload.category),
            price: Set(payload.price),
            stock: Set(payload.stock),
            description: Set(payload.description),
            active: Set(true),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let p = active_prod.insert(db).await?;

        Ok(ProductResponse {
            id: p.id,
            name: p.name,
            sku: p.sku,
            category: p.category,
            price: p.price,
            stock: p.stock,
            description: p.description,
            active: p.active,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        })
    }

    /// Updates product details and saves changes
    pub async fn update_product(
        id: &str,
        payload: UpdateProductRequest,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let p = product::Entity::find_by_id(id.to_string())
            .filter(product::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Produto não encontrado".to_string()))?;

        // SKU conflict check
        if payload.sku != p.sku {
            let conflict = product::Entity::find()
                .filter(product::Column::Sku.eq(&payload.sku))
                .filter(product::Column::IsDeleted.ne(true))
                .one(db)
                .await?;
            if conflict.is_some() {
                return Err(AppError::Conflict("SKU já está sendo utilizado por outro produto".to_string()));
            }
        }

        let mut active_prod: product::ActiveModel = p.into();
        active_prod.name = Set(payload.name);
        active_prod.sku = Set(payload.sku);
        active_prod.category = Set(payload.category);
        active_prod.price = Set(payload.price);
        active_prod.stock = Set(payload.stock);
        active_prod.description = Set(payload.description);
        
        if let Some(act) = payload.active {
            active_prod.active = Set(act);
        }
        
        active_prod.updated_at = Set(chrono::Utc::now().into());

        let updated = active_prod.update(db).await?;

        Ok(ProductResponse {
            id: updated.id,
            name: updated.name,
            sku: updated.sku,
            category: updated.category,
            price: updated.price,
            stock: updated.stock,
            description: updated.description,
            active: updated.active,
            created_at: updated.created_at.to_rfc3339(),
            updated_at: updated.updated_at.to_rfc3339(),
        })
    }

    /// Soft deletes a product
    pub async fn delete_product(id: &str, db: &DatabaseConnection) -> Result<(), AppError> {
        let p = product::Entity::find_by_id(id.to_string())
            .filter(product::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Produto não encontrado".to_string()))?;

        let mut active_prod: product::ActiveModel = p.into();
        active_prod.active = Set(false);
        active_prod.is_deleted = Set(Some(true));
        active_prod.deleted_at = Set(Some(chrono::Utc::now().into()));
        active_prod.update(db).await?;

        Ok(())
    }

    /// Changes the active status of a product
    pub async fn toggle_product_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let p = product::Entity::find_by_id(id.to_string())
            .filter(product::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Produto não encontrado".to_string()))?;

        let mut active_prod: product::ActiveModel = p.into();
        active_prod.active = Set(active);
        active_prod.updated_at = Set(chrono::Utc::now().into());

        let updated = active_prod.update(db).await?;

        Ok(ProductResponse {
            id: updated.id,
            name: updated.name,
            sku: updated.sku,
            category: updated.category,
            price: updated.price,
            stock: updated.stock,
            description: updated.description,
            active: updated.active,
            created_at: updated.created_at.to_rfc3339(),
            updated_at: updated.updated_at.to_rfc3339(),
        })
    }
}
