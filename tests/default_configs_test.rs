// tests/default_configs_test.rs - Default Config CRUD and filtering tests
mod common;

use common::TestApp;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct RelayConfig {
    public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DefaultConfigResponse {
    name: String,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    #[allow(dead_code)]
    min_value: Option<String>,
    active: bool,
    relays: Option<HashMap<String, RelayConfig>>,
    #[allow(dead_code)]
    created_at: String,
    #[allow(dead_code)]
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct DefaultConfigListItem {
    name: String,
    #[allow(dead_code)]
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    #[allow(dead_code)]
    min_value: Option<String>,
    active: bool,
    #[allow(dead_code)]
    created_at: String,
    #[allow(dead_code)]
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    total: i64,
    limit: i64,
    offset: i64,
}

/// Helper to create a unique test config name
fn unique_config_name(prefix: &str) -> String {
    format!("test_{}_{}", prefix, TestApp::unique_id())
}

/// Helper to delete a config (cleanup)
async fn delete_config(app: &TestApp, name: &str) {
    let _ = app.client()
        .delete(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .send()
        .await;
}

// ============================================================================
// CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_create_default_config() {
    let app = TestApp::get().await;
    let name = unique_config_name("create");

    let response = app
        .client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "min_value": "10000000000000000",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 201);

    let body: DefaultConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert_eq!(body.fee_recipient, Some("0x1234567890abcdef1234567890abcdef12345678".to_string()));
    assert_eq!(body.gas_limit, Some("30000000".to_string()));
    assert!(body.active);

    delete_config(app, &name).await;
}

#[tokio::test]
async fn test_create_default_config_with_relays() {
    let app = TestApp::get().await;
    let name = unique_config_name("relays");

    let response = app
        .client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "active": true,
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f",
                    "fee_recipient": "0xabcdef1234567890abcdef1234567890abcdef12"
                },
                "https://relay2.example.com": {
                    "public_key": "0xb0b07cd0abef743db4260b0ed50619cf6ad4d82064cb4fbec9d3ec530f7c5e6793d9f286c4e082c0244ffb9f2658fe88"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 201);

    // Fetch the config to verify relays
    let get_response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .send()
        .await
        .expect("Failed to get config");

    let body: DefaultConfigResponse = get_response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert!(body.relays.is_some(), "Relays should be present");
    assert_eq!(body.relays.as_ref().unwrap().len(), 2);

    delete_config(app, &name).await;
}

#[tokio::test]
async fn test_create_default_config_duplicate() {
    let app = TestApp::get().await;
    let name = unique_config_name("dup");

    // Create first config
    let _ = app
        .client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "active": true
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Try to create duplicate
    let response = app
        .client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "active": true
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);

    delete_config(app, &name).await;
}

#[tokio::test]
async fn test_get_default_config() {
    let app = TestApp::get().await;
    let name = unique_config_name("get");

    // Create config
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "gas_limit": "32000000",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Get config
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: DefaultConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert_eq!(body.gas_limit, Some("32000000".to_string()));

    delete_config(app, &name).await;
}

