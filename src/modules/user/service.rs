use crate::{
    core::crud::CrudEntity,
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
        let query = user::Entity::find()
            .left_join(role::Entity)
            .filter(user::Column::IsDeleted.ne(true));

        crate::core::crud::list_records_with_query::<user::Entity, UserResponse, _>(
            filters,
            db,
            query,
            &user::Entity::filter_definitions(),
            &user::Entity::search_definitions(),
            &user::Entity::order_definitions(),
            (user::Entity, user::Entity::default_order_column()),
            UserResponse::from,
        )
        .await
    }

    pub async fn get_user_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<UserResponse, AppError> {
        let u = crate::core::crud::get_by_id::<user::Entity>(id, db).await?;
        Ok(UserResponse::from(u))
    }

    pub async fn create_user(
        payload: CreateUserRequest,
        db: &DatabaseConnection,
    ) -> Result<UserResponse, AppError> {
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

        let active_user = user::ActiveModel {
            name: Set(payload.name),
            email: Set(payload.email),
            phone: Set(payload.phone),
            document: Set(payload.document),
            id_auth: Set(Some(auth_id)),
            id_role: Set(payload.id_role),
            ..Default::default()
        };

        let u = crate::core::crud::create_record::<user::Entity, _>(db, active_user).await?;

        Ok(UserResponse::from(u))
    }

    pub async fn update_user(
        id: &str,
        payload: UpdateUserRequest,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<UserResponse, AppError> {
        let role_exists = role::Entity::find_by_id(&payload.id_role).one(db).await?;
        if role_exists.is_none() {
            return Err(AppError::BadRequest(
                "ID de perfil (role) fornecido inválido".to_string(),
            ));
        }

        let mut active_user = user::ActiveModel {
            id: Set(id.to_string()),
            name: Set(payload.name),
            email: Set(payload.email),
            id_role: Set(payload.id_role),
            phone: Set(payload.phone),
            document: Set(payload.document),
            ..Default::default()
        };

        if let Some(act) = payload.active {
            active_user.active = Set(act);
        }

        let updated = crate::core::crud::update_record::<user::Entity, _>(db, active_user).await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(UserResponse::from(updated))
    }

    pub async fn delete_user(
        id: &str,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<(), AppError> {
        let u = crate::core::crud::get_by_id::<user::Entity>(id, db).await?;

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
        let updated =
            crate::core::crud::toggle_status::<user::Entity, user::ActiveModel>(id, active, db)
                .await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(UserResponse::from(updated))
    }
}
