use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::time::Duration;

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(database_url.to_string());
    
    // Set pool configurations matching gold standards
    opt.max_connections(50)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false); // Can enable in debug if needed

    tracing::info!("Conectando ao banco de dados...");
    let db = Database::connect(opt).await?;
    tracing::info!("Conectado com sucesso!");
    
    Ok(db)
}
