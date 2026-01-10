// tests/health_test.rs - Health endpoint tests
mod common;

use common::TestApp;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = TestApp::get().await;

    let response = app
        .client()
        .get(&format!("{}/health", app.address))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: HealthResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.status, "healthy");
}

#[tokio::test]
async fn test_ready_endpoint() {
    let app = TestApp::get().await;

    let response = app
        .client()
        .get(&format!("{}/ready", app.address))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: HealthResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.status, "ready");
}
