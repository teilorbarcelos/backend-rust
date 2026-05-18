use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use crate::{
    errors::AppError,
    infra::auth::AuthService,
    infra::cache::Cache,
    config::AppConfig,
    models::{auth, role, role_feature, user},
    modules::auth::schemas::{AuthResponse, LoginRequest, PermissionInfo, RoleInfo, SimpleStatusResponse, UserInfo, UserMeResponse},
};

pub struct AuthModuleService;

impl AuthModuleService {
    /// Validates login credentials and returns signed tokens & user metadata
    pub async fn login(
        payload: LoginRequest,
        db: &DatabaseConnection,
        cache: &Cache,
        config: &AppConfig,
    ) -> Result<AuthResponse, AppError> {
        // 1. Find User by Email
        let user_record = user::Entity::find()
            .filter(user::Column::Email.eq(&payload.email))
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Credenciais inválidas".to_string()))?;

        // 2. Block inactive users
        if !user_record.active {
            return Err(AppError::Forbidden("Usuário inativo. Login não permitido.".to_string()));
        }

        // 3. Find and check Role
        let role_record = role::Entity::find_by_id(&user_record.id_role)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Perfil do usuário não encontrado".to_string()))?;

        if !role_record.active {
            return Err(AppError::Forbidden("Perfil de acesso inativo. Login não permitido.".to_string()));
        }

        // 4. Find Auth Credentials
        let auth_id = user_record.id_auth.as_ref()
            .ok_or_else(|| AppError::Unauthorized("Credenciais não configuradas".to_string()))?;
            
        let auth_record = auth::Entity::find_by_id(auth_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Credenciais não encontradas".to_string()))?;

        if !auth_record.active {
            return Err(AppError::Forbidden("Credenciais de acesso inativas.".to_string()));
        }

        // 5. Verify bcrypt password
        let hash = auth_record.password.as_deref().unwrap_or("");
        let matches = AuthService::verify_password(&payload.password, hash)?;
        if !matches {
            return Err(AppError::Unauthorized("Credenciais inválidas".to_string()));
        }

        // 6. Fetch RBAC Mapped Permissions
        let permissions_records = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&role_record.id))
            .all(db)
            .await?;

        let permissions = permissions_records
            .into_iter()
            .map(|p| PermissionInfo {
                feature: p.id_feature,
                create: p.create,
                view: p.view,
                activate: p.activate,
                delete: p.delete,
            })
            .collect::<Vec<_>>();

        // 7. Generate access & refresh tokens
        let (access_token, refresh_token) = AuthService::generate_tokens(
            &user_record.id,
            &user_record.email,
            &role_record.id,
            &config.jwt_secret,
            config.jwt_expires_in,
        )?;

        // 8. Cache token session in Redis
        cache.create_session(&user_record.id, &access_token, config.jwt_expires_in).await?;

        Ok(AuthResponse {
            token: access_token,
            refresh_token,
            user: UserInfo {
                id: user_record.id,
                name: user_record.name,
                email: user_record.email,
                role: RoleInfo {
                    id: role_record.id,
                    name: role_record.name,
                    permissions,
                },
            },
        })
    }

    /// Fetches currently authenticated user context
    pub async fn get_me(
        user_id: &str,
        db: &DatabaseConnection,
    ) -> Result<UserMeResponse, AppError> {
        let user_record = user::Entity::find_by_id(user_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        let role_record = role::Entity::find_by_id(&user_record.id_role)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Perfil não encontrado".to_string()))?;

        let permissions_records = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&role_record.id))
            .all(db)
            .await?;

        let permissions = permissions_records
            .into_iter()
            .map(|p| PermissionInfo {
                feature: p.id_feature,
                create: p.create,
                view: p.view,
                activate: p.activate,
                delete: p.delete,
            })
            .collect::<Vec<_>>();

        Ok(UserMeResponse {
            user: UserInfo {
                id: user_record.id,
                name: user_record.name,
                email: user_record.email,
                role: RoleInfo {
                    id: role_record.id,
                    name: role_record.name,
                    permissions,
                },
            },
        })
    }

    /// Logs out a user session from Redis cache
    pub async fn logout(user_id: &str, cache: &Cache) -> Result<SimpleStatusResponse, AppError> {
        cache.invalidate_user_sessions(user_id).await?;
        Ok(SimpleStatusResponse { status: true })
    }
}
