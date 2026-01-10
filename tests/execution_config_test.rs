// tests/execution_config_test.rs - Vouch execution config public endpoint tests
mod common;

use common::TestApp;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RelayConfig {
    public_key: String,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    min_value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ProposerEntry {
    proposer: String,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    min_value: Option<String>,
    reset_relays: Option<bool>,
    relays: Option<HashMap<String, RelayConfig>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ExecutionConfigResponse {
    version: u8,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    min_value: Option<String>,
    relays: Option<HashMap<String, RelayConfig>>,
    proposers: Option<Vec<ProposerEntry>>,
}

/// Helper to create unique config name for this test
fn unique_config_name(prefix: &str) -> String {
    format!("test_{}_{}", prefix, TestApp::unique_id())
}

/// Helper to delete a config
async fn delete_config(app: &TestApp, name: &str) {
    let _ = app.client()
        .delete(&format!("{}/api/admin/vouch/configs/default/{}", app.address, name))
        .send()
        .await;
}

/// Helper to delete a proposer pattern
async fn delete_pattern(app: &TestApp, name: &str) {
    let _ = app.client()
        .delete(&format!("{}/api/admin/vouch/proposer-patterns/{}", app.address, name))
        .send()
        .await;
}

/// Helper to delete a proposer
async fn delete_proposer(app: &TestApp, pubkey: &str) {
    let _ = app.client()
        .delete(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .send()
        .await;
}

// ============================================================================
// Basic Tests
// ============================================================================

#[tokio::test]
async fn test_get_execution_config_basic() {
    let app = TestApp::get().await;
    let config_name = unique_config_name("exec_basic");

    // Create a default config
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": config_name,
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "min_value": "10000000000000000",
            "active": true,
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Get execution config
    let response = app
        .client()
        .post(&format!("{}/vouch/v2/execution-config/{}", app.address, config_name))
        .json(&json!([]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.version, 2);
    assert_eq!(body.fee_recipient, Some("0x1234567890abcdef1234567890abcdef12345678".to_string()));
    assert_eq!(body.gas_limit, Some("30000000".to_string()));
    assert!(body.relays.is_some());

    delete_config(app, &config_name).await;
}

#[tokio::test]
async fn test_get_execution_config_not_found() {
    let app = TestApp::get().await;
    let config_name = unique_config_name("nonexistent");

    let response = app
        .client()
        .post(&format!("{}/vouch/v2/execution-config/{}", app.address, config_name))
        .json(&json!([]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_get_execution_config_inactive() {
    let app = TestApp::get().await;
    let config_name = unique_config_name("exec_inactive");

    // Create an inactive default config
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": config_name,
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "active": false
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Should return 404 for inactive config
    let response = app
        .client()
        .post(&format!("{}/vouch/v2/execution-config/{}", app.address, config_name))
        .json(&json!([]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);

    delete_config(app, &config_name).await;
}

// ============================================================================
// Proposer-specific Config Tests
// ============================================================================

#[tokio::test]
async fn test_get_execution_config_with_proposer_keys() {
    let app = TestApp::get().await;
    let config_name = unique_config_name("exec_proposers");
    let pubkey = TestApp::test_bls_pubkey(&format!("prop{}", TestApp::unique_id()));

    // Create default config
    let create_resp = app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": config_name,
            "fee_recipient": "0xdef1def1def1def1def1def1def1def1def1def1",
            "gas_limit": "30000000",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    assert_eq!(create_resp.status(), 201, "Config creation failed");

    // Create proposer with custom config
    let proposer_resp = app.client()
        .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
        .json(&json!({
            "fee_recipient": "0x5e8422345238f34275888049021821e8e08caa1f",
            "gas_limit": "35000000",
            "reset_relays": true
        }))
        .send()
        .await
        .expect("Failed to create proposer");

    assert!(proposer_resp.status() == 200 || proposer_resp.status() == 201, "Proposer creation failed");

    // Get execution config with proposer key
    let response = app
        .client()
        .post(&format!("{}/vouch/v2/execution-config/{}", app.address, config_name))
        .json(&json!([pubkey]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");

    // Should have default values
    assert_eq!(body.fee_recipient, Some("0xdef1def1def1def1def1def1def1def1def1def1".to_string()));

    // Should have proposer-specific entry
    assert!(body.proposers.is_some());
    let proposers = body.proposers.as_ref().unwrap();
    assert_eq!(proposers.len(), 1);
    assert_eq!(proposers[0].proposer, pubkey);
    assert_eq!(proposers[0].fee_recipient, Some("0x5e8422345238f34275888049021821e8e08caa1f".to_string()));
    assert_eq!(proposers[0].gas_limit, Some("35000000".to_string()));
    assert_eq!(proposers[0].reset_relays, Some(true));

    delete_proposer(app, &pubkey).await;
    delete_config(app, &config_name).await;
}

#[tokio::test]
async fn test_get_execution_config_unknown_keys() {
    let app = TestApp::get().await;
    let config_name = unique_config_name("exec_unknown");
    let unknown_key = TestApp::test_bls_pubkey(&format!("unk{}", TestApp::unique_id()));

    // Create default config
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": config_name,
            "fee_recipient": "0xdef1def1def1def1def1def1def1def1def1def1",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Request with keys that don't have specific configs
    let response = app
        .client()
        .post(&format!("{}/vouch/v2/execution-config/{}", app.address, config_name))
        .json(&json!([unknown_key]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");

    // Should return default config
    assert_eq!(body.fee_recipient, Some("0xdef1def1def1def1def1def1def1def1def1def1".to_string()));
    // For unknown keys, either proposers is empty/None, or if returned, they shouldn't have specific overrides
    if let Some(proposers) = &body.proposers {
        // If proposers is returned for unknown keys, verify no specific overrides for our key
        let our_proposer = proposers.iter().find(|p| p.proposer == unknown_key);
        if let Some(p) = our_proposer {
            // Unknown key should have no specific fee_recipient override
            assert!(p.fee_recipient.is_none(), "Unknown key should not have specific fee_recipient");
        }
    }

    delete_config(app, &config_name).await;
}

// ============================================================================
// Tags Filter Tests
// ============================================================================

#[tokio::test]
async fn test_get_execution_config_with_tags() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let config_name = format!("test_exec_tags_{}", id);
    let pattern_name = format!("test_pattern_lido_{}", id);

    // Create default config
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": config_name,
            "fee_recipient": "0xdef1def1def1def1def1def1def1def1def1def1",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Create proposer pattern with tags
    app.client()
        .post(&format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": pattern_name,
            "pattern": "^0xtest.*$",
            "tags": ["lido", "liquid-staking"],
            "fee_recipient": "0x11d011d011d011d011d011d011d011d011d011d0",
            "gas_limit": "32000000"
        }))
        .send()
        .await
        .expect("Failed to create pattern");

    // Get execution config with tags filter
    let response = app
        .client()
        .post(&format!("{}/vouch/v2/execution-config/{}?tags=lido", app.address, config_name))
        .json(&json!([]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");

    // Should include pattern in proposers
    assert!(body.proposers.is_some());
    let proposers = body.proposers.as_ref().unwrap();
    // Should have the pattern entry
    let pattern_entry = proposers.iter().find(|p| p.proposer == "^0xtest.*$");
    assert!(pattern_entry.is_some());
    assert_eq!(pattern_entry.unwrap().fee_recipient, Some("0x11d011d011d011d011d011d011d011d011d011d0".to_string()));

    delete_pattern(app, &pattern_name).await;
    delete_config(app, &config_name).await;
}

// ============================================================================
// Multiple Proposers Test
// ============================================================================

#[tokio::test]
async fn test_get_execution_config_multiple_proposers() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let config_name = format!("test_exec_multi_{}", id);

    // Create default config
    app.client()
        .post(&format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": config_name,
            "fee_recipient": "0xdef1def1def1def1def1def1def1def1def1def1",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Create multiple proposers with unique keys
    let pubkey1 = TestApp::test_bls_pubkey(&format!("m1{}", id));
    let pubkey2 = TestApp::test_bls_pubkey(&format!("m2{}", id));
    let pubkey3 = TestApp::test_bls_pubkey(&format!("m3{}", id));

    for (i, pubkey) in [&pubkey1, &pubkey2, &pubkey3].iter().enumerate() {
        app.client()
            .put(&format!("{}/api/admin/vouch/proposers/{}", app.address, pubkey))
            .json(&json!({
                "gas_limit": format!("{}0000000", 30 + i)
            }))
            .send()
            .await
            .expect("Failed to create proposer");
    }

    // Get execution config with all three keys
    let response = app
        .client()
        .post(&format!("{}/vouch/v2/execution-config/{}", app.address, config_name))
        .json(&json!([pubkey1.clone(), pubkey2.clone(), pubkey3.clone()]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");

    assert!(body.proposers.is_some());
    let proposers = body.proposers.as_ref().unwrap();
    assert_eq!(proposers.len(), 3);

    // Cleanup
    for pubkey in [&pubkey1, &pubkey2, &pubkey3] {
        delete_proposer(app, pubkey).await;
    }
    delete_config(app, &config_name).await;
}
