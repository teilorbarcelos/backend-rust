use std::fs;
use std::path::Path;

struct Field {
    name: String,
    rust_type: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    let entity_name = &args[1];
    let entity_slug = entity_name.to_lowercase();
    let fields = parse_fields(&args[2..]);

    println!("🛠️  Iniciando geração do CRUD para '{}'...", entity_name);

    generate_model(entity_name, &entity_slug, &fields);

    let module_dir = format!("src/modules/{}", entity_slug);
    if !Path::new(&module_dir).exists() {
        fs::create_dir_all(&module_dir).expect("Falha ao criar diretório do módulo");
    }

    generate_schemas(entity_name, &entity_slug, &fields);
    generate_service(entity_name, &entity_slug, &fields);
    generate_controller(entity_name, &entity_slug);
    generate_routes(entity_name, &entity_slug);

    register_model(&entity_slug);
    register_module(entity_name, &entity_slug);

    println!(
        "✅ CRUD gerado com sucesso para a feature '{}'!",
        entity_name
    );
    println!("💡 Dica: Rode 'cargo build' para validar a compilação do seu novo CRUD.");
}

fn print_usage() {
    println!("📖 Mage Backend - CLI CRUD Generator (Rust)");
    println!("Uso:");
    println!("  cargo run --bin generator <NomeEntidade> [campo:tipo ...]");
    println!("\nTipos suportados: string, int, bool, decimal, float, date");
    println!("\nExemplo:");
    println!(
        "  cargo run --bin generator Customer name:string email:string active:bool price:decimal"
    );
}

fn parse_fields(args: &[String]) -> Vec<Field> {
    let mut fields = Vec::new();
    for arg in args {
        let parts: Vec<&str> = arg.split(':').collect();
        if parts.len() != 2 {
            continue;
        }
        let name = parts[0].to_string();
        let raw_type = parts[1].to_lowercase();

        let rust_type = match raw_type.as_str() {
            "int" => "i32".to_string(),
            "bool" => "bool".to_string(),
            "decimal" => "Decimal".to_string(),
            "float" => "f64".to_string(),
            "date" => "DateTimeWithTimeZone".to_string(),
            _ => "String".to_string(),
        };

        fields.push(Field { name, rust_type });
    }
    fields
}

fn generate_model(entity_name: &str, slug: &str, fields: &[Field]) {
    let path = format!("src/models/{}.rs", slug);
    let mut code = String::new();

    code.push_str("use sea_orm::entity::prelude::*;\n");
    code.push_str("use serde::{Deserialize, Serialize};\n");
    code.push_str("use utoipa::ToSchema;\n\n");

    code.push_str(&format!(
        "#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]\n"
    ));
    code.push_str(&format!(
        "#[sea_orm(schema_name = \"public\", table_name = \"{}\")]\n",
        entity_name
    ));
    code.push_str("pub struct Model {\n");
    code.push_str("    #[sea_orm(primary_key, auto_increment = false)]\n");
    code.push_str("    pub id: String,\n");

    for field in fields {
        code.push_str(&format!("    pub {}: {},\n", field.name, field.rust_type));
    }

    code.push_str("    pub active: bool,\n");
    code.push_str("    pub created_at: DateTimeWithTimeZone,\n");
    code.push_str("    pub updated_at: DateTimeWithTimeZone,\n");
    code.push_str("    pub is_deleted: Option<bool>,\n");
    code.push_str("    pub deleted_at: Option<DateTimeWithTimeZone>,\n");
    code.push_str("}\n\n");

    code.push_str("#[derive(Copy, Clone, Debug, DeriveRelation)]\n");
    code.push_str("pub enum Relation {}\n\n");
    code.push_str("impl ActiveModelBehavior for ActiveModel {}\n");

    fs::write(path, code).expect("Falha ao salvar modelo");
    println!("  📄 [NEW] src/models/{}.rs", slug);
}

