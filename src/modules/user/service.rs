use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, Order, Condition,
    PaginatorTrait,
};
use uuid::Uuid;
use crate::{
    errors::AppError,
    infra::auth::AuthService,
    infra::cache::Cache,
    core::query_parser::{ParsedFilters, PaginatedResponse},
    models::{auth, role, user},
    modules::user::schemas::{CreateUserRequest, UpdateUserRequest, UserResponse},
};

pub struct UserModuleService;

impl UserModuleService {
    /// Paginated list search with dynamic filters and left join on Role table for role name searches
    pub async fn list_users(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<UserResponse>, AppError> {
        // Base query - left join with Role to allow searching on Role.name
        let mut query = user::Entity::find()
            .left_join(role::Entity)
            .filter(user::Column::IsDeleted.ne(true));

        // Apply global searchWord across allowed searchFields
        if let Some(word) = filters.search_word {
            use sea_orm::sea_query::{Expr, Func};
            let lower_word = word.to_lowercase();
            let mut or_cond = Condition::any();
            for field in filters.search_fields {
                if field == "name" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col((user::Entity, user::Column::Name)))).like(format!("%{}%", lower_word)));
                } else if field == "email" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col((user::Entity, user::Column::Email)))).like(format!("%{}%", lower_word)));
                } else if field == "Role.name" {
                    or_cond = or_cond.add(Expr::expr(Func::lower(Expr::col((role::Entity, role::Column::Name)))).like(format!("%{}%", lower_word)));
                }
            }
            query = query.filter(or_cond);
        }

        // Apply creation date ranges
        if let Some(start) = filters.start_date {
            query = query.filter(user::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filters.end_date {
            query = query.filter(user::Column::CreatedAt.lte(end));
        }

        // Count total matching records
        let total = query.clone().paginate(db, 1).num_items().await?;

        // Apply sorting
        if let Some(field) = filters.order_by {
            let dir = if filters.order_direction.to_lowercase() == "desc" {
                Order::Desc
            } else {
                Order::Asc
            };
            
            if field == "name" {
                query = query.order_by(user::Column::Name, dir);
            } else if field == "email" {
                query = query.order_by(user::Column::Email, dir);
            } else {
                query = query.order_by(user::Column::CreatedAt, dir);
            }
        } else {
            query = query.order_by(user::Column::CreatedAt, Order::Desc);
        }

        // Apply paging offset & limit
        let offset = filters.page * filters.size;
        let records = query
            .limit(filters.size)
            .offset(offset)
            .all(db)
            .await?;

        let items = records
            .into_iter()
            .map(|u| UserResponse {
                id: u.id,
                name: u.name,
                email: u.email,
                phone: u.phone,
                document: u.document,
                active: u.active,
                id_role: u.id_role,
                created_at: u.created_at.to_rfc3339(),
                updated_at: u.updated_at.to_rfc3339(),
            })
            .collect();

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    /// Fetches a single active user by ID
    pub async fn get_user_by_id(id: &str, db: &DatabaseConnection) -> Result<UserResponse, AppError> {
        let u = user::Entity::find_by_id(id.to_string())
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        Ok(UserResponse {
            id: u.id,
            name: u.name,
            email: u.email,
            phone: u.phone,
            document: u.document,
            active: u.active,
            id_role: u.id_role,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        })
    }

    /// Creates a user, hashes their password, and creates linked credentials
    pub async fn create_user(
        payload: CreateUserRequest,
        db: &DatabaseConnection,
    ) -> Result<UserResponse, AppError> {
        // Validate if email already exists
        let exists = user::Entity::find()
            .filter(user::Column::Email.eq(&payload.email))
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?;

        if exists.is_some() {
            return Err(AppError::Conflict("E-mail já cadastrado no sistema".to_string()));
        }

        // Verify if role exists
        let role_exists = role::Entity::find_by_id(&payload.id_role).one(db).await?;
        if role_exists.is_none() {
            return Err(AppError::BadRequest("ID de perfil (role) fornecido inválido".to_string()));
        }

        // 1. Create credentials in Auth table
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

        // 2. Create User record
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

        Ok(UserResponse {
            id: u.id,
            name: u.name,
            email: u.email,
            phone: u.phone,
            document: u.document,
            active: u.active,
            id_role: u.id_role,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        })
    }

    /// Edits a user, invalidates sessions, and saves to database
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

        // Check email conflict
        if payload.email != u.email {
            let conflict = user::Entity::find()
                .filter(user::Column::Email.eq(&payload.email))
                .filter(user::Column::IsDeleted.ne(true))
                .one(db)
                .await?;
            if conflict.is_some() {
                return Err(AppError::Conflict("E-mail já está sendo utilizado por outro usuário".to_string()));
            }
        }

        // Verify if role exists
        let role_exists = role::Entity::find_by_id(&payload.id_role).one(db).await?;
        if role_exists.is_none() {
            return Err(AppError::BadRequest("ID de perfil (role) fornecido inválido".to_string()));
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

        // Invalidate all active sessions for this user (complying with test_session_invalidation_on_mutation)
        cache.invalidate_user_sessions(id).await?;

        Ok(UserResponse {
            id: updated.id,
            name: updated.name,
            email: updated.email,
            phone: updated.phone,
            document: updated.document,
            active: updated.active,
            id_role: updated.id_role,
            created_at: updated.created_at.to_rfc3339(),
            updated_at: updated.updated_at.to_rfc3339(),
        })
    }

    /// Soft deletes a user, redacts and anonymizes sensitive data (LGPD), and destroys sessions
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

        // 1. Soft delete and Anonymize Auth credentials if linked
        if let Some(ref auth_id) = u.id_auth {
            if let Some(auth_rec) = auth::Entity::find_by_id(auth_id).one(db).await? {
                let mut active_auth: auth::ActiveModel = auth_rec.into();
                active_auth.active = Set(false);
                active_auth.is_deleted = Set(Some(true));
                active_auth.deleted_at = Set(Some(now.into()));
                active_auth.password = Set(None); // Wipe credentials completely
                active_auth.update(db).await?;
            }
        }

        // 2. Anonymize User table data (satisfies test_lgpd_user_anonymization)
        let mut active_user: user::ActiveModel = u.into();
        let unique_uuid = Uuid::new_v4().to_string();
        
        active_user.name = Set("Deleted User".to_string());
        active_user.email = Set(format!("deleted-anonymized-{}@deleted.com", &unique_uuid[..8]));
        active_user.phone = Set(Some("00000000000".to_string()));
        active_user.document = Set(Some("00000000000".to_string()));
        active_user.active = Set(false);
        active_user.is_deleted = Set(Some(true));
        active_user.deleted_at = Set(Some(now.into()));
        active_user.avatar = Set(None);
        active_user.update(db).await?;

        // 3. Immediately invalidate and wipe active Redis sessions
        cache.invalidate_user_sessions(id).await?;

        Ok(())
    }
}
