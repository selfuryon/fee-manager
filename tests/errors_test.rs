// tests/errors_test.rs - Every error response carries the JSON error body
mod common;

use common::TestApp;
use reqwest::Response;
use serde_json::{Value, json};

/// Asserts the status, a JSON content type and the error code; returns the message
async fn assert_error(response: Response, status: u16, code: &str) -> String {
    assert_eq!(response.status(), status);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.starts_with("application/json"),
        "content-type was {content_type:?}"
    );
    let body: Value = response.json().await.expect("JSON error body");
    assert_eq!(body["error"]["code"], code, "{body}");
    body["error"]["message"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_malformed_json() {
    let app = TestApp::get().await;
    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
        .header("content-type", "application/json")
        .body(r#"{"name":"#)
        .send()
        .await
        .unwrap();
    assert_error(response, 400, "INVALID_JSON").await;
}

#[tokio::test]
async fn test_wrong_field_type() {
    let app = TestApp::get().await;
    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({"name": "test_wrong_type", "active": "yes"}))
        .send()
        .await
        .unwrap();
    let message = assert_error(response, 400, "INVALID_DATA").await;
    assert!(message.contains("active"), "{message}");
}

#[tokio::test]
async fn test_invalid_key_on_public_endpoint() {
    let app = TestApp::get().await;
    let response = app
        .client_unauthenticated()
        .post(format!("{}/vouch/v2/execution-config/main", app.address))
        .json(&json!(["0x12"]))
        .send()
        .await
        .unwrap();
    assert_error(response, 400, "INVALID_DATA").await;
}

#[tokio::test]
async fn test_unparsable_query() {
    let app = TestApp::get().await;
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposers?limit=abc",
            app.address
        ))
        .send()
        .await
        .unwrap();
    assert_error(response, 400, "INVALID_QUERY").await;
}

#[tokio::test]
async fn test_invalid_path_parameter() {
    let app = TestApp::get().await;
    let response = app
        .client()
        .delete(format!("{}/api/admin/tokens/not-a-uuid", app.address))
        .send()
        .await
        .unwrap();
    assert_error(response, 400, "INVALID_DATA").await;
}

#[tokio::test]
async fn test_unknown_route() {
    let app = TestApp::get().await;
    for url in ["/api/admin/does-not-exist", "/does-not-exist"] {
        let response = app
            .client()
            .get(format!("{}{url}", app.address))
            .send()
            .await
            .unwrap();
        assert_error(response, 404, "NOT_FOUND").await;
    }
}

#[tokio::test]
async fn test_unsupported_method() {
    let app = TestApp::get().await;
    let response = app
        .client()
        .patch(format!("{}/api/admin/vouch/proposers", app.address))
        .send()
        .await
        .unwrap();
    assert_error(response, 405, "METHOD_NOT_ALLOWED").await;
}

#[tokio::test]
async fn test_missing_token() {
    let app = TestApp::get().await;
    let response = app
        .client_unauthenticated()
        .get(format!("{}/api/admin/vouch/proposers", app.address))
        .send()
        .await
        .unwrap();
    assert_error(response, 401, "UNAUTHORIZED").await;
}