fn generate_schemas(entity_name: &str, slug: &str, fields: &[Field]) {
    let path = format!("src/modules/{}/schemas.rs", slug);
    let mut code = String::new();

    code.push_str("use serde::{Deserialize, Serialize};\n");
    code.push_str("use utoipa::ToSchema;\n");
    code.push_str("use sea_orm::prelude::Decimal;\n\n");

    code.push_str(&format!("#[derive(Debug, Deserialize, ToSchema)]\n"));
    code.push_str(&format!("pub struct Create{}Request {{\n", entity_name));
    for field in fields {
        code.push_str(&format!("    pub {}: {},\n", field.name, field.rust_type));
    }
    code.push_str("}\n\n");

    code.push_str(&format!("#[derive(Debug, Deserialize, ToSchema)]\n"));
    code.push_str(&format!("pub struct Update{}Request {{\n", entity_name));
    for field in fields {
        code.push_str(&format!("    pub {}: {},\n", field.name, field.rust_type));
    }
    code.push_str("    pub active: Option<bool>,\n");
    code.push_str("}\n\n");

    code.push_str(&format!("#[derive(Debug, Serialize, ToSchema)]\n"));
    code.push_str(&format!("pub struct {}Response {{\n", entity_name));
    code.push_str("    pub id: String,\n");
    for field in fields {
        code.push_str(&format!("    pub {}: {},\n", field.name, field.rust_type));
    }
    code.push_str("    pub active: bool,\n");
    code.push_str("    pub created_at: String,\n");
    code.push_str("    pub updated_at: String,\n");
    code.push_str("}\n");

    fs::write(path, code).expect("Falha ao salvar schemas do módulo");
    println!("  📄 [NEW] src/modules/{}/schemas.rs", slug);
}

