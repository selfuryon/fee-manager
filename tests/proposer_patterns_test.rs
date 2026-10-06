// tests/proposer_patterns_test.rs - Proposer Pattern CRUD and filtering tests
mod common;

use common::TestApp;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
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
#[allow(dead_code)]
struct ProposerPatternResponse {
    name: String,
    pattern: String,
    tags: Vec<String>,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    min_value: Option<String>,
    reset_relays: bool,
    relays: Option<HashMap<String, RelayConfig>>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ProposerPatternListItem {
    name: String,
    pattern: String,
    tags: Vec<String>,
    fee_recipient: Option<String>,
    gas_limit: Option<String>,
    min_value: Option<String>,
    reset_relays: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ListResponse<T> {
    items: Vec<T>,
    #[allow(dead_code)]
    next_cursor: Option<String>,
}

/// Helper to create unique pattern name
fn unique_pattern_name(prefix: &str) -> String {
    format!("test_pattern_{}_{}", prefix, TestApp::unique_id())
}

/// Helper to delete a pattern
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

// ============================================================================
// CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_create_proposer_pattern() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("create");

    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name,
            "pattern": "^0x8[0-9a-f]{94}$",
            "tags": ["lido", "liquid-staking"],
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "reset_relays": false
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 201);

    let body: ProposerPatternResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert_eq!(body.pattern, "^0x8[0-9a-f]{94}$");
    assert_eq!(body.tags, vec!["lido", "liquid-staking"]);
    assert!(!body.reset_relays);

    delete_pattern(app, &name).await;
}

#[tokio::test]
async fn test_create_proposer_pattern_with_relays() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("relays");

    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name,
            "pattern": "^0x9[0-9a-f]{94}$",
            "tags": ["rocketpool"],
            "reset_relays": true,
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f"
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

    let body: ProposerPatternResponse = response.json().await.expect("Failed to parse JSON");
    assert!(body.reset_relays);
    assert!(body.relays.is_some());
    assert_eq!(body.relays.as_ref().unwrap().len(), 2);

    delete_pattern(app, &name).await;
}

#[tokio::test]
async fn test_create_proposer_pattern_duplicate() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("dup");

    // Create first pattern
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name,
            "pattern": "^0x[0-9a-f]+$"
        }))
        .send()
        .await
        .expect("Failed to create pattern");

    // Try to create duplicate
    let response = app
        .client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name,
            "pattern": "^0x[0-9a-f]+$"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "CONFLICT");

    delete_pattern(app, &name).await;
}

#[tokio::test]
async fn test_get_proposer_pattern() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("get");

    // Create pattern
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name,
            "pattern": "^0xa[0-9a-f]{94}$",
            "tags": ["coinbase"]
        }))
        .send()
        .await
        .expect("Failed to create pattern");

    // Get pattern
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns/{}",
            app.address, name
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ProposerPatternResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.name, name);
    assert_eq!(body.pattern, "^0xa[0-9a-f]{94}$");
    assert_eq!(body.tags, vec!["coinbase"]);

    delete_pattern(app, &name).await;
}

#[tokio::test]
async fn test_get_proposer_pattern_not_found() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("notfound");

    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns/{}",
            app.address, name
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_update_proposer_pattern() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("update");

    // Create pattern
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name,
            "pattern": "^0xb[0-9a-f]{94}$",
            "tags": ["solo"],
            "reset_relays": false
        }))
        .send()
        .await
        .expect("Failed to create pattern");

    // Update pattern
    let response = app
        .client()
        .put(format!(
            "{}/api/admin/vouch/proposer-patterns/{}",
            app.address, name
        ))
        .json(&json!({
            "pattern": "^0xc[0-9a-f]{94}$",
            "tags": ["solo", "home-staker"],
            "reset_relays": true
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ProposerPatternResponse = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.pattern, "^0xc[0-9a-f]{94}$");
    assert_eq!(body.tags, vec!["solo", "home-staker"]);
    assert!(body.reset_relays);

    delete_pattern(app, &name).await;
}

#[tokio::test]
async fn test_delete_proposer_pattern() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("delete");

    // Create pattern
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name,
            "pattern": "^0x[0-9a-f]+$"
        }))
        .send()
        .await
        .expect("Failed to create pattern");

    // Delete pattern
    let response = app
        .client()
        .delete(format!(
            "{}/api/admin/vouch/proposer-patterns/{}",
            app.address, name
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 204);

    // Verify deleted
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns/{}",
            app.address, name
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

// ============================================================================
// Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_list_proposer_patterns() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create multiple patterns
    let names: Vec<String> = (1..=3).map(|i| format!("test_list_{}_{}", id, i)).collect();
    for (i, name) in names.iter().enumerate() {
        app.client()
            .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
            .json(&json!({
                "name": name,
                "pattern": format!("^0x{}[0-9a-f]{{94}}$", i),
                "tags": [format!("tag{}", i)]
            }))
            .send()
            .await
            .expect("Failed to create pattern");
    }

    // List all test patterns
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_list_{}",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: ListResponse<ProposerPatternListItem> =
        response.json().await.expect("Failed to parse JSON");
    assert_eq!(body.items.len(), 3);

    // Cleanup
    for name in &names {
        delete_pattern(app, name).await;
    }
}

