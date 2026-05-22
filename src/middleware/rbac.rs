#[macro_export]
macro_rules! auth_route {
    ($db:expr, $feature:expr, $action:expr, $handler:expr) => {
        $handler
            .layer(axum::middleware::from_fn_with_state(
                $db.clone(),
                $crate::middleware::rbac::rbac_middleware,
            ))
            .layer(axum::Extension(
                $crate::middleware::rbac::RequirePermission {
                    feature: $feature,
                    action: $action,
                },
            ))
    };
}

use crate::{errors::AppError, middleware::auth::CurrentUser};
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

pub async fn authorize(
    user_id: &str,
    feature: &str,
    action: &str,
    db: &DatabaseConnection,
) -> Result<(), AppError> {
    use crate::models::{role, role_feature, user};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

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

    let r = role::Entity::find_by_id(u.id_role.clone())
        .filter(role::Column::IsDeleted.ne(true))
        .one(db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Perfil não encontrado ou inativo".to_string()))?;

    if !r.active {
        return Err(AppError::Forbidden("Perfil de acesso inativo".to_string()));
    }

    if r.id == "administrator" {
        return Ok(());
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authorize_invalid_action() {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5432/backend_rust".to_string()
        });
        if let Ok(db) = sea_orm::Database::connect(&database_url).await {
            let res = authorize("non-existent-user-id", "product", "invalid_action", &db).await;
            assert!(res.is_err());

            use crate::models::{role, role_feature, user};
            use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

            let role_id = uuid::Uuid::new_v4().to_string();
            let temp_role = role::ActiveModel {
                id: Set(role_id.clone()),
                name: Set("Temp Role".to_string()),
                description: Set("Temp".to_string()),
                active: Set(true),
                is_deleted: Set(Some(false)),
                deleted_at: Set(None),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
            };
            temp_role.insert(&db).await.unwrap();

            let mapping = role_feature::ActiveModel {
                id_role: Set(role_id.clone()),
                id_feature: Set("product".to_string()),
                create: Set(true),
                view: Set(true),
                activate: Set(true),
                delete: Set(true),
            };
            mapping.insert(&db).await.unwrap();

            let user_id = uuid::Uuid::new_v4().to_string();
            let temp_user = user::ActiveModel {
                id: Set(user_id.clone()),
                name: Set("Temp User".to_string()),
                email: Set(format!("{}@temp.com", user_id)),
                id_role: Set(role_id.clone()),
                active: Set(true),
                is_deleted: Set(Some(false)),
                deleted_at: Set(None),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
                ..Default::default()
            };
            temp_user.insert(&db).await.unwrap();

            let res = authorize(&user_id, "product", "invalid_action", &db).await;
            assert!(res.is_err());
            assert_eq!(
                res.unwrap_err().message(),
                "Sem permissão para executar a ação 'invalid_action' na funcionalidade 'product'"
            );

            let _ = user::Entity::delete_by_id(&user_id).exec(&db).await;
            let _ = role_feature::Entity::delete_many()
                .filter(role_feature::Column::IdRole.eq(&role_id))
                .exec(&db)
                .await;
            let _ = role::Entity::delete_by_id(&role_id).exec(&db).await;
        }
    }
}