fn generate_service(entity_name: &str, slug: &str, fields: &[Field]) {
    let path = format!("src/modules/{}/service.rs", slug);
    let mut code = String::new();

    code.push_str("use sea_orm::{\n");
    code.push_str("    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,\n");
    code.push_str("    QueryFilter, QueryOrder, QuerySelect, Set, Order,\n");
    code.push_str("};\n");
    code.push_str("use uuid::Uuid;\n");
    code.push_str("use crate::{\n");
    code.push_str("    errors::AppError,\n");
    code.push_str("    core::query_parser::{ParsedFilters, PaginatedResponse},\n");
    code.push_str(&format!("    models::{},\n", slug));
    code.push_str(&format!(
        "    modules::{}::schemas::{{Create{}Request, {}Response, Update{}Request}},\n",
        slug, entity_name, entity_name, entity_name
    ));
    code.push_str("};\n\n");

    code.push_str(&format!("pub struct {}ModuleService;\n\n", entity_name));
    code.push_str(&format!("impl {}ModuleService {{\n", entity_name));

    code.push_str(&format!(
        "    pub async fn list_{}s(\n\
        \x20       filters: ParsedFilters,\n\
        \x20       db: &DatabaseConnection,\n\
        \x20   ) -> Result<PaginatedResponse<{}Response>, AppError> {{\n",
        slug, entity_name
    ));
    code.push_str(&format!(
        "        let mut query = {}::Entity::find()\n\
        \x20           .filter({}::Column::IsDeleted.ne(true));\n\n",
        slug, slug
    ));
    code.push_str(&format!(
        "        if let Some(word) = filters.search_word {{\n\
        \x20           query = query.filter({}::Column::Name.ilike(format!(\"%{{}}%\", word)));\n\
        \x20       }}\n\n",
        slug
    ));
    code.push_str("        let total = query.clone().count(db).await?;\n\n");
    code.push_str("        let dir = if filters.order_direction.to_lowercase() == \"desc\" { Order::Desc } else { Order::Asc };\n");
    code.push_str(&format!(
        "        query = query.order_by({}::Column::CreatedAt, dir);\n\n",
        slug
    ));
    code.push_str("        let offset = filters.page * filters.size;\n");
    code.push_str(
        "        let records = query.limit(filters.size).offset(offset).all(db).await?;\n\n",
    );
    code.push_str("        let items = records.into_iter().map(|p| {\n");
    code.push_str(&format!("            {}Response {{\n", entity_name));
    code.push_str("                id: p.id,\n");
    for field in fields {
        code.push_str(&format!(
            "                {}: p.{},\n",
            field.name, field.name
        ));
    }
    code.push_str("                active: p.active,\n");
    code.push_str("                created_at: p.created_at.to_rfc3339(),\n");
    code.push_str("                updated_at: p.updated_at.to_rfc3339(),\n");
    code.push_str("            }\n");
    code.push_str("        }).collect();\n\n");
    code.push_str(
        "        Ok(PaginatedResponse { items, total, page: filters.page, size: filters.size })\n",
    );
    code.push_str("    }\n\n");

    code.push_str(&format!(
        "    pub async fn get_{}_by_id(id: &str, db: &DatabaseConnection) -> Result<{}Response, AppError> {{\n",
        slug, entity_name
    ));
    code.push_str(&format!(
        "        let p = {}::Entity::find_by_id(id.to_string())\n\
        \x20           .filter({}::Column::IsDeleted.ne(true))\n\
        \x20           .one(db).await?\n\
        \x20           .ok_or_else(|| AppError::NotFound(\"Registro não encontrado\".to_string()))?;\n\n",
        slug, slug
    ));
    code.push_str(&format!("        Ok({}Response {{\n", entity_name));
    code.push_str("            id: p.id,\n");
    for field in fields {
        code.push_str(&format!("            {}: p.{},\n", field.name, field.name));
    }
    code.push_str("            active: p.active,\n");
    code.push_str("            created_at: p.created_at.to_rfc3339(),\n");
    code.push_str("            updated_at: p.updated_at.to_rfc3339(),\n");
    code.push_str("        })\n");
    code.push_str("    }\n\n");

    code.push_str(&format!(
        "    pub async fn create_{}(payload: Create{}Request, db: &DatabaseConnection) -> Result<{}Response, AppError> {{\n",
        slug, entity_name, entity_name
    ));
    code.push_str("        let new_id = Uuid::new_v4().to_string();\n");
    code.push_str(&format!(
        "        let active_model = {}::ActiveModel {{\n",
        slug
    ));
    code.push_str("            id: Set(new_id),\n");
    for field in fields {
        code.push_str(&format!(
            "            {}: Set(payload.{}),\n",
            field.name, field.name
        ));
    }
    code.push_str("            active: Set(true),\n");
    code.push_str("            is_deleted: Set(Some(false)),\n");
    code.push_str("            deleted_at: Set(None),\n");
    code.push_str("            created_at: Set(chrono::Utc::now().into()),\n");
    code.push_str("            updated_at: Set(chrono::Utc::now().into()),\n");
    code.push_str("        };\n\n");
    code.push_str("        let p = active_model.insert(db).await?;\n\n");
    code.push_str(&format!("        Ok({}Response {{\n", entity_name));
    code.push_str("            id: p.id,\n");
    for field in fields {
        code.push_str(&format!("            {}: p.{},\n", field.name, field.name));
    }
    code.push_str("            active: p.active,\n");
    code.push_str("            created_at: p.created_at.to_rfc3339(),\n");
    code.push_str("            updated_at: p.updated_at.to_rfc3339(),\n");
    code.push_str("        })\n");
    code.push_str("    }\n\n");

    code.push_str(&format!(
        "    pub async fn update_{}(id: &str, payload: Update{}Request, db: &DatabaseConnection) -> Result<{}Response, AppError> {{\n",
        slug, entity_name, entity_name
    ));
    code.push_str(&format!(
        "        let p = {}::Entity::find_by_id(id.to_string())\n\
        \x20           .filter({}::Column::IsDeleted.ne(true))\n\
        \x20           .one(db).await?\n\
        \x20           .ok_or_else(|| AppError::NotFound(\"Registro não encontrado\".to_string()))?;\n\n",
        slug, slug
    ));
    code.push_str(&format!(
        "        let mut active_model: {}::ActiveModel = p.into();\n",
        slug
    ));
    for field in fields {
        code.push_str(&format!(
            "        active_model.{} = Set(payload.{});\n",
            field.name, field.name
        ));
    }
    code.push_str(
        "        if let Some(act) = payload.active { active_model.active = Set(act); }\n",
    );
    code.push_str("        active_model.updated_at = Set(chrono::Utc::now().into());\n\n");
    code.push_str("        let updated = active_model.update(db).await?;\n\n");
    code.push_str(&format!("        Ok({}Response {{\n", entity_name));
    code.push_str("            id: updated.id,\n");
    for field in fields {
        code.push_str(&format!(
            "            {}: updated.{},\n",
            field.name, field.name
        ));
    }
    code.push_str("            active: updated.active,\n");
    code.push_str("            created_at: updated.created_at.to_rfc3339(),\n");
    code.push_str("            updated_at: updated.updated_at.to_rfc3339(),\n");
    code.push_str("        })\n");
    code.push_str("    }\n\n");

    code.push_str(&format!(
        "    pub async fn delete_{}(id: &str, db: &DatabaseConnection) -> Result<(), AppError> {{\n",
        slug
    ));
    code.push_str(&format!(
        "        let p = {}::Entity::find_by_id(id.to_string())\n\
        \x20           .filter({}::Column::IsDeleted.ne(true))\n\
        \x20           .one(db).await?\n\
        \x20           .ok_or_else(|| AppError::NotFound(\"Registro não encontrado\".to_string()))?;\n\n",
        slug, slug
    ));
    code.push_str(&format!(
        "        let mut active_model: {}::ActiveModel = p.into();\n",
        slug
    ));
    code.push_str("        active_model.active = Set(false);\n");
    code.push_str("        active_model.is_deleted = Set(Some(true));\n");
    code.push_str("        active_model.deleted_at = Set(Some(chrono::Utc::now().into()));\n");
    code.push_str("        active_model.update(db).await?;\n\n");
    code.push_str("        Ok(())\n");
    code.push_str("    }\n");

    code.push_str("}\n");

    fs::write(path, code).expect("Falha ao salvar service do módulo");
    println!("  📄 [NEW] src/modules/{}/service.rs", slug);
}

