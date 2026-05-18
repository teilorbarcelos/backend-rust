pub mod config;
pub mod core;
pub mod errors;
pub mod infra;
pub mod middleware;
pub mod migration;
pub mod models;
pub mod modules;

use crate::{
    config::AppConfig,
    infra::{bootstrap::bootstrap_database, cache::Cache, database},
    migration::Migrator,
};
use axum::Router;
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // 1. Initialize sleek observability logger
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("🚀 Iniciando Mage Backend Boilerplate (Rust)...");

    // 2. Parse configurations
    let config = AppConfig::load();

    // 3. Connect to PostgreSQL
    let db = database::connect(&config.database_url)
        .await
        .expect("Falha ao se conectar com o banco de dados PostgreSQL");

    // 4. Run automatic migrations (Prisma/Gold standard approach)
    tracing::info!("🔄 Verificando e executando migrações pendentes...");
    Migrator::up(&db, None)
        .await
        .expect("Falha ao executar migrações do banco de dados");
    tracing::info!("✅ Migrações aplicadas com sucesso!");

    // 5. Seed initial data (Bootstrap Admin user / Roles / Permissions)
    bootstrap_database(&db)
        .await
        .expect("Falha ao executar rotina de bootstrap do banco de dados");

    // 6. Connect to Redis cache
    let cache = Cache::new(&config.redis_url);
    tracing::info!("✅ Conexão com Redis Cache estabelecida.");

    // 7. Assemble Axum Router
    let api_router = modules::app_router(db.clone(), cache.clone(), config.clone());
    let obs_router = modules::observability::router(db.clone(), cache.clone());

    // Merge routers and apply global rate limiter, audit, metrics and CORS middleware
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any);

    let app = Router::new()
        .merge(api_router)
        .merge(obs_router)
        .layer(axum::middleware::from_fn(
            modules::observability::track_metrics_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            db.clone(),
            middleware::audit::audit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            cache.clone(),
            middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(cors);

    // 8. Bind TCP listener and serve
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port as u16));
    tracing::info!(
        "⚡ Servidor rodando com sucesso no endereço http://{}",
        addr
    );
    tracing::info!(
        "📖 Documentação Swagger disponível em http://{}/v1/swagger",
        addr
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
