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
    let _ = app
        .client()
        .delete(format!(
            "{}/api/admin/vouch/configs/default/{}",
            app.address, name
        ))
        .send()
        .await;
}

/// Helper to delete a proposer pattern
async fn delete_pattern(app: &TestApp, name: &str) {
    let _ = app
        .client()
        .delete(format!(
            "{}/api/admin/vouch/proposer-patterns/{}",
            app.address, name
        ))
        .send()
        .await;
}

/// Helper to delete a proposer
async fn delete_proposer(app: &TestApp, pubkey: &str) {
    let _ = app
        .client()
        .delete(format!(
            "{}/api/admin/vouch/proposers/{}",
            app.address, pubkey
        ))
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
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
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
        .post(format!(
            "{}/vouch/v2/execution-config/{}",
            app.address, config_name
        ))
        .json(&json!([]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.version, 2);
    assert_eq!(
        body.fee_recipient,
        Some("0x1234567890abcdef1234567890abcdef12345678".to_string())
    );
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
        .post(format!(
            "{}/vouch/v2/execution-config/{}",
            app.address, config_name
        ))
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
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
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
        .post(format!(
            "{}/vouch/v2/execution-config/{}",
            app.address, config_name
        ))
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
    let create_resp = app
        .client()
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
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
    let proposer_resp = app
        .client()
        .put(format!(
            "{}/api/admin/vouch/proposers/{}",
            app.address, pubkey
        ))
        .json(&json!({
            "fee_recipient": "0x5e8422345238f34275888049021821e8e08caa1f",
            "gas_limit": "35000000",
            "reset_relays": true
        }))
        .send()
        .await
        .expect("Failed to create proposer");

    assert!(
        proposer_resp.status() == 200 || proposer_resp.status() == 201,
        "Proposer creation failed"
    );

    // Get execution config with proposer key
    let response = app
        .client()
        .post(format!(
            "{}/vouch/v2/execution-config/{}",
            app.address, config_name
        ))
        .json(&json!([pubkey]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");

    // Should have default values
    assert_eq!(
        body.fee_recipient,
        Some("0xdef1def1def1def1def1def1def1def1def1def1".to_string())
    );

    // Should have proposer-specific entry
    assert!(body.proposers.is_some());
    let proposers = body.proposers.as_ref().unwrap();
    assert_eq!(proposers.len(), 1);
    assert_eq!(proposers[0].proposer, pubkey);
    assert_eq!(
        proposers[0].fee_recipient,
        Some("0x5e8422345238f34275888049021821e8e08caa1f".to_string())
    );
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
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
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
        .post(format!(
            "{}/vouch/v2/execution-config/{}",
            app.address, config_name
        ))
        .json(&json!([unknown_key]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");

    // Should return default config
    assert_eq!(
        body.fee_recipient,
        Some("0xdef1def1def1def1def1def1def1def1def1def1".to_string())
    );
    // For unknown keys, either proposers is empty/None, or if returned, they shouldn't have specific overrides
    if let Some(proposers) = &body.proposers {
        // If proposers is returned for unknown keys, verify no specific overrides for our key
        let our_proposer = proposers.iter().find(|p| p.proposer == unknown_key);
        if let Some(p) = our_proposer {
            // Unknown key should have no specific fee_recipient override
            assert!(
                p.fee_recipient.is_none(),
                "Unknown key should not have specific fee_recipient"
            );
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
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
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
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
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
        .post(format!(
            "{}/vouch/v2/execution-config/{}?tags=lido",
            app.address, config_name
        ))
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
    assert_eq!(
        pattern_entry.unwrap().fee_recipient,
        Some("0x11d011d011d011d011d011d011d011d011d011d0".to_string())
    );

    delete_pattern(app, &pattern_name).await;
    delete_config(app, &config_name).await;
}

// ============================================================================
// Tag Ordering Tests
// ============================================================================

#[tokio::test]
async fn test_execution_config_patterns_after_proposers_in_tag_order() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let config_name = format!("test_order_{}", id);
    let pattern_lido = format!("pattern_lido_{}", id);
    let pattern_dev = format!("pattern_dev_{}", id);
    let pubkey = TestApp::test_bls_pubkey(&format!("order{}", id));

    // Create default config
    app.client()
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": config_name,
            "fee_recipient": "0xdef1def1def1def1def1def1def1def1def1def1",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create config");

    // Create proposer
    app.client()
        .put(format!(
            "{}/api/admin/vouch/proposers/{}",
            app.address, pubkey
        ))
        .json(&json!({
            "fee_recipient": "0x1111111111111111111111111111111111111111"
        }))
        .send()
        .await
        .expect("Failed to create proposer");

    // Create pattern with "dev" tag
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": pattern_dev,
            "pattern": "^dev/.*$",
            "tags": ["dev"],
            "fee_recipient": "0x2222222222222222222222222222222222222222"
        }))
        .send()
        .await
        .expect("Failed to create dev pattern");

    // Create pattern with "lido" tag
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": pattern_lido,
            "pattern": "^lido/.*$",
            "tags": ["lido"],
            "fee_recipient": "0x3333333333333333333333333333333333333333"
        }))
        .send()
        .await
        .expect("Failed to create lido pattern");

    // Request with tags=lido,dev - lido should come before dev
    let response = app
        .client()
        .post(format!(
            "{}/vouch/v2/execution-config/{}?tags=lido,dev",
            app.address, config_name
        ))
        .json(&json!([pubkey]))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: ExecutionConfigResponse = response.json().await.expect("Failed to parse JSON");

    let proposers = body.proposers.expect("Should have proposers");

    // First should be the regular proposer (not a pattern)
    assert_eq!(proposers[0].proposer, pubkey, "Proposer should come first");

    // Find positions of our patterns
    let lido_pos = proposers.iter().position(|p| p.proposer == "^lido/.*$");
    let dev_pos = proposers.iter().position(|p| p.proposer == "^dev/.*$");

    assert!(lido_pos.is_some(), "Lido pattern should be present");
    assert!(dev_pos.is_some(), "Dev pattern should be present");

    // Both patterns should come after the proposer
    assert!(
        lido_pos.unwrap() > 0,
        "Lido pattern should come after proposer"
    );
    assert!(
        dev_pos.unwrap() > 0,
        "Dev pattern should come after proposer"
    );

    // Lido should come before dev (matches tag order: lido,dev)
    assert!(
        lido_pos.unwrap() < dev_pos.unwrap(),
        "Lido pattern (pos {}) should come before dev pattern (pos {}) per tag order",
        lido_pos.unwrap(),
        dev_pos.unwrap()
    );

    // Cleanup
    delete_proposer(app, &pubkey).await;
    delete_pattern(app, &pattern_lido).await;
    delete_pattern(app, &pattern_dev).await;
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
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
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
            .put(format!(
                "{}/api/admin/vouch/proposers/{}",
                app.address, pubkey
            ))
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
        .post(format!(
            "{}/vouch/v2/execution-config/{}",
            app.address, config_name
        ))
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

// ============================================================================
// Ordering, determinism and batching
// ============================================================================

async fn create_config(app: &TestApp, name: &str) {
    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
        .json(&json!({
            "name": name,
            "active": true,
            "relays": {
                "https://b.relay.example.invalid/": {"public_key": TestApp::test_bls_pubkey("b1")},
                "https://a.relay.example.invalid/": {"public_key": TestApp::test_bls_pubkey("a1")}
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
}

async fn create_pattern(app: &TestApp, name: &str, tag: &str) {
    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({"name": name, "pattern": format!("^{name}/.*$"), "tags": [tag]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn test_many_keys_ordered_with_relays() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let config_name = unique_config_name("many");
    create_config(app, &config_name).await;

    // Requested in reverse and created concurrently, so neither request order
    // nor insertion order matches key order
    let keys: Vec<String> = (0..200)
        .rev()
        .map(|i| TestApp::test_bls_pubkey(&format!("e0{id}{i:04x}")))
        .collect();
    let mut creates = tokio::task::JoinSet::new();
    for (i, key) in keys.iter().cloned().enumerate() {
        creates.spawn(async move {
            app.client()
                .put(format!("{}/api/admin/vouch/proposers/{}", app.address, key))
                .json(&json!({"relays": {
                    format!("https://r{i}.relay.example.invalid/"): {"public_key": TestApp::test_bls_pubkey("c1")}
                }}))
                .send()
                .await
                .unwrap()
                .status()
        });
    }
    for status in creates.join_all().await {
        assert_eq!(status, 201);
    }

    let started = std::time::Instant::now();
    let response = app
        .client_unauthenticated()
        .post(format!(
            "{}/vouch/v2/execution-config/{}",
            app.address, config_name
        ))
        .json(&keys)
        .send()
        .await
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(response.status(), 200);
    let body: ExecutionConfigResponse = response.json().await.unwrap();

    let proposers = body.proposers.unwrap();
    let returned: Vec<&str> = proposers.iter().map(|p| p.proposer.as_str()).collect();
    let mut expected: Vec<&str> = keys.iter().map(String::as_str).collect();
    expected.sort();
    assert_eq!(returned, expected, "entries must be ordered by public key");
    for (i, key) in keys.iter().enumerate() {
        let entry = proposers.iter().find(|p| &p.proposer == key).unwrap();
        let relays = entry.relays.as_ref().unwrap();
        assert_eq!(relays.len(), 1);
        assert!(relays.contains_key(&format!("https://r{i}.relay.example.invalid/")));
    }
    assert!(elapsed.as_secs_f64() < 1.0, "took {elapsed:?}");

    let mut deletes = tokio::task::JoinSet::new();
    for key in keys {
        deletes.spawn(async move { delete_proposer(app, &key).await });
    }
    deletes.join_all().await;
    delete_config(app, &config_name).await;
}

#[tokio::test]
async fn test_pattern_ties_ordered_by_name() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let config_name = unique_config_name("tie");
    let tag = format!("tie_{id}");
    let (zeta, alpha) = (format!("zeta_{id}"), format!("alpha_{id}"));
    create_config(app, &config_name).await;
    // zeta first, so insertion order would put it first
    create_pattern(app, &zeta, &tag).await;
    create_pattern(app, &alpha, &tag).await;

    for _ in 0..5 {
        let body: ExecutionConfigResponse = app
            .client_unauthenticated()
            .post(format!(
                "{}/vouch/v2/execution-config/{}?tags={}",
                app.address, config_name, tag
            ))
            .json(&json!([]))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let order: Vec<String> = body
            .proposers
            .unwrap()
            .into_iter()
            .map(|p| p.proposer)
            .collect();
        assert_eq!(order, vec![format!("^{alpha}/.*$"), format!("^{zeta}/.*$")]);
    }

    delete_pattern(app, &zeta).await;
    delete_pattern(app, &alpha).await;
    delete_config(app, &config_name).await;
}

#[tokio::test]
async fn test_identical_requests_identical_bodies() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let config_name = unique_config_name("bytes");
    let tag = format!("bytes_{id}");
    let pattern = format!("p_{id}");
    create_config(app, &config_name).await;
    create_pattern(app, &pattern, &tag).await;

    let fetch = || async {
        app.client_unauthenticated()
            .post(format!(
                "{}/vouch/v2/execution-config/{}?tags={}",
                app.address, config_name, tag
            ))
            .json(&json!([]))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap()
    };
    let first = fetch().await;
    for _ in 0..5 {
        assert_eq!(fetch().await, first);
    }
    // Relay keys serialize in sorted order
    let text = String::from_utf8(first.to_vec()).unwrap();
    assert!(text.find("https://a.relay").unwrap() < text.find("https://b.relay").unwrap());

    delete_pattern(app, &pattern).await;
    delete_config(app, &config_name).await;
}

// ============================================================================
// Request body limit
// ============================================================================

#[tokio::test]
async fn test_body_limit() {
    let app = TestApp::get().await;
    let config_name = unique_config_name("body");
    create_config(app, &config_name).await;
    let url = format!("{}/vouch/v2/execution-config/{}", app.address, config_name);

    // 30,000 keys (~3 MB, above the old 2 MB default) are accepted
    let keys: Vec<String> = (0..30_000u32).map(|i| format!("0x{:0>96x}", i)).collect();
    let response = app
        .client_unauthenticated()
        .post(&url)
        .json(&keys)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // Over 10 MB is rejected with the JSON error body
    let big = format!("[\"{}\"]", "a".repeat(10 * 1024 * 1024 + 1));
    let response = app
        .client_unauthenticated()
        .post(&url)
        .header("content-type", "application/json")
        .body(big)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 413);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "PAYLOAD_TOO_LARGE");

    // Admin routes keep the smaller default limit
    let admin_body = format!("{{\"name\": \"{}\"}}", "a".repeat(3 * 1024 * 1024));
    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/configs/default", app.address))
        .header("content-type", "application/json")
        .body(admin_body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 413);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "PAYLOAD_TOO_LARGE");

    delete_config(app, &config_name).await;
}