fn generate_controller(entity_name: &str, slug: &str) {
    let path = format!("src/modules/{}/controller.rs", slug);
    let mut code = String::new();

    code.push_str("use axum::{\n");
    code.push_str("    extract::{Path, Query, State},\n");
    code.push_str("    http::StatusCode,\n");
    code.push_str("    response::IntoResponse,\n");
    code.push_str("    Json,\n");
    code.push_str("};\n");
    code.push_str("use sea_orm::DatabaseConnection;\n");
    code.push_str("use crate::{\n");
    code.push_str("    errors::{AppError, AppJson},\n");
    code.push_str("    infra::cache::Cache,\n");
    code.push_str("    core::query_parser::{FilterParams, QueryValidator, PaginatedResponse},\n");
    code.push_str(&format!(
        "    modules::{}::schemas::{{Create{}Request, {}Response, Update{}Request}},\n",
        slug, entity_name, entity_name, entity_name
    ));
    code.push_str(&format!(
        "    modules::{}::service::{}ModuleService,\n",
        slug, entity_name
    ));
    code.push_str("};\n\n");

    code.push_str(&format!(
        "pub async fn list_{}s_handler(\n\
        \x20   State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,\n\
        \x20   Query(params): Query<FilterParams>,\n\
        ) -> Result<Json<PaginatedResponse<{}Response>>, AppError> {{\n\
        \x20   let parsed_filters = QueryValidator::validate_and_parse(&params, &[\"name\"])?;\n\
        \x20   let items = {}ModuleService::list_{}s(parsed_filters, &db).await?;\n\
        \x20   Ok(Json(items))\n\
         }}\n\n",
        slug, entity_name, entity_name, slug
    ));

    code.push_str(&format!(
        "pub async fn get_{}_handler(\n\
        \x20   State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,\n\
        \x20   Path(id): Path<String>,\n\
        ) -> Result<Json<{}Response>, AppError> {{\n\
        \x20   let item = {}ModuleService::get_{}_by_id(&id, &db).await?;\n\
        \x20   Ok(Json(item))\n\
         }}\n\n",
        slug, entity_name, entity_name, slug
    ));

    code.push_str(&format!(
        "pub async fn create_{}_handler(\n\
        \x20   State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,\n\
        \x20   AppJson(payload): AppJson<Create{}Request>,\n\
        ) -> Result<impl IntoResponse, AppError> {{\n\
        \x20   let created = {}ModuleService::create_{}(payload, &db).await?;\n\
        \x20   Ok((StatusCode::CREATED, Json(created)))\n\
         }}\n\n",
        slug, entity_name, entity_name, slug
    ));

    code.push_str(&format!(
        "pub async fn update_{}_handler(\n\
        \x20   State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,\n\
        \x20   Path(id): Path<String>,\n\
        \x20   AppJson(payload): AppJson<Update{}Request>,\n\
        ) -> Result<Json<{}Response>, AppError> {{\n\
        \x20   let updated = {}ModuleService::update_{}(&id, payload, &db).await?;\n\
        \x20   Ok(Json(updated))\n\
         }}\n\n",
        slug, entity_name, entity_name, entity_name, slug
    ));

    code.push_str(&format!(
        "pub async fn delete_{}_handler(\n\
        \x20   State((db, _, _)): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,\n\
        \x20   Path(id): Path<String>,\n\
        ) -> Result<impl IntoResponse, AppError> {{\n\
        \x20   {}ModuleService::delete_{}(&id, &db).await?;\n\
        \x20   Ok(StatusCode::NO_CONTENT)\n\
        }}\n",
        slug, entity_name, slug
    ));

    fs::write(path, code).expect("Falha ao salvar controller do módulo");
    println!("  📄 [NEW] src/modules/{}/controller.rs", slug);
}