#[tokio::test]
async fn test_get_default_config_not_found() {
    let app = TestApp::get().await;
    let name = unique_config_name("notfound");

    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_update_default_config() {
    let app = TestApp::get().await;
    let name = unique_config_name("update");

    // Create config
    let create_resp = app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "gas_limit": "30000000",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    assert_eq!(create_resp.status(), 201, "Failed to create config for update test");

    // Update config
    let response = app
        .client()
        .put(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .json(&json!({
            "gas_limit": "35000000",
            "active": false
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: DefaultConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.gas_limit, Some("35000000".to_string()));
    assert!(!body.active);

    delete_config(app, &name).await;
}

#[tokio::test]
async fn test_delete_default_config() {
    let app = TestApp::get().await;
    let name = unique_config_name("delete");

    // Create config
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Delete config
    let response = app
        .client()
        .delete(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 204);

    // Verify deleted
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

// ============================================================================
// Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_list_default_configs() {
    let app = TestApp::get().await;
    let prefix = TestApp::unique_id();
    let names: Vec<String> = (1..=3).map(|i| format!("test_list_{}_{}", prefix, i)).collect();

    // Create multiple configs
    for (i, name) in names.iter().enumerate() {
        let resp = app.client()
            .post(&format!("{}/api/admin/vouch/configs/default", app.address))
            .json(&json!({
                "name": name,
                "gas_limit": format!("{}0000000", 30 + i),
                "active": i % 2 == 0
            }))
            .send()
            .await
            .expect("Failed to create config");
        assert_eq!(resp.status(), 201, "Failed to create {}", name);
    }

    // List all test configs
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default?name=test_list_{}", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.data.len(), 3);

    // Cleanup
    for name in &names {
        delete_config(app, name).await;
    }
}

#[tokio::test]
async fn test_filter_by_active() {
    let app = TestApp::get().await;
    let prefix = TestApp::unique_id();
    let names: Vec<String> = (1..=3).map(|i| format!("test_active_{}_{}", prefix, i)).collect();

    // Create configs with different active states
    for (i, name) in names.iter().enumerate() {
        app.client()
            .post(&format!("{}/api/admin/vouch/configs/default", app.address))
            .json(&json!({ "name": name, "active": i != 1 }))  // 0,2 = true, 1 = false
            .send()
            .await
            .unwrap();
    }

    // Filter active only
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default?name=test_active_{}&active=true", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert!(body.data.iter().all(|c| c.active));

    // Filter inactive only
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default?name=test_active_{}&active=false", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 1);
    assert!(body.data.iter().all(|c| !c.active));

    // Cleanup
    for name in &names {
        delete_config(app, name).await;
    }
}

#[tokio::test]
async fn test_filter_by_gas_limit() {
    let app = TestApp::get().await;
    let prefix = TestApp::unique_id();
    let names: Vec<String> = (1..=3).map(|i| format!("test_gas_{}_{}", prefix, i)).collect();

    // Create configs with different gas limits
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({ "name": &names[0], "gas_limit": "30000000", "active": true }))
        .send()
        .await
        .unwrap();

    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({ "name": &names[1], "gas_limit": "35000000", "active": true }))
        .send()
        .await
        .unwrap();

    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({ "name": &names[2], "gas_limit": "30000000", "active": true }))
        .send()
        .await
        .unwrap();

    // Filter by gas_limit
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default?name=test_gas_{}&gas_limit=30000000", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert!(body.data.iter().all(|c| c.gas_limit == Some("30000000".to_string())));

    // Cleanup
    for name in &names {
        delete_config(app, name).await;
    }
}

#[tokio::test]
async fn test_pagination() {
    let app = TestApp::get().await;
    let prefix = TestApp::unique_id();
    let names: Vec<String> = (1..=5).map(|i| format!("test_page_{}_{}", prefix, i)).collect();

    // Create 5 configs
    for name in &names {
        app.client()
            .post(&format!("{}/api/admin/vouch/configs/default", app.address))
            .json(&json!({ "name": name, "active": true }))
            .send()
            .await
            .unwrap();
    }

    // Test limit
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default?name=test_page_{}&limit=2", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert_eq!(body.limit, 2);
    assert_eq!(body.total, 5);

    // Test offset
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/configs/default?name=test_page_{}&limit=2&offset=2", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert_eq!(body.offset, 2);

    // Cleanup
    for name in &names {
        delete_config(app, name).await;
    }
}

// ============================================================================
// Relay Filter Tests
// ============================================================================

#[tokio::test]
async fn test_filter_by_relay_url() {
    let app = TestApp::get().await;
    let prefix = TestApp::unique_id();
    let name_with_relay = format!("test_relay_url_{}_with", prefix);
    let name_without_relay = format!("test_relay_url_{}_without", prefix);

    // Create config with relay
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name_with_relay,
            "active": true,
            "relays": {
                "https://flashbots.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create config with relay");

    // Create config without relay
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name_without_relay,
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config without relay");

    // Filter by relay_url prefix
    let response = app
        .client()
        .get(&format!(
            "{}/api/admin/vouch/configs/default?name=test_relay_url_{}&relay_url=https://flashbots",
            app.address, prefix
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 1);
    assert!(body.data[0].name.contains("with"));

    // Cleanup
    delete_config(app, &name_with_relay).await;
    delete_config(app, &name_without_relay).await;
}

#[tokio::test]
async fn test_filter_by_relay_min_value() {
    let app = TestApp::get().await;
    let prefix = TestApp::unique_id();
    let name_with_min = format!("test_relay_min_{}_with", prefix);
    let name_without_min = format!("test_relay_min_{}_without", prefix);

    // Create config with relay that has min_value
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name_with_min,
            "active": true,
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f",
                    "min_value": "50000000000000000"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create config with relay min_value");

    // Create config with relay without min_value
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name_without_min,
            "active": true,
            "relays": {
                "https://relay2.example.com": {
                    "public_key": "0xac6e77dfe25ecd6110b8e780608cce0dab71fdd5ebea22a16c0205200f2f8e2e3ad3b71d3499c54ad14d6c21b41a37ae"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create config without relay min_value");

    // Filter by relay_min_value
    let response = app
        .client()
        .get(&format!(
            "{}/api/admin/vouch/configs/default?name=test_relay_min_{}&relay_min_value=50000000000000000",
            app.address, prefix
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: PaginatedResponse<DefaultConfigListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 1);
    assert!(body.data[0].name.contains("with"));

    // Cleanup
    delete_config(app, &name_with_min).await;
    delete_config(app, &name_without_min).await;
}