#[tokio::test]
async fn test_filter_by_tag() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create patterns with different tags
    let name1 = format!("test_tag_{}_{}", id, 1);
    let name2 = format!("test_tag_{}_{}", id, 2);
    let name3 = format!("test_tag_{}_{}", id, 3);

    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name1,
            "pattern": "^0x1[0-9a-f]{94}$",
            "tags": ["lido", "liquid-staking"]
        }))
        .send()
        .await
        .unwrap();

    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name2,
            "pattern": "^0x2[0-9a-f]{94}$",
            "tags": ["rocketpool", "decentralized"]
        }))
        .send()
        .await
        .unwrap();

    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name3,
            "pattern": "^0x3[0-9a-f]{94}$",
            "tags": ["lido", "institutional"]
        }))
        .send()
        .await
        .unwrap();

    // Filter by tag "lido"
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_tag_{}&tag=lido",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 2);
    assert!(
        body.items
            .iter()
            .all(|p| p.tags.contains(&"lido".to_string()))
    );

    // Filter by tag "decentralized"
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_tag_{}&tag=decentralized",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 1);

    // Cleanup
    delete_pattern(app, &name1).await;
    delete_pattern(app, &name2).await;
    delete_pattern(app, &name3).await;
}

#[tokio::test]
async fn test_filter_by_pattern() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create patterns
    let name1 = format!("test_pat_{}_{}", id, 1);
    let name2 = format!("test_pat_{}_{}", id, 2);

    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name1,
            "pattern": "^0x8[0-9a-f]{94}$"
        }))
        .send()
        .await
        .unwrap();

    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name2,
            "pattern": "^0x9[0-9a-f]{94}$"
        }))
        .send()
        .await
        .unwrap();

    // Filter by pattern substring
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_pat_{}&pattern=0x8",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 1);
    assert!(body.items[0].pattern.contains("0x8"));

    // Cleanup
    delete_pattern(app, &name1).await;
    delete_pattern(app, &name2).await;
}

#[tokio::test]
async fn test_filter_by_reset_relays() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create patterns with different reset_relays
    let names: Vec<String> = (1..=4)
        .map(|i| format!("test_reset_{}_{}", id, i))
        .collect();
    for (i, name) in names.iter().enumerate() {
        app.client()
            .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
            .json(&json!({
                "name": name,
                "pattern": format!("^0x{}[0-9a-f]{{94}}$", i),
                "reset_relays": (i + 1) % 2 == 0  // 2,4 = true; 1,3 = false
            }))
            .send()
            .await
            .expect("Failed to create pattern");
    }

    // Filter reset_relays = true
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_reset_{}&reset_relays=true",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 2);
    assert!(body.items.iter().all(|p| p.reset_relays));

    // Cleanup
    for name in &names {
        delete_pattern(app, name).await;
    }
}

#[tokio::test]
async fn test_proposer_patterns_pagination() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();

    // Create 5 patterns
    let names: Vec<String> = (1..=5).map(|i| format!("test_page_{}_{}", id, i)).collect();
    for (i, name) in names.iter().enumerate() {
        app.client()
            .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
            .json(&json!({
                "name": name,
                "pattern": format!("^0x{}[0-9a-f]{{94}}$", i)
            }))
            .send()
            .await
            .expect("Failed to create pattern");
    }

    // Walk the five patterns two at a time by following next_cursor
    let (pages, keys) = app
        .walk_pages(
            &format!(
                "{}/api/admin/vouch/proposer-patterns?name=test_page_{}&limit=2",
                app.address, id
            ),
            "name",
        )
        .await;
    assert_eq!(pages, vec![2, 2, 1]);
    let mut expected = names.clone();
    expected.sort();
    assert_eq!(keys, expected, "every pattern once, in name order");

    // Cleanup
    for name in &names {
        delete_pattern(app, name).await;
    }
}

// ============================================================================
// Relay Filter Tests
// ============================================================================

#[tokio::test]
async fn test_filter_patterns_by_relay_url() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let name_with_relay = format!("test_relay_url_{}_with", id);
    let name_without_relay = format!("test_relay_url_{}_without", id);

    // Create pattern with relay
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name_with_relay,
            "pattern": "^0xa[0-9a-f]{94}$",
            "relays": {
                "https://flashbots.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create pattern with relay");

    // Create pattern without relay
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name_without_relay,
            "pattern": "^0xb[0-9a-f]{94}$"
        }))
        .send()
        .await
        .expect("Failed to create pattern without relay");

    // Filter by relay_url prefix
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_relay_url_{}&relay_url=https://flashbots",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 1);
    assert!(body.items[0].name.contains("with"));

    // Cleanup
    delete_pattern(app, &name_with_relay).await;
    delete_pattern(app, &name_without_relay).await;
}