fn generate_routes(_entity_name: &str, slug: &str) {
    let path = format!("src/modules/{}/routes.rs", slug);
    let mut code = String::new();

    code.push_str("use axum::{\n");
    code.push_str("    middleware::from_fn_with_state,\n");
    code.push_str("    routing::{get, post, put, delete},\n");
    code.push_str("    Router,\n");
    code.push_str("};\n");
    code.push_str("use sea_orm::DatabaseConnection;\n");
    code.push_str("use crate::{\n");
    code.push_str("    infra::cache::Cache,\n");
    code.push_str("    config::AppConfig,\n");
    code.push_str("    middleware::auth::auth_middleware,\n");
    code.push_str(&format!(
        "    modules::{}::controller::{{\n\
        \x20       create_{}_handler, delete_{}_handler, get_{}_handler, list_{}s_handler, update_{}_handler,\n\
        \x20   }},\n",
        slug, slug, slug, slug, slug, slug
    ));
    code.push_str("};\n\n");

    code.push_str(
        "pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {\n",
    );
    code.push_str("    let state = (db.clone(), cache.clone(), config.clone());\n\n");
    code.push_str("    let secure_routes = Router::new()\n");
    code.push_str(&format!(
        "        .route(\"/all\", get(list_{}s_handler))\n",
        slug
    ));
    code.push_str(&format!(
        "        .route(\"/\", post(create_{}_handler))\n",
        slug
    ));
    code.push_str(&format!(
        "        .route(\"/:id\", get(get_{}_handler))\n",
        slug
    ));
    code.push_str(&format!(
        "        .route(\"/:id\", put(update_{}_handler))\n",
        slug
    ));
    code.push_str(&format!(
        "        .route(\"/:id\", delete(delete_{}_handler))\n",
        slug
    ));
    code.push_str(
        "        .layer(from_fn_with_state((cache.clone(), config.clone()), auth_middleware))\n",
    );
    code.push_str("        .with_state(state);\n\n");
    code.push_str(&format!(
        "    Router::new().nest(\"/v1/{}\", secure_routes)\n",
        slug
    ));
    code.push_str("}\n");

    fs::write(path, code).expect("Falha ao salvar rotas do módulo");
    println!("  📄 [NEW] src/modules/{}/routes.rs", slug);
}

fn register_model(slug: &str) {
    let path = "src/models/mod.rs";
    let mut content = fs::read_to_string(path).unwrap_or_default();
    let mod_line = format!("pub mod {};\n", slug);

    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
        fs::write(path, content).expect("Falha ao atualizar src/models/mod.rs");
        println!("  📝 [EDIT] src/models/mod.rs (Registrado '{}')", slug);
    }
}

fn register_module(_entity_name: &str, slug: &str) {
    let path = "src/modules/mod.rs";
    let content = fs::read_to_string(path).unwrap_or_default();

    let mod_declaration = format!("pub mod {};", slug);
    if !content.contains(&mod_declaration) {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        lines.insert(0, mod_declaration);

        for i in 0..lines.len() {
            if lines[i].contains("Router::new()") {
                lines[i] = format!(
                    "{}\n        .merge({}::router(db.clone(), cache.clone(), config.clone()))",
                    lines[i], slug
                );
                break;
            }
        }

        let new_content = lines.join("\n");
        fs::write(path, new_content).expect("Falha ao atualizar src/modules/mod.rs");
        println!("  📝 [EDIT] src/modules/mod.rs (Registrado '{}')", slug);
    }
}
