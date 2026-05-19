#[macro_export]
macro_rules! auth_route {
    ($db:expr, $feature:expr, $action:expr, $handler:expr) => {
        $handler
            .layer(axum::middleware::from_fn_with_state($db.clone(), crate::middleware::rbac::rbac_middleware))
            .layer(axum::Extension(crate::middleware::rbac::RequirePermission {
                feature: $feature,
                action: $action,
            }))
    };
}

use crate::{
    errors::AppError,
    middleware::auth::CurrentUser,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sea_orm::DatabaseConnection;

#[derive(Clone, Debug)]
pub struct RequirePermission {
    pub feature: &'static str,
    pub action: &'static str,
}

/// Validates that the current user has the required permission for the specified feature action
pub async fn authorize(
    user_id: &str,
    feature: &str,
    action: &str,
    db: &DatabaseConnection,
) -> Result<(), AppError> {
    use crate::models::{role, role_feature, user};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // 1. Fetch user to check active status and get their role ID
    let u = user::Entity::find_by_id(user_id.to_string())
        .filter(user::Column::IsDeleted.ne(true))
        .one(db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Usuário não encontrado ou inativo".to_string()))?;

    if !u.active {
        return Err(AppError::Forbidden(
            "Usuário inativo no sistema".to_string(),
        ));
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
    if r.id == "administrator" {
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

/// Dynamic RBAC authorization middleware. It reads permission requirements from
/// request extensions (added by individual routes) and authorises the request.
pub async fn rbac_middleware(
    State(db): State<DatabaseConnection>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let current_user = req
        .extensions()
        .get::<CurrentUser>()
        .ok_or_else(|| AppError::Unauthorized("Usuário não autenticado".to_string()))?
        .clone();

    if let Some(perm) = req.extensions().get::<RequirePermission>() {
        authorize(&current_user.id, perm.feature, perm.action, &db).await?;
    }

    Ok(next.run(req).await)
}
