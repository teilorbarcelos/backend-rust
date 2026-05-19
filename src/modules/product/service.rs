use crate::{
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    models::product,
    modules::product::schemas::{CreateProductRequest, ProductResponse, UpdateProductRequest},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set,
};
use uuid::Uuid;

pub struct ProductModuleService;

impl ProductModuleService {
    /// Paginated list of products, filtering out soft-deleted ones by default
    pub async fn list_products(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<ProductResponse>, AppError> {
        use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};

        // 1. Define allowed filters
        let mut filter_defs = vec![
            FilterDefinition::contains("name", product::Column::Name),
            FilterDefinition::equals("sku", product::Column::Sku),
            FilterDefinition::equals("category", product::Column::Category),
            FilterDefinition::boolean("active", product::Column::Active),
        ];
        filter_defs.extend(FilterDefinition::date_range(
            "createdAt",
            product::Column::CreatedAt,
        ));
        filter_defs.extend(FilterDefinition::date_range(
            "updatedAt",
            product::Column::UpdatedAt,
        ));

        // 2. Define search fields
        let search_defs = vec![
            SearchDefinition::contains("name", product::Column::Name),
            SearchDefinition::contains("sku", product::Column::Sku),
            SearchDefinition::contains("category", product::Column::Category),
        ];

        // 3. Define allowed sorting
        let order_defs = vec![
            OrderDefinition::case_insensitive("name", product::Column::Name),
            OrderDefinition::column("sku", product::Column::Sku),
            OrderDefinition::case_insensitive("category", product::Column::Category),
            OrderDefinition::column("createdAt", product::Column::CreatedAt),
        ];

        let mut query = product::Entity::find().filter(product::Column::IsDeleted.ne(true));

        // Apply global searchWord and filter definitions dynamically
        query = filters.apply_search(query, &search_defs);
        query = filters.apply_filters(query, &filter_defs);

        // Apply sorting dynamically
        query = filters.apply_order(query, &order_defs, product::Column::CreatedAt);

        let (records, total) = filters.paginate(query, db).await?;

        let items = records.into_iter().map(ProductResponse::from).collect();

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    /// Fetches a single product by ID
    pub async fn get_product_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let p = product::Entity::find_by_id(id.to_string())
            .filter(product::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Produto não encontrado".to_string()))?;

        Ok(ProductResponse::from(p))
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
            return Err(AppError::Conflict(
                "SKU já cadastrado no sistema".to_string(),
            ));
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

        Ok(ProductResponse::from(p))
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
                return Err(AppError::Conflict(
                    "SKU já está sendo utilizado por outro produto".to_string(),
                ));
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

        Ok(ProductResponse::from(updated))
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

        Ok(ProductResponse::from(updated))
    }
}
