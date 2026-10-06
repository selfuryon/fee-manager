// main.rs
use fee_manager::{AppState, config, create_router, run_migrations};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Load configuration
    let config = config::load_config().expect("Failed to load configuration");

    // Initialize tracing with conditional JSON format
    let subscriber =
        tracing_subscriber::registry().with(tracing_subscriber::EnvFilter::new(&config.log_level));

    if config.log_format == "json" {
        subscriber
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        subscriber.with(tracing_subscriber::fmt::layer()).init();
    }

    // Initialize audit writer if audit is enabled
    if config.audit_enabled {
        fee_manager::audit::init_audit_writer(&config.audit_output);
    }

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

    // Generate initial API token if none exist
    match fee_manager::auth::service::ensure_default_token(&pool).await {
        Ok(Some(token)) => {
            tracing::warn!("===========================================");
            tracing::warn!("GENERATED INITIAL API TOKEN: {}", token);
            tracing::warn!("Save this token! It will not be shown again.");
            tracing::warn!("===========================================");
        }
        Ok(None) => {
            // Token already exists, nothing to do
        }
        Err(e) => {
            tracing::error!("Failed to ensure default token: {}", e);
        }
    }

    // Kept to close the pool once the server has drained
    let pool_handle = pool.clone();

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
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    pool_handle.close().await;
    tracing::info!("Shutdown complete");
}

/// Resolves on SIGTERM (Kubernetes, `podman stop`) or Ctrl-C. In the container
/// the binary is PID 1, so without this handler SIGTERM is ignored and every
/// stop ends in SIGKILL after the grace period.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl-C handler");
    };
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Shutdown signal received, draining connections");
}
