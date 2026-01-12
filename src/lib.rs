// lib.rs - Library exports for testing
use sqlx::PgPool;

pub mod addresses;
pub mod audit;
pub mod auth;
pub mod config;
pub mod errors;
pub mod handlers;
pub mod models;
pub mod openapi;
pub mod schema;

pub use config::AppConfig;
pub use handlers::create_router;

#[derive(Debug)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!("Migrations completed successfully");
    Ok(())
}
