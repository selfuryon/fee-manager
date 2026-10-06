// tests/probes_test.rs - Readiness and liveness with the database unreachable
use fee_manager::{AppState, config, create_router};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Serves the app against a pool pointing at a closed port and returns its address
async fn spawn_without_database() -> String {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    let state = Arc::new(AppState {
        pool,
        config: config::load_config().expect("config"),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, create_router(state)).await });
    address
}

#[tokio::test]
async fn test_ready_fails_without_database() {
    let address = spawn_without_database().await;
    let started = Instant::now();
    let response = reqwest::get(format!("{address}/ready")).await.unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(response.status(), 503);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");
}

#[tokio::test]
async fn test_health_ignores_database() {
    let address = spawn_without_database().await;
    let response = reqwest::get(format!("{address}/health")).await.unwrap();
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
}
