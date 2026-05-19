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
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("🚀 Iniciando Mage Backend Boilerplate (Rust)...");

    let config = AppConfig::load();

    let db = database::connect(&config.database_url)
        .await
        .expect("Falha ao se conectar com o banco de dados PostgreSQL");

    tracing::info!("🔄 Verificando e executando migrações pendentes...");
    Migrator::up(&db, None)
        .await
        .expect("Falha ao executar migrações do banco de dados");
    tracing::info!("✅ Migrações aplicadas com sucesso!");

    bootstrap_database(&db)
        .await
        .expect("Falha ao executar rotina de bootstrap do banco de dados");

    let cache = Cache::new(&config.redis_url);
    tracing::info!("✅ Conexão com Redis Cache estabelecida.");

    let api_router = modules::app_router(db.clone(), cache.clone(), config.clone());
    let obs_router = modules::observability::router(db.clone(), cache.clone());

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
            middleware::error_log::error_logging_middleware,
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

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(
        "⚡ Servidor rodando com sucesso no endereço http://{}",
        addr
    );
    tracing::info!(
        "📖 Documentação Swagger disponível em http://{}/v1/docs",
        addr
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
