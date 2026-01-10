// tests/mux_test.rs - Commit-Boost Mux CRUD and key management tests
mod common;

use common::TestApp;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MuxConfigResponse {
    name: String,
    keys: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MuxConfigListItem {
    name: String,
    key_count: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MuxKeysResponse {
    added: Option<i64>,
    removed: Option<i64>,
    total_keys: i64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    total: i64,
    limit: i64,
    offset: i64,
}

/// Helper to create unique mux name
fn unique_mux_name(prefix: &str) -> String {
    format!("test_mux_{}_{}", prefix, TestApp::unique_id())
}

/// Helper to delete a mux config
async fn delete_mux(app: &TestApp, name: &str) {
    let _ = app.client()
        .delete(&format!("{}/api/admin/commit-boost/mux/{}", app.address, name))
        .send()
        .await;
}

// ============================================================================
// CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_create_mux_config() {
    let app = TestApp::get().await;
    let name = unique_mux_name("create");

    let response = app
        .client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name,
            "keys": []
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 201);

    let body: MuxConfigListItem = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert_eq!(body.key_count, 0);

    delete_mux(app, &name).await;
}

#[tokio::test]
async fn test_create_mux_config_with_keys() {
    let app = TestApp::get().await;
    let name = unique_mux_name("keys");
    let id = TestApp::unique_id();
    let key1 = TestApp::test_bls_pubkey(&format!("k1{}", id));
    let key2 = TestApp::test_bls_pubkey(&format!("k2{}", id));

    let response = app
        .client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name,
            "keys": [key1, key2]
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 201);

    let body: MuxConfigListItem = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert_eq!(body.key_count, 2);

    delete_mux(app, &name).await;
}

#[tokio::test]
async fn test_create_mux_config_duplicate() {
    let app = TestApp::get().await;
    let name = unique_mux_name("dup");

    // Create first config
    app.client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Try to create duplicate
    let response = app
        .client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);

    delete_mux(app, &name).await;
}

