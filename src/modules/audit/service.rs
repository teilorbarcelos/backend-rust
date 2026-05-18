use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, QuerySelect, Order, QueryFilter, PaginatorTrait};
use crate::{
    errors::AppError,
    core::query_parser::{ParsedFilters, PaginatedResponse},
    models::audit,
    modules::audit::schemas::AuditLogResponse,
};

pub struct AuditModuleService;

impl AuditModuleService {
    /// Paginated search through the audit.tb_audit log records
    pub async fn list_audit_logs(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<AuditLogResponse>, AppError> {
        use crate::core::query_parser::{FilterDefinition, SearchDefinition};

        // 1. Define allowed filters (only dates)
        let filter_defs = FilterDefinition::date_range("createdAt", audit::Column::CreatedAt);

        // 2. Define search fields (username)
        let search_defs = vec![
            SearchDefinition::contains("username", audit::Column::UserName),
        ];

        let mut query = audit::Entity::find();

        // Apply dynamic search and filters
        query = filters.apply_search(query, &search_defs);
        query = filters.apply_filters(query, &filter_defs);

        let total = query.clone().paginate(db, 1).num_items().await?;

        // Sort by created_at DESC by default
        query = query.order_by(audit::Column::CreatedAt, Order::Desc);

        // Apply paging
        let offset = filters.page * filters.size;
        let records = query
            .limit(filters.size)
            .offset(offset)
            .all(db)
            .await?;

        let items = records
            .into_iter()
            .map(|a| AuditLogResponse {
                id: a.id,
                id_user: a.id_user,
                user_name: a.user_name,
                action_type: a.action_type,
                execute_type: a.execute_type,
                class: a.class,
                function: a.function,
                params: a.params,
                raw: a.raw,
                table_name: a.table_name,
                diff_value: a.diff_value,
                original_url: a.original_url,
                method: a.method,
                created_at: a.created_at.to_rfc3339(),
            })
            .collect();

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }
}
