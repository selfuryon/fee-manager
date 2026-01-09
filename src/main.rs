// main.rs
use config::AppConfig;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod addresses;
mod config;
mod errors;
mod handlers;
mod models;
mod openapi;
mod schema;

use crate::handlers::create_router;

#[derive(Debug)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
}

async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;

    tracing::info!("Migrations completed successfully");
    Ok(())
}

#[tokio::main]
async fn main() {
    // Load configuration
    let config = config::load_config().expect("Failed to load configuration");

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.log_level))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database.database_url())
        .await
        .expect("Failed to create pool");

    // Run migrations
    if let Err(e) = run_migrations(&pool).await {
        panic!("Error running migrations: {}", e);
    }

    // Create shared state
    let state = Arc::new(AppState {
        pool,
        config: config.clone(),
    });

    // Build our application with routes
    let app = create_router(state);

    // Run it
    let addr = &config.address();
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
