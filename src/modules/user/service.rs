use crate::{
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    infra::auth::AuthService,
    infra::cache::Cache,
    models::{auth, role, user},
    modules::user::schemas::{CreateUserRequest, UpdateUserRequest, UserResponse},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

pub struct UserModuleService;

impl UserModuleService {
    pub async fn list_users(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<UserResponse>, AppError> {
        use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};

        let mut filter_defs = vec![
            FilterDefinition::contains("name", (user::Entity, user::Column::Name)),
            FilterDefinition::contains("email", (user::Entity, user::Column::Email)),
            FilterDefinition::boolean("active", (user::Entity, user::Column::Active)),
            FilterDefinition::contains("Role.name", (role::Entity, role::Column::Name)),
        ];
        filter_defs.extend(FilterDefinition::date_range(
            "createdAt",
            (user::Entity, user::Column::CreatedAt),
        ));
        filter_defs.extend(FilterDefinition::date_range(
            "updatedAt",
            (user::Entity, user::Column::UpdatedAt),
        ));

        let search_defs = vec![
            SearchDefinition::contains("name", (user::Entity, user::Column::Name)),
            SearchDefinition::contains("email", (user::Entity, user::Column::Email)),
            SearchDefinition::contains("Role.name", (role::Entity, role::Column::Name)),
        ];

        let order_defs = vec![
            OrderDefinition::case_insensitive("name", (user::Entity, user::Column::Name)),
            OrderDefinition::case_insensitive("email", (user::Entity, user::Column::Email)),
            OrderDefinition::column("createdAt", (user::Entity, user::Column::CreatedAt)),
        ];

        let mut query = user::Entity::find()
            .left_join(role::Entity)
            .filter(user::Column::IsDeleted.ne(true));

        query = filters.apply_search(query, &search_defs);
        query = filters.apply_filters(query, &filter_defs);

        query = filters.apply_order(query, &order_defs, (user::Entity, user::Column::CreatedAt));

        let (records, total) = filters.paginate(query, db).await?;

        let items = records.into_iter().map(UserResponse::from).collect();

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    pub async fn get_user_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<UserResponse, AppError> {
        let u = user::Entity::find_by_id(id.to_string())
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        Ok(UserResponse::from(u))
    }

    pub async fn create_user(
        payload: CreateUserRequest,
        db: &DatabaseConnection,
    ) -> Result<UserResponse, AppError> {
        let exists = user::Entity::find()
            .filter(user::Column::Email.eq(&payload.email))
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?;

        if exists.is_some() {
            return Err(AppError::Conflict(
                "E-mail já cadastrado no sistema".to_string(),
            ));
        }

        let role_exists = role::Entity::find_by_id(&payload.id_role).one(db).await?;
        if role_exists.is_none() {
            return Err(AppError::BadRequest(
                "ID de perfil (role) fornecido inválido".to_string(),
            ));
        }

        let auth_id = Uuid::new_v4().to_string();
        let hashed_pass = AuthService::hash_password(&payload.password)?;

        let active_auth = auth::ActiveModel {
            id: Set(auth_id.clone()),
            password: Set(Some(hashed_pass)),
            request_password_token: Set(None),
            request_password_expiration: Set(None),
            retries: Set(0),
            first_access: Set(true),
            active: Set(true),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };
        active_auth.insert(db).await?;

        let user_id = Uuid::new_v4().to_string();
        let active_user = user::ActiveModel {
            id: Set(user_id.clone()),
            name: Set(payload.name),
            email: Set(payload.email),
            phone: Set(payload.phone),
            document: Set(payload.document),
            cognito_id: Set(None),
            active: Set(true),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            avatar: Set(None),
            id_auth: Set(Some(auth_id)),
            id_role: Set(payload.id_role),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let u = active_user.insert(db).await?;

        Ok(UserResponse::from(u))
    }

    pub async fn update_user(
        id: &str,
        payload: UpdateUserRequest,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<UserResponse, AppError> {
        let u = user::Entity::find_by_id(id.to_string())
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        if payload.email != u.email {
            let conflict = user::Entity::find()
                .filter(user::Column::Email.eq(&payload.email))
                .filter(user::Column::IsDeleted.ne(true))
                .one(db)
                .await?;
            if conflict.is_some() {
                return Err(AppError::Conflict(
                    "E-mail já está sendo utilizado por outro usuário".to_string(),
                ));
            }
        }

        let role_exists = role::Entity::find_by_id(&payload.id_role).one(db).await?;
        if role_exists.is_none() {
            return Err(AppError::BadRequest(
                "ID de perfil (role) fornecido inválido".to_string(),
            ));
        }

        let mut active_user: user::ActiveModel = u.into();
        active_user.name = Set(payload.name);
        active_user.email = Set(payload.email);
        active_user.id_role = Set(payload.id_role);
        active_user.phone = Set(payload.phone);
        active_user.document = Set(payload.document);

        if let Some(act) = payload.active {
            active_user.active = Set(act);
        }

        active_user.updated_at = Set(chrono::Utc::now().into());

        let updated = active_user.update(db).await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(UserResponse::from(updated))
    }

    pub async fn delete_user(
        id: &str,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<(), AppError> {
        let u = user::Entity::find_by_id(id.to_string())
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        let now = chrono::Utc::now();

        if let Some(ref auth_id) = u.id_auth {
            if let Some(auth_rec) = auth::Entity::find_by_id(auth_id).one(db).await? {
                let mut active_auth: auth::ActiveModel = auth_rec.into();
                active_auth.active = Set(false);
                active_auth.is_deleted = Set(Some(true));
                active_auth.deleted_at = Set(Some(now.into()));
                active_auth.password = Set(None);
                active_auth.update(db).await?;
            }
        }

        let mut active_user: user::ActiveModel = u.into();
        let unique_uuid = Uuid::new_v4().to_string();

        active_user.name = Set("Deleted User".to_string());
        active_user.email = Set(format!(
            "deleted-anonymized-{}@deleted.com",
            &unique_uuid[..8]
        ));
        active_user.phone = Set(Some("00000000000".to_string()));
        active_user.document = Set(Some("00000000000".to_string()));
        active_user.active = Set(false);
        active_user.is_deleted = Set(Some(true));
        active_user.deleted_at = Set(Some(now.into()));
        active_user.avatar = Set(None);
        active_user.update(db).await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(())
    }

    pub async fn toggle_user_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<UserResponse, AppError> {
        let u = user::Entity::find_by_id(id.to_string())
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        let mut active_user: user::ActiveModel = u.into();
        active_user.active = Set(active);
        active_user.updated_at = Set(chrono::Utc::now().into());

        let updated = active_user.update(db).await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(UserResponse::from(updated))
    }
}
