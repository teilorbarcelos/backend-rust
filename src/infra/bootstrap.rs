use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, ColumnTrait, Set};
use crate::{
    infra::auth::AuthService,
    models::{auth, feature, role, role_feature, user},
};

pub async fn bootstrap_database(db: &DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!(" Iniciando rotina de bootstrap do banco de dados...");

    // 1. Check and Seed Features
    let features_data = vec![
        ("user", "Gestão de Usuários"),
        ("role", "Gestão de Perfis de Acesso"),
        ("product", "Gestão de Produtos"),
    ];

    for (id, desc) in features_data {
        let exists = feature::Entity::find_by_id(id.to_string()).one(db).await?;
        if exists.is_none() {
            let active_feature = feature::ActiveModel {
                id: Set(id.to_string()),
                name: Set(id.to_string()),
                description: Set(desc.to_string()),
                active: Set(true),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
            };
            active_feature.insert(db).await?;
            tracing::info!("Feature '{}' injetada com sucesso.", id);
        }
    }

    // 2. Check and Seed Administrator Role
    let admin_role_id = "administrator";
    let exists_role = role::Entity::find_by_id(admin_role_id.to_string()).one(db).await?;
    if exists_role.is_none() {
        let active_role = role::ActiveModel {
            id: Set(admin_role_id.to_string()),
            name: Set("Administrador".to_string()),
            description: Set("Perfil com acesso total ao sistema".to_string()),
            active: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
        };
        active_role.insert(db).await?;
        tracing::info!("Perfil 'administrator' injetado com sucesso.");
    } else {
        let role_item = exists_role.unwrap();
        if role_item.name != "Administrador" {
            let mut active_role: role::ActiveModel = role_item.into();
            active_role.name = Set("Administrador".to_string());
            active_role.update(db).await?;
            tracing::info!("Nome do perfil 'administrator' atualizado para 'Administrador'.");
        }
    }

    // 3. Check and Seed Role-Feature Mappings for Administrator
    let features = vec!["user", "role", "product"];
    for feat_id in features {
        let exists_mapping = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(admin_role_id))
            .filter(role_feature::Column::IdFeature.eq(feat_id))
            .one(db)
            .await?;

        if exists_mapping.is_none() {
            let active_mapping = role_feature::ActiveModel {
                id_role: Set(admin_role_id.to_string()),
                id_feature: Set(feat_id.to_string()),
                create: Set(true),
                view: Set(true),
                activate: Set(true),
                delete: Set(true),
            };
            active_mapping.insert(db).await?;
            tracing::info!("Permissões da feature '{}' vinculadas ao 'administrator'.", feat_id);
        }
    }

    // 4. Check and Seed Supreme Admin User
    let admin_email = "admin@email.com";
    let exists_user = user::Entity::find()
        .filter(user::Column::Email.eq(admin_email))
        .one(db)
        .await?;

    if exists_user.is_none() {
        // Create Auth Credentials
        let auth_id = "auth-admin-uuid-00000000000000000001".to_string();
        let pass_hash = AuthService::hash_password("admin@123").unwrap();

        let active_auth = auth::ActiveModel {
            id: Set(auth_id.clone()),
            password: Set(Some(pass_hash)),
            request_password_token: Set(None),
            request_password_expiration: Set(None),
            retries: Set(0),
            first_access: Set(false),
            active: Set(true),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };
        active_auth.insert(db).await?;

        // Create User Details linked to Credentials and Admin Role
        let user_id = "user-admin-uuid-00000000000000000001".to_string();
        let active_user = user::ActiveModel {
            id: Set(user_id),
            name: Set("Supreme Administrator".to_string()),
            phone: Set(Some("11999999999".to_string())),
            email: Set(admin_email.to_string()),
            cognito_id: Set(None),
            active: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            document: Set(Some("00000000000".to_string())),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            avatar: Set(None),
            id_auth: Set(Some(auth_id)),
            id_role: Set(admin_role_id.to_string()),
        };
        active_user.insert(db).await?;

        tracing::info!("Usuário Administrador Supremo 'admin@email.com' semeado com sucesso.");
    }

    tracing::info!(" Rotina de bootstrap concluída.");
    Ok(())
}
