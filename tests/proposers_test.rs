// tests/proposers_test.rs - Proposer CRUD and filtering tests
mod common;

use common::TestApp;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct ProposerRelayConfig {
    public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_value: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ProposerResponse {
    public_key: String,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    min_value: Option<String>,
    reset_relays: bool,
    relays: Option<HashMap<String, ProposerRelayConfig>>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ProposerListItem {
    public_key: String,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    min_value: Option<String>,
    reset_relays: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    total: i64,
    limit: i64,
    offset: i64,
}

/// Helper to delete a proposer
async fn delete_proposer(app: &TestApp, pubkey: &str) {
    let _ = app.client()
        .delete(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .send()
        .await;
}

// ============================================================================
// CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_create_proposer() {
    let app = TestApp::get().await;
    let pubkey = TestApp::test_bls_pubkey(&format!("cr{}", TestApp::unique_id()));

    let response = app
        .client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .json(&json!({
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "min_value": "10000000000000000",
            "reset_relays": false
        }))
        .send()
        .await
        .expect("Failed to send request");

    // PUT creates with 201 or updates with 200
    assert!(response.status() == 200 || response.status() == 201);

    let body: ProposerResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.public_key, pubkey);
    assert_eq!(body.fee_recipient, Some("0x1234567890abcdef1234567890abcdef12345678".to_string()));
    assert_eq!(body.gas_limit, Some("30000000".to_string()));
    assert!(!body.reset_relays);

    delete_proposer(app, &pubkey).await;
}

#[tokio::test]
async fn test_create_proposer_with_relays() {
    let app = TestApp::get().await;
    let pubkey = TestApp::test_bls_pubkey(&format!("rl{}", TestApp::unique_id()));

    let response = app
        .client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .json(&json!({
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "reset_relays": true,
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f",
                    "disabled": false
                },
                "https://relay2.example.com": {
                    "public_key": "0xb0b07cd0abef743db4260b0ed50619cf6ad4d82064cb4fbec9d3ec530f7c5e6793d9f286c4e082c0244ffb9f2658fe88",
                    "disabled": true
                }
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status() == 200 || response.status() == 201);

    let body: ProposerResponse = response.json().await.expect("Failed to parse JSON");
    assert!(body.reset_relays);
    assert!(body.relays.is_some());
    assert_eq!(body.relays.as_ref().unwrap().len(), 2);

    delete_proposer(app, &pubkey).await;
}

#[tokio::test]
async fn test_get_proposer() {
    let app = TestApp::get().await;
    let pubkey = TestApp::test_bls_pubkey(&format!("gt{}", TestApp::unique_id()));

    // Create proposer
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .json(&json!({
            "gas_limit": "32000000"
        }))
        .send()
        .await
        .expect("Failed to create proposer");

    // Get proposer
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ProposerResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.public_key, pubkey);
    assert_eq!(body.gas_limit, Some("32000000".to_string()));

    delete_proposer(app, &pubkey).await;
}

#[tokio::test]
async fn test_get_proposer_not_found() {
    let app = TestApp::get().await;
    let pubkey = TestApp::test_bls_pubkey(&format!("nf{}", TestApp::unique_id()));

    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_update_proposer() {
    let app = TestApp::get().await;
    let pubkey = TestApp::test_bls_pubkey(&format!("up{}", TestApp::unique_id()));

    // Create proposer
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .json(&json!({
            "gas_limit": "30000000",
            "reset_relays": false
        }))
        .send()
        .await
        .expect("Failed to create proposer");

    // Update proposer
    let response = app
        .client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .json(&json!({
            "gas_limit": "35000000",
            "reset_relays": true
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ProposerResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.gas_limit, Some("35000000".to_string()));
    assert!(body.reset_relays);

    delete_proposer(app, &pubkey).await;
}

#[tokio::test]
async fn test_delete_proposer() {
    let app = TestApp::get().await;
    let pubkey = TestApp::test_bls_pubkey(&format!("dl{}", TestApp::unique_id()));

    // Create proposer
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to create proposer");

    // Delete proposer
    let response = app
        .client()
        .delete(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 204);

    // Verify deleted
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

// ============================================================================
// Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_list_proposers() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create multiple proposers - using hex-only prefix
    let prefix = format!("aa{}", id);  // aa + hex id
    let pubkeys: Vec<String> = (1..=3).map(|i| TestApp::test_bls_pubkey(&format!("{}0{}", prefix, i))).collect();
    for (i, pubkey) in pubkeys.iter().enumerate() {
        app.client()
            .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
            .json(&json!({
                "gas_limit": format!("{}0000000", 30 + i),
                "reset_relays": i % 2 == 0
            }))
            .send()
            .await
            .expect("Failed to create proposer");
    }

    // List all test proposers - use hex prefix for filtering
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers?public_key=0xdead{}", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: PaginatedResponse<ProposerListItem> = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.data.len(), 3);

    // Cleanup
    for pubkey in &pubkeys {
        delete_proposer(app, pubkey).await;
    }
}

#[tokio::test]
async fn test_filter_by_reset_relays() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create proposers with different reset_relays values - hex prefix
    let prefix = format!("bb{}", id);
    let pubkeys: Vec<String> = (1..=4).map(|i| TestApp::test_bls_pubkey(&format!("{}0{}", prefix, i))).collect();
    for (i, pubkey) in pubkeys.iter().enumerate() {
        app.client()
            .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
            .json(&json!({
                "reset_relays": (i + 1) % 2 == 0  // 2,4 = true
            }))
            .send()
            .await
            .expect("Failed to create proposer");
    }

    // Filter reset_relays = true
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers?public_key=0xdead{}&reset_relays=true", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert!(body.data.iter().all(|p| p.reset_relays));

    // Filter reset_relays = false
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers?public_key=0xdead{}&reset_relays=false", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert!(body.data.iter().all(|p| !p.reset_relays));

    // Cleanup
    for pubkey in &pubkeys {
        delete_proposer(app, pubkey).await;
    }
}

#[tokio::test]
async fn test_filter_by_public_key_prefix() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create proposers with different prefixes - using hex prefixes
    let pubkey1 = TestApp::test_bls_pubkey(&format!("cc1{}", id));
    let pubkey2 = TestApp::test_bls_pubkey(&format!("cc2{}", id));
    let pubkey3 = TestApp::test_bls_pubkey(&format!("dd{}", id));

    for pubkey in [&pubkey1, &pubkey2, &pubkey3] {
        app.client()
            .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
            .json(&json!({}))
            .send()
            .await
            .expect("Failed to create proposer");
    }

    // Filter by prefix "cc" - should match cc1 and cc2
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers?public_key=0xdeadcc", app.address))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    // Filter matches both cc1 and cc2
    let matching: Vec<_> = body.data.iter().filter(|p| p.public_key.contains(&format!("cc1{}", id)) || p.public_key.contains(&format!("cc2{}", id))).collect();
    assert_eq!(matching.len(), 2);

    // Cleanup
    delete_proposer(app, &pubkey1).await;
    delete_proposer(app, &pubkey2).await;
    delete_proposer(app, &pubkey3).await;
}

#[tokio::test]
async fn test_proposers_pagination() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create 5 proposers - hex prefix for filtering
    let prefix = format!("ee{}", id);
    let pubkeys: Vec<String> = (1..=5).map(|i| TestApp::test_bls_pubkey(&format!("{}0{}", prefix, i))).collect();
    for pubkey in &pubkeys {
        app.client()
            .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
            .json(&json!({}))
            .send()
            .await
            .expect("Failed to create proposer");
    }

    // Test limit
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers?public_key=0xdead{}&limit=2", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert_eq!(body.limit, 2);
    assert_eq!(body.total, 5);

    // Test offset
    let response = app
        .client()
        .get(&format!("{}/api/admin/vouch/proposers?public_key=0xdead{}&limit=2&offset=3", app.address, prefix))
        .send()
        .await
        .expect("Failed to send request");

    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 2);
    assert_eq!(body.offset, 3);

    // Cleanup
    for pubkey in &pubkeys {
        delete_proposer(app, pubkey).await;
    }
}

