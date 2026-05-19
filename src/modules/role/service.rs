use crate::{
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    infra::cache::Cache,
    models::{feature, role, role_feature, user},
    modules::role::schemas::{
        CreateRoleRequest, FeatureResponse, PermissionRequest, RoleResponse, UpdateRoleRequest,
    },
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

pub struct RoleModuleService;

impl RoleModuleService {
    pub async fn list_roles(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<RoleResponse>, AppError> {
        use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};

        let mut filter_defs = vec![
            FilterDefinition::contains("name", role::Column::Name),
            FilterDefinition::contains("description", role::Column::Description),
            FilterDefinition::boolean("active", role::Column::Active),
        ];
        filter_defs.extend(FilterDefinition::date_range(
            "createdAt",
            role::Column::CreatedAt,
        ));
        filter_defs.extend(FilterDefinition::date_range(
            "updatedAt",
            role::Column::UpdatedAt,
        ));

        let search_defs = vec![
            SearchDefinition::contains("name", role::Column::Name),
            SearchDefinition::contains("description", role::Column::Description),
        ];

        let order_defs = vec![
            OrderDefinition::case_insensitive("name", role::Column::Name),
            OrderDefinition::case_insensitive("description", role::Column::Description),
            OrderDefinition::column("createdAt", role::Column::CreatedAt),
        ];

        let mut query = role::Entity::find().filter(role::Column::IsDeleted.ne(true));

        query = filters.apply_search(query, &search_defs);
        query = filters.apply_filters(query, &filter_defs);

        query = filters.apply_order(query, &order_defs, role::Column::CreatedAt);

        let (records, total) = filters.paginate(query, db).await?;

        let mut items = Vec::new();
        for r in records {
            let perms = role_feature::Entity::find()
                .filter(role_feature::Column::IdRole.eq(&r.id))
                .all(db)
                .await?
                .into_iter()
                .map(PermissionRequest::from)
                .collect();

            items.push(RoleResponse::from((r, perms)));
        }

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    pub async fn get_role_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<RoleResponse, AppError> {
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
            .map(PermissionRequest::from)
            .collect();

        Ok(RoleResponse::from((r, perms)))
    }

    pub async fn create_role(
        payload: CreateRoleRequest,
        db: &DatabaseConnection,
    ) -> Result<RoleResponse, AppError> {
        let role_id = payload
            .name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        let role_id = if role_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            format!("{}-{}", role_id, &uuid::Uuid::new_v4().to_string()[..6])
        };

        let exists = role::Entity::find_by_id(&role_id).one(db).await?;
        if exists.is_some() {
            return Err(AppError::Conflict(
                "Perfil com ID ou nome correspondente já cadastrado".to_string(),
            ));
        }

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

        Ok(RoleResponse::from((created, permissions_response)))
    }

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

        let mut active_role: role::ActiveModel = r.into();
        active_role.name = Set(payload.name);
        active_role.description = Set(payload.description);
        active_role.updated_at = Set(chrono::Utc::now().into());
        let updated = active_role.update(db).await?;

        let mut permissions_response = Vec::new();
        if let Some(perms) = payload.permissions {
            role_feature::Entity::delete_many()
                .filter(role_feature::Column::IdRole.eq(id))
                .exec(db)
                .await?;

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
            permissions_response = role_feature::Entity::find()
                .filter(role_feature::Column::IdRole.eq(id))
                .all(db)
                .await?
                .into_iter()
                .map(PermissionRequest::from)
                .collect();
        }

        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(RoleResponse::from((updated, permissions_response)))
    }

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

        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(())
    }

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

        let perms = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&updated.id))
            .all(db)
            .await?
            .into_iter()
            .map(PermissionRequest::from)
            .collect();

        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(RoleResponse::from((updated, perms)))
    }

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

    pub async fn list_features(db: &DatabaseConnection) -> Result<Vec<FeatureResponse>, AppError> {
        let features = feature::Entity::find()
            .filter(feature::Column::Active.eq(true))
            .all(db)
            .await?;

        let resp = features.into_iter().map(FeatureResponse::from).collect();

        Ok(resp)
    }
}
