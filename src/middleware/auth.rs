use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use crate::{
    errors::AppError,
    infra::auth::{AuthService, Claims},
    infra::cache::Cache,
    config::AppConfig,
};
use sea_orm::DatabaseConnection;

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

/// Authentication middleware. Checks Bearer JWT token, validates active session in Redis cache,
/// and binds current user credentials as request Extension.
pub async fn auth_middleware(
    State((cache, config)): State<(Cache, AppConfig)>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Cabeçalho de autorização ausente".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized("Token deve ser do tipo Bearer".to_string()));
    }

    let token = &auth_header[7..];

    // Decode and verify JWT signature
    let claims: Claims = AuthService::verify_token(token, &config.jwt_secret)?;

    // Query Redis cache to verify session isn't expired or revoked
    let is_valid = cache.validate_session(&claims.sub, &format!("access:{}", token)).await?;
    if !is_valid {
        return Err(AppError::Unauthorized("Sessão revogada ou expirada".to_string()));
    }

    // Insert user info into extensions for downstream extraction
    let current_user = CurrentUser {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
    };
    req.extensions_mut().insert(current_user.clone());

    let mut res = next.run(req).await;
    res.extensions_mut().insert(current_user);
    Ok(res)
}

/// Validates that the current user has the required permission for the specified feature action
pub async fn authorize(
    user_id: &str,
    feature: &str,
    action: &str,
    db: &DatabaseConnection,
) -> Result<(), AppError> {
    use crate::models::{user, role, role_feature};
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

    // 1. Fetch user to check active status and get their role ID
    let u = user::Entity::find_by_id(user_id.to_string())
        .filter(user::Column::IsDeleted.ne(true))
        .one(db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Usuário não encontrado ou inativo".to_string()))?;

    if !u.active {
        return Err(AppError::Forbidden("Usuário inativo no sistema".to_string()));
    }

    // 2. Fetch role to check active status
    let r = role::Entity::find_by_id(u.id_role.clone())
        .filter(role::Column::IsDeleted.ne(true))
        .one(db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Perfil não encontrado ou inativo".to_string()))?;

    if !r.active {
        return Err(AppError::Forbidden("Perfil de acesso inativo".to_string()));
    }

    // 3. Bypass permission check if user is administrator
    if r.name == "administrator" {
        return Ok(());
    }

    // 4. Query permissions for the user's role and feature
    let mapping = role_feature::Entity::find()
        .filter(role_feature::Column::IdRole.eq(&u.id_role))
        .filter(role_feature::Column::IdFeature.eq(feature))
        .one(db)
        .await?;

    let allowed = if let Some(m) = mapping {
        match action {
            "create" => m.create,
            "view" => m.view,
            "activate" => m.activate,
            "delete" => m.delete,
            _ => false,
        }
    } else {
        false
    };

    if !allowed {
        return Err(AppError::Forbidden(format!(
            "Sem permissão para executar a ação '{}' na funcionalidade '{}'",
            action, feature
        )));
    }

    Ok(())
}