// ============================================================================
// Relay Filter Tests
// ============================================================================

#[tokio::test]
async fn test_filter_proposers_by_relay_url() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let prefix = format!("ff{}", id);

    let pubkey_with_relay = TestApp::test_bls_pubkey(&format!("{}01", prefix));
    let pubkey_without_relay = TestApp::test_bls_pubkey(&format!("{}02", prefix));

    // Create proposer with relay
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey_with_relay))
        .json(&json!({
            "relays": {
                "https://flashbots.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create proposer with relay");

    // Create proposer without relay
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey_without_relay))
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to create proposer without relay");

    // Filter by relay_url prefix
    let response = app
        .client()
        .get(&format!(
            "{}/api/admin/vouch/proposers?public_key=0xdead{}&relay_url=https://flashbots",
            app.address, prefix
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 1);
    assert!(body.data[0].public_key.contains("01"));

    // Cleanup
    delete_proposer(app, &pubkey_with_relay).await;
    delete_proposer(app, &pubkey_without_relay).await;
}

#[tokio::test]
async fn test_filter_proposers_by_relay_min_value() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let prefix = format!("a0{}", id);

    let pubkey_with_min = TestApp::test_bls_pubkey(&format!("{}01", prefix));
    let pubkey_without_min = TestApp::test_bls_pubkey(&format!("{}02", prefix));

    // Create proposer with relay that has min_value
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey_with_min))
        .json(&json!({
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f",
                    "min_value": "99000000000000000"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create proposer with relay min_value");

    // Create proposer with relay without min_value
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey_without_min))
        .json(&json!({
            "relays": {
                "https://relay2.example.com": {
                    "public_key": "0xac6e77dfe25ecd6110b8e780608cce0dab71fdd5ebea22a16c0205200f2f8e2e3ad3b71d3499c54ad14d6c21b41a37ae"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create proposer without relay min_value");

    // Filter by relay_min_value
    let response = app
        .client()
        .get(&format!(
            "{}/api/admin/vouch/proposers?public_key=0xdead{}&relay_min_value=99000000000000000",
            app.address, prefix
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 1);
    assert!(body.data[0].public_key.contains("01"));

    // Cleanup
    delete_proposer(app, &pubkey_with_min).await;
    delete_proposer(app, &pubkey_without_min).await;
}

#[tokio::test]
async fn test_filter_proposers_by_relay_disabled() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let prefix = format!("b1{}", id);

    let pubkey_disabled = TestApp::test_bls_pubkey(&format!("{}01", prefix));
    let pubkey_enabled = TestApp::test_bls_pubkey(&format!("{}02", prefix));

    // Create proposer with disabled relay
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey_disabled))
        .json(&json!({
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f",
                    "disabled": true
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create proposer with disabled relay");

    // Create proposer with enabled relay
    app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey_enabled))
        .json(&json!({
            "relays": {
                "https://relay2.example.com": {
                    "public_key": "0xac6e77dfe25ecd6110b8e780608cce0dab71fdd5ebea22a16c0205200f2f8e2e3ad3b71d3499c54ad14d6c21b41a37ae",
                    "disabled": false
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create proposer with enabled relay");

    // Filter by relay_disabled=true
    let response = app
        .client()
        .get(&format!(
            "{}/api/admin/vouch/proposers?public_key=0xdead{}&relay_disabled=true",
            app.address, prefix
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 1);
    assert!(body.data[0].public_key.contains("01"));

    // Filter by relay_disabled=false
    let response = app
        .client()
        .get(&format!(
            "{}/api/admin/vouch/proposers?public_key=0xdead{}&relay_disabled=false",
            app.address, prefix
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: PaginatedResponse<ProposerListItem> = response.json().await.unwrap();
    assert_eq!(body.data.len(), 1);
    assert!(body.data[0].public_key.contains("02"));

    // Cleanup
    delete_proposer(app, &pubkey_disabled).await;
    delete_proposer(app, &pubkey_enabled).await;
}