#[tokio::test]
async fn test_filter_patterns_by_relay_min_value() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let name_with_min = format!("test_relay_min_{}_with", id);
    let name_without_min = format!("test_relay_min_{}_without", id);

    // Create pattern with relay that has min_value
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name_with_min,
            "pattern": "^0xc[0-9a-f]{94}$",
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f",
                    "min_value": "77000000000000000"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create pattern with relay min_value");

    // Create pattern with relay without min_value
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name_without_min,
            "pattern": "^0xd[0-9a-f]{94}$",
            "relays": {
                "https://relay2.example.com": {
                    "public_key": "0xac6e77dfe25ecd6110b8e780608cce0dab71fdd5ebea22a16c0205200f2f8e2e3ad3b71d3499c54ad14d6c21b41a37ae"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create pattern without relay min_value");

    // Filter by relay_min_value
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_relay_min_{}&relay_min_value=77000000000000000",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 1);
    assert!(body.items[0].name.contains("with"));

    // Cleanup
    delete_pattern(app, &name_with_min).await;
    delete_pattern(app, &name_without_min).await;
}

#[tokio::test]
async fn test_filter_patterns_by_relay_disabled() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let name_disabled = format!("test_relay_dis_{}_disabled", id);
    let name_enabled = format!("test_relay_dis_{}_enabled", id);

    // Create pattern with disabled relay
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name_disabled,
            "pattern": "^0xe[0-9a-f]{94}$",
            "relays": {
                "https://relay1.example.com": {
                    "public_key": "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f",
                    "disabled": true
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create pattern with disabled relay");

    // Create pattern with enabled relay
    app.client()
        .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
        .json(&json!({
            "name": name_enabled,
            "pattern": "^0xf[0-9a-f]{94}$",
            "relays": {
                "https://relay2.example.com": {
                    "public_key": "0xac6e77dfe25ecd6110b8e780608cce0dab71fdd5ebea22a16c0205200f2f8e2e3ad3b71d3499c54ad14d6c21b41a37ae",
                    "disabled": false
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create pattern with enabled relay");

    // Filter by relay_disabled=true
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_relay_dis_{}&relay_disabled=true",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 1);
    assert!(body.items[0].name.contains("disabled"));

    // Filter by relay_disabled=false
    let response = app
        .client()
        .get(format!(
            "{}/api/admin/vouch/proposer-patterns?name=test_relay_dis_{}&relay_disabled=false",
            app.address, id
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: ListResponse<ProposerPatternListItem> = response.json().await.unwrap();
    assert_eq!(body.items.len(), 1);
    assert!(body.items[0].name.contains("enabled"));

    // Cleanup
    delete_pattern(app, &name_disabled).await;
    delete_pattern(app, &name_enabled).await;
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn test_pattern_validation() {
    let app = TestApp::get().await;
    let base = format!("{}/api/admin/vouch/proposer-patterns", app.address);

    // A pattern that does not compile is rejected and nothing is stored
    let bad = unique_pattern_name("badre");
    let response = app
        .client()
        .post(&base)
        .json(&json!({"name": bad, "pattern": "([unclosed", "tags": ["test"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    let error: serde_json::Value = response.json().await.unwrap();
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("pattern")
    );
    let response = app
        .client()
        .get(format!("{base}/{bad}"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);

    // Invalid min_value on create is rejected
    let response = app
        .client()
        .post(&base)
        .json(
            &json!({"name": unique_pattern_name("badmin"), "pattern": "^x$", "min_value": "cheap"}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);

    // Valid values round-trip unchanged
    let name = unique_pattern_name("valid");
    let response = app
        .client()
        .post(&base)
        .json(&json!({"name": name, "pattern": "^pool/.*$", "min_value": "0.10"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    let stored: ProposerPatternResponse = response.json().await.unwrap();
    assert_eq!(stored.min_value.as_deref(), Some("0.10"));

    // An invalid pattern on update is rejected and the stored one is kept
    let response = app
        .client()
        .put(format!("{base}/{name}"))
        .json(&json!({"pattern": "([unclosed"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    let stored: ProposerPatternResponse = app
        .client()
        .get(format!("{base}/{name}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored.pattern, "^pool/.*$");

    delete_pattern(app, &name).await;
}

#[tokio::test]
async fn test_concurrent_creates_one_wins() {
    let app = TestApp::get().await;
    let name = unique_pattern_name("race");
    let create = |name: String| async move {
        app.client()
            .post(format!("{}/api/admin/vouch/proposer-patterns", app.address))
            .json(&json!({"name": name, "pattern": "^race/.*$"}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16()
    };
    let (a, b) = tokio::join!(
        tokio::spawn(create(name.clone())),
        tokio::spawn(create(name.clone()))
    );
    let mut statuses = [a.unwrap(), b.unwrap()];
    statuses.sort();
    assert_eq!(statuses, [201, 409]);

    delete_pattern(app, &name).await;
}
