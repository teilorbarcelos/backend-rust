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
        let mut query = audit::Entity::find();

        if let Some(word) = filters.search_word {
            use sea_orm::sea_query::{Expr, Func};
            query = query.filter(Expr::expr(Func::lower(Expr::col(audit::Column::UserName))).like(format!("%{}%", word.to_lowercase())));
        }

        // Apply creation date filters using generic macro
        query = crate::apply_common_filters!(query, filters, audit::Column::CreatedAt);

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
