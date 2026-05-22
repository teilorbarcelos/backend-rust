use crate::{
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    models::{{entity_slug}},
    modules::{{entity_slug}}::schemas::{Create{{EntityName}}Request, {{EntityName}}Response, Update{{EntityName}}Request},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

pub struct {{EntityName}}ModuleService;

impl {{EntityName}}ModuleService {
    pub async fn list_{{entity_slug}}s(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<{{EntityName}}Response>, AppError> {
        use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};

        let mut filter_defs = vec![
{{ServiceListFilterDefinitions}}
            FilterDefinition::boolean("active", {{entity_slug}}::Column::Active),
        ];
        filter_defs.extend(FilterDefinition::date_range(
            "createdAt",
            {{entity_slug}}::Column::CreatedAt,
        ));
        filter_defs.extend(FilterDefinition::date_range(
            "updatedAt",
            {{entity_slug}}::Column::UpdatedAt,
        ));

        let search_defs = vec![
{{ServiceListSearchDefinitions}}
        ];

        let order_defs = vec![
{{ServiceListOrderDefinitions}}
            OrderDefinition::column("createdAt", {{entity_slug}}::Column::CreatedAt),
        ];

        let mut query = {{entity_slug}}::Entity::find().filter({{entity_slug}}::Column::IsDeleted.ne(true));

        query = filters.apply_search(query, &search_defs);
        query = filters.apply_filters(query, &filter_defs);

        query = filters.apply_order(query, &order_defs, {{entity_slug}}::Column::CreatedAt);

        let (records, total) = filters.paginate(query, db).await?;

        let items = records.into_iter().map({{EntityName}}Response::from).collect();

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    pub async fn get_{{entity_slug}}_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let p = {{entity_slug}}::Entity::find_by_id(id.to_string())
            .filter({{entity_slug}}::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Registro não encontrado".to_string()))?;

        Ok({{EntityName}}Response::from(p))
    }

    pub async fn create_{{entity_slug}}(
        payload: Create{{EntityName}}Request,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let new_id = Uuid::new_v4().to_string();
        let active_item = {{entity_slug}}::ActiveModel {
            id: Set(new_id),
{{ServiceCreateFieldsMappings}}
            active: Set(true),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let p = active_item.insert(db).await?;

        Ok({{EntityName}}Response::from(p))
    }

    pub async fn update_{{entity_slug}}(
        id: &str,
        payload: Update{{EntityName}}Request,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let p = {{entity_slug}}::Entity::find_by_id(id.to_string())
            .filter({{entity_slug}}::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Registro não encontrado".to_string()))?;

        let mut active_item: {{entity_slug}}::ActiveModel = p.into();
{{ServiceUpdateFieldsMappings}}

        if let Some(act) = payload.active {
            active_item.active = Set(act);
        }

        active_item.updated_at = Set(chrono::Utc::now().into());

        let updated = active_item.update(db).await?;

        Ok({{EntityName}}Response::from(updated))
    }

    pub async fn delete_{{entity_slug}}(id: &str, db: &DatabaseConnection) -> Result<(), AppError> {
        let p = {{entity_slug}}::Entity::find_by_id(id.to_string())
            .filter({{entity_slug}}::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Registro não encontrado".to_string()))?;

        let mut active_item: {{entity_slug}}::ActiveModel = p.into();
        active_item.active = Set(false);
        active_item.is_deleted = Set(Some(true));
        active_item.deleted_at = Set(Some(chrono::Utc::now().into()));
        active_item.update(db).await?;

        Ok(())
    }

    pub async fn toggle_{{entity_slug}}_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let p = {{entity_slug}}::Entity::find_by_id(id.to_string())
            .filter({{entity_slug}}::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Registro não encontrado".to_string()))?;

        let mut active_item: {{entity_slug}}::ActiveModel = p.into();
        active_item.active = Set(active);
        active_item.updated_at = Set(chrono::Utc::now().into());

        let updated = active_item.update(db).await?;

        Ok({{EntityName}}Response::from(updated))
    }
}