#[tokio::test]
async fn test_get_mux_config() {
    let app = TestApp::get().await;
    let name = unique_mux_name("get");
    let id = TestApp::unique_id();
    let key1 = TestApp::test_bls_pubkey(&format!("g1{}", id));
    let key2 = TestApp::test_bls_pubkey(&format!("g2{}", id));

    // Create config with keys
    app.client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name,
            "keys": [key1, key2]
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Get config
    let response = app
        .client()
        .get(&format!("{}/api/admin/commit-boost/mux/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: MuxConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert_eq!(body.keys.len(), 2);

    delete_mux(app, &name).await;
}

#[tokio::test]
async fn test_get_mux_config_not_found() {
    let app = TestApp::get().await;
    let name = unique_mux_name("notfound");

    let response = app
        .client()
        .get(&format!("{}/api/admin/commit-boost/mux/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_update_mux_config() {
    let app = TestApp::get().await;
    let name = unique_mux_name("update");
    let id = TestApp::unique_id();
    let key1 = TestApp::test_bls_pubkey(&format!("u1{}", id));
    let key2 = TestApp::test_bls_pubkey(&format!("u2{}", id));
    let key3 = TestApp::test_bls_pubkey(&format!("u3{}", id));

    // Create config
    app.client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name,
            "keys": [key1.clone()]
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Update config (replace keys)
    let response = app
        .client()
        .put(&format!("{}/api/admin/commit-boost/mux/{}", app.address, name))
        .json(&json!({
            "keys": [key2.clone(), key3.clone()]
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: MuxConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.keys.len(), 2);
    assert!(!body.keys.contains(&key1));
    assert!(body.keys.contains(&key2));
    assert!(body.keys.contains(&key3));

    delete_mux(app, &name).await;
}

#[tokio::test]
async fn test_delete_mux_config() {
    let app = TestApp::get().await;
    let name = unique_mux_name("delete");

    // Create config
    app.client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Delete config
    let response = app
        .client()
        .delete(&format!("{}/api/admin/commit-boost/mux/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 204);

    // Verify deleted
    let response = app
        .client()
        .get(&format!("{}/api/admin/commit-boost/mux/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

// ============================================================================
// Keys Management Tests
// ============================================================================

#[tokio::test]
async fn test_add_mux_keys() {
    let app = TestApp::get().await;
    let name = unique_mux_name("addkeys");
    let id = TestApp::unique_id();

    // Create config
    app.client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Add keys
    let key1 = TestApp::test_bls_pubkey(&format!("a1{}", id));
    let key2 = TestApp::test_bls_pubkey(&format!("a2{}", id));

    let response = app
        .client()
        .post(&format!("{}/api/admin/commit-boost/mux/{}/keys", app.address, name))
        .json(&json!({
            "keys": [key1.clone(), key2]
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: MuxKeysResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.added, Some(2));
    assert_eq!(body.total_keys, 2);

    // Add more keys (including duplicates)
    let key3 = TestApp::test_bls_pubkey(&format!("a3{}", id));

    let response = app
        .client()
        .post(&format!("{}/api/admin/commit-boost/mux/{}/keys", app.address, name))
        .json(&json!({
            "keys": [key1, key3]  // key1 is duplicate
        }))
        .send()
        .await
        .expect("Failed to send request");

    let body: MuxKeysResponse = response.json().await.expect("Failed to parse JSON");
    // Only key3 should be added (key1 is duplicate)
    assert_eq!(body.added, Some(1));
    assert_eq!(body.total_keys, 3);

    delete_mux(app, &name).await;
}

#[tokio::test]
async fn test_remove_mux_keys() {
    let app = TestApp::get().await;
    let name = unique_mux_name("remkeys");
    let id = TestApp::unique_id();

    let key1 = TestApp::test_bls_pubkey(&format!("r1{}", id));
    let key2 = TestApp::test_bls_pubkey(&format!("r2{}", id));
    let key3 = TestApp::test_bls_pubkey(&format!("r3{}", id));

    // Create config with keys
    app.client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name,
            "keys": [key1.clone(), key2.clone(), key3.clone()]
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Remove some keys
    let response = app
        .client()
        .delete(&format!("{}/api/admin/commit-boost/mux/{}/keys", app.address, name))
        .json(&json!({
            "keys": [key1, key2]
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: MuxKeysResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.removed, Some(2));
    assert_eq!(body.total_keys, 1);

    // Verify remaining key
    let response = app
        .client()
        .get(&format!("{}/api/admin/commit-boost/mux/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    let body: MuxConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.keys.len(), 1);
    assert!(body.keys.contains(&key3));

    delete_mux(app, &name).await;
}

// ============================================================================
// Public Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_get_mux_keys_public() {
    let app = TestApp::get().await;
    let name = unique_mux_name("public");
    let id = TestApp::unique_id();

    let key1 = TestApp::test_bls_pubkey(&format!("p1{}", id));
    let key2 = TestApp::test_bls_pubkey(&format!("p2{}", id));

    // Create config with keys
    app.client()
        .post(&format!("{}/api/admin/commit-boost/mux", app.address))
        .json(&json!({
            "name": name,
            "keys": [key1.clone(), key2.clone()]
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Get keys via public endpoint
    let response = app
        .client()
        .get(&format!("{}/commit-boost/v1/mux/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: Vec<String> = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.len(), 2);
    assert!(body.contains(&key1));
    assert!(body.contains(&key2));

    delete_mux(app, &name).await;
}

#[tokio::test]
async fn test_get_mux_keys_public_not_found() {
    let app = TestApp::get().await;
    let name = unique_mux_name("pubnotfound");

    let response = app
        .client()
        .get(&format!("{}/commit-boost/v1/mux/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

// ============================================================================
// List Tests
// ============================================================================

#[tokio::test]
async fn test_list_mux_configs() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create multiple configs with unique prefix
    let names: Vec<String> = (1..=3).map(|i| format!("test_mux_list_{}_{}", id, i)).collect();
    for name in &names {
        app.client()
            .post(&format!("{}/api/admin/commit-boost/mux", app.address))
            .json(&json!({
                "name": name
            }))
            .send()
            .await
            .expect("Failed to create config");
    }

    // List all configs
    let response = app
        .client()
        .get(&format!("{}/api/admin/commit-boost/mux", app.address))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: PaginatedResponse<MuxConfigListItem> = response.json().await.expect("Failed to parse JSON");
    let test_configs: Vec<_> = body.data.iter().filter(|c| c.name.starts_with(&format!("test_mux_list_{}", id))).collect();
    assert_eq!(test_configs.len(), 3);

    // Cleanup
    for name in &names {
        delete_mux(app, name).await;
    }
}

#[tokio::test]
async fn test_mux_pagination() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create 5 configs
    let names: Vec<String> = (1..=5).map(|i| format!("test_mux_page_{}_{}", id, i)).collect();
    for name in &names {
        app.client()
            .post(&format!("{}/api/admin/commit-boost/mux", app.address))
            .json(&json!({
                "name": name
            }))
            .send()
            .await
            .expect("Failed to create config");
    }

    // Test limit
    let response = app
        .client()
        .get(&format!("{}/api/admin/commit-boost/mux?limit=2", app.address))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<MuxConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert_eq!(body.limit, 2);

    // Cleanup
    for name in &names {
        delete_mux(app, name).await;
    }
}
