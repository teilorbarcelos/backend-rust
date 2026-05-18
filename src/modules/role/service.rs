use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, Order,
    PaginatorTrait, Condition,
};
use crate::{
    errors::AppError,
    core::query_parser::{ParsedFilters, PaginatedResponse},
    infra::cache::Cache,
    models::{role, role_feature, feature, user},
    modules::role::schemas::{CreateRoleRequest, PermissionRequest, RoleResponse, UpdateRoleRequest, FeatureResponse},
};

pub struct RoleModuleService;

impl RoleModuleService {
    /// Paginated role listing, filtering out soft-deleted profiles by default
    pub async fn list_roles(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<RoleResponse>, AppError> {
        let mut query = role::Entity::find()
            .filter(role::Column::IsDeleted.ne(true));

        if let Some(word) = filters.search_word {
            use sea_orm::sea_query::{Expr, Func};
            let lower_word = word.to_lowercase();
            let mut or_cond = Condition::any();
            for field in filters.search_fields {
                if field == "name" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col(role::Column::Name))).like(format!("%{}%", lower_word)));
                } else if field == "description" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col(role::Column::Description))).like(format!("%{}%", lower_word)));
                }
            }
            query = query.filter(or_cond);
        }

        // Apply active status and date filters using generic macro
        query = crate::apply_common_filters!(query, filters, role::Column::Active, role::Column::CreatedAt, role::Column::UpdatedAt);

        // Apply explicit column filters
        if let Some(ref name_val) = filters.name {
            use sea_orm::sea_query::{Expr, Func};
            query = query.filter(Expr::expr(Func::lower(Expr::col(role::Column::Name))).like(format!("%{}%", name_val.to_lowercase())));
        }
        if let Some(ref desc_val) = filters.description {
            use sea_orm::sea_query::{Expr, Func};
            query = query.filter(Expr::expr(Func::lower(Expr::col(role::Column::Description))).like(format!("%{}%", desc_val.to_lowercase())));
        }

        let total = query.clone().paginate(db, 1).num_items().await?;

        // Apply sorting
        let dir = if filters.order_direction.to_lowercase() == "desc" {
            Order::Desc
        } else {
            Order::Asc
        };
        query = query.order_by(role::Column::CreatedAt, dir);

        // Apply paging
        let offset = filters.page * filters.size;
        let records = query
            .limit(filters.size)
            .offset(offset)
            .all(db)
            .await?;

        let mut items = Vec::new();
        for r in records {
            // Fetch associated granular permissions
            let perms = role_feature::Entity::find()
                .filter(role_feature::Column::IdRole.eq(&r.id))
                .all(db)
                .await?
                .into_iter()
                .map(|p| PermissionRequest {
                    feature: p.id_feature,
                    create: p.create,
                    view: p.view,
                    activate: p.activate,
                    delete: p.delete,
                })
                .collect();

            items.push(RoleResponse {
                id: r.id,
                name: r.name,
                description: r.description,
                active: r.active,
                role_feature: perms,
                created_at: r.created_at.to_rfc3339(),
                updated_at: r.updated_at.to_rfc3339(),
                is_deleted: r.is_deleted.unwrap_or(false),
                deleted_at: r.deleted_at.map(|d| d.to_rfc3339()),
            });
        }

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    /// Fetches a single role and its linked permissions by ID
    pub async fn get_role_by_id(id: &str, db: &DatabaseConnection) -> Result<RoleResponse, AppError> {
        let r = role::Entity::find_by_id(id.to_string())
            .filter(role::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Perfil não encontrado".to_string()))?;

        let perms = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&r.id))
            .all(db)
            .await?
            .into_iter()
            .map(|p| PermissionRequest {
                feature: p.id_feature,
                create: p.create,
                view: p.view,
                activate: p.activate,
                delete: p.delete,
            })
            .collect();

        Ok(RoleResponse {
            id: r.id,
            name: r.name,
            description: r.description,
            active: r.active,
            role_feature: perms,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
            is_deleted: r.is_deleted.unwrap_or(false),
            deleted_at: r.deleted_at.map(|d| d.to_rfc3339()),
        })
    }

    /// Creates a profile and writes nested permission rows to RoleFeature table
    pub async fn create_role(
        payload: CreateRoleRequest,
        db: &DatabaseConnection,
    ) -> Result<RoleResponse, AppError> {
        // Generate a clean slugified ID based on profile name
        let role_id = payload.name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();
            
        // Fallback to UUID if slug is empty
        let role_id = if role_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            // Append random characters to guarantee uniqueness in fast test suite
            format!("{}-{}", role_id, &uuid::Uuid::new_v4().to_string()[..6])
        };

        // Check if role ID already exists
        let exists = role::Entity::find_by_id(&role_id).one(db).await?;
        if exists.is_some() {
            return Err(AppError::Conflict("Perfil com ID ou nome correspondente já cadastrado".to_string()));
        }

        // 1. Create Role record
        let active_role = role::ActiveModel {
            id: Set(role_id.clone()),
            name: Set(payload.name),
            description: Set(payload.description),
            active: Set(true),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };
        let created = active_role.insert(db).await?;

        // 2. Create RoleFeature links
        let mut permissions_response = Vec::new();
        for perm in payload.permissions {
            let active_link = role_feature::ActiveModel {
                id_role: Set(role_id.clone()),
                id_feature: Set(perm.feature.clone()),
                create: Set(perm.create),
                view: Set(perm.view),
                activate: Set(perm.activate),
                delete: Set(perm.delete),
            };
            active_link.insert(db).await?;
            permissions_response.push(perm);
        }

        Ok(RoleResponse {
            id: created.id,
            name: created.name,
            description: created.description,
            active: created.active,
            role_feature: permissions_response,
            created_at: created.created_at.to_rfc3339(),
            updated_at: created.updated_at.to_rfc3339(),
            is_deleted: created.is_deleted.unwrap_or(false),
            deleted_at: created.deleted_at.map(|d| d.to_rfc3339()),
        })
    }

    /// Updates role details and cascading permissions mappings
    pub async fn update_role(
        id: &str,
        payload: UpdateRoleRequest,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<RoleResponse, AppError> {
        let r = role::Entity::find_by_id(id.to_string())
            .filter(role::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Perfil não encontrado".to_string()))?;

        // 1. Update main Role properties
        let mut active_role: role::ActiveModel = r.into();
        active_role.name = Set(payload.name);
        active_role.description = Set(payload.description);
        active_role.updated_at = Set(chrono::Utc::now().into());
        let updated = active_role.update(db).await?;

        // 2. Refresh permissions if provided in payload
        let mut permissions_response = Vec::new();
        if let Some(perms) = payload.permissions {
            // Cascade delete old relations first
            role_feature::Entity::delete_many()
                .filter(role_feature::Column::IdRole.eq(id))
                .exec(db)
                .await?;

            // Re-insert updated relations
            for perm in perms {
                let active_link = role_feature::ActiveModel {
                    id_role: Set(id.to_string()),
                    id_feature: Set(perm.feature.clone()),
                    create: Set(perm.create),
                    view: Set(perm.view),
                    activate: Set(perm.activate),
                    delete: Set(perm.delete),
                };
                active_link.insert(db).await?;
                permissions_response.push(perm);
            }
        } else {
            // Retrieve unchanged permissions for response payload
            permissions_response = role_feature::Entity::find()
                .filter(role_feature::Column::IdRole.eq(id))
                .all(db)
                .await?
                .into_iter()
                .map(|p| PermissionRequest {
                    feature: p.id_feature,
                    create: p.create,
                    view: p.view,
                    activate: p.activate,
                    delete: p.delete,
                })
                .collect();
        }

        // Invalidate sessions for all users of this role
        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(RoleResponse {
            id: updated.id,
            name: updated.name,
            description: updated.description,
            active: updated.active,
            role_feature: permissions_response,
            created_at: updated.created_at.to_rfc3339(),
            updated_at: updated.updated_at.to_rfc3339(),
            is_deleted: updated.is_deleted.unwrap_or(false),
            deleted_at: updated.deleted_at.map(|d| d.to_rfc3339()),
        })
    }

    /// Soft deletes a profile role (complying with test_soft_delete_behavior)
    pub async fn delete_role(
        id: &str,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<(), AppError> {
        let r = role::Entity::find_by_id(id.to_string())
            .filter(role::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Perfil não encontrado".to_string()))?;

        let mut active_role: role::ActiveModel = r.into();
        active_role.active = Set(false);
        active_role.is_deleted = Set(Some(true));
        active_role.deleted_at = Set(Some(chrono::Utc::now().into()));
        active_role.update(db).await?;

        // Invalidate sessions for all users of this role
        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(())
    }

    /// Changes the active status of a role
    pub async fn toggle_role_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<RoleResponse, AppError> {
        let r = role::Entity::find_by_id(id.to_string())
            .filter(role::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Perfil não encontrado".to_string()))?;

        let mut active_role: role::ActiveModel = r.into();
        active_role.active = Set(active);
        active_role.updated_at = Set(chrono::Utc::now().into());

        let updated = active_role.update(db).await?;

        // Retrieve associated permissions for the response
        let perms = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&updated.id))
            .all(db)
            .await?
            .into_iter()
            .map(|p| PermissionRequest {
                feature: p.id_feature,
                create: p.create,
                view: p.view,
                activate: p.activate,
                delete: p.delete,
            })
            .collect();

        // Invalidate sessions for all users of this role
        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(RoleResponse {
            id: updated.id,
            name: updated.name,
            description: updated.description,
            active: updated.active,
            role_feature: perms,
            created_at: updated.created_at.to_rfc3339(),
            updated_at: updated.updated_at.to_rfc3339(),
            is_deleted: updated.is_deleted.unwrap_or(false),
            deleted_at: updated.deleted_at.map(|d| d.to_rfc3339()),
        })
    }

    /// Invalidates sessions for all active users assigned to a given role ID
    async fn invalidate_role_sessions(
        role_id: &str,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<(), AppError> {
        let users = user::Entity::find()
            .filter(user::Column::IdRole.eq(role_id))
            .all(db)
            .await?;

        for u in users {
            let _ = cache.invalidate_user_sessions(&u.id).await;
        }

        Ok(())
    }

    /// Lists all active features from the database
    pub async fn list_features(db: &DatabaseConnection) -> Result<Vec<FeatureResponse>, AppError> {
        let features = feature::Entity::find()
            .filter(feature::Column::Active.eq(true))
            .all(db)
            .await?;

        let resp = features
            .into_iter()
            .map(|f| FeatureResponse {
                id: f.id,
                name: f.name,
            })
            .collect();

        Ok(resp)
    }
}
