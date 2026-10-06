// tests/config_management_test.rs - PUT replaces, and proposer key identity
mod common;

use common::TestApp;
use serde_json::{Value, json};

const RELAY_KEY: &str = "0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f";

async fn send(
    app: &TestApp,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let mut request = app
        .client()
        .request(method, format!("{}{path}", app.address));
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    (status, response.json().await.unwrap_or_default())
}

#[tokio::test]
async fn test_put_default_config_clears_omitted_fields() {
    let app = TestApp::get().await;
    let name = format!("test_replace_{}", TestApp::unique_id());
    let path = format!("/api/admin/vouch/configs/default/{name}");
    let (status, _) = send(
        app,
        reqwest::Method::POST,
        "/api/admin/vouch/configs/default",
        Some(json!({
            "name": name,
            "active": false,
            "fee_recipient": TestApp::test_eth_address("01"),
            "min_value": "0.1",
            "relays": {"https://relay.example.invalid/": {"public_key": RELAY_KEY}}
        })),
    )
    .await;
    assert_eq!(status, 201);

    let (status, body) = send(
        app,
        reqwest::Method::PUT,
        &path,
        Some(json!({"gas_limit": "30000000"})),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["gas_limit"], "30000000");
    assert!(
        body.get("fee_recipient").is_none_or(Value::is_null),
        "{body}"
    );
    assert!(body.get("min_value").is_none_or(Value::is_null), "{body}");
    assert!(body.get("relays").is_none_or(Value::is_null), "{body}");
    assert_eq!(body["active"], true, "omitted active resets to true");

    send(app, reqwest::Method::DELETE, &path, None).await;
}

#[tokio::test]
async fn test_put_missing_default_config_is_404() {
    let app = TestApp::get().await;
    let (status, body) = send(
        app,
        reqwest::Method::PUT,
        &format!(
            "/api/admin/vouch/configs/default/missing_{}",
            TestApp::unique_id()
        ),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_put_pattern_replaces_and_requires_pattern() {
    let app = TestApp::get().await;
    let name = format!("test_replace_pat_{}", TestApp::unique_id());
    let path = format!("/api/admin/vouch/proposer-patterns/{name}");
    let (status, _) = send(
        app,
        reqwest::Method::POST,
        "/api/admin/vouch/proposer-patterns",
        Some(json!({
            "name": name,
            "pattern": "^old/.*$",
            "tags": ["test"],
            "relays": {
                "https://a.relay.example.invalid/": {"public_key": RELAY_KEY},
                "https://b.relay.example.invalid/": {"public_key": RELAY_KEY}
            }
        })),
    )
    .await;
    assert_eq!(status, 201);

    // Missing pattern: rejected, nothing changes
    let (status, body) = send(
        app,
        reqwest::Method::PUT,
        &path,
        Some(json!({"tags": ["x"]})),
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(body["error"]["code"], "INVALID_DATA");
    let (_, stored) = send(app, reqwest::Method::GET, &path, None).await;
    assert_eq!(stored["pattern"], "^old/.*$");
    assert_eq!(stored["relays"].as_object().map(|r| r.len()), Some(2));

    // Only pattern sent: tags and relays are emptied
    let (status, body) = send(
        app,
        reqwest::Method::PUT,
        &path,
        Some(json!({"pattern": "^pool-example/.*$"})),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["pattern"], "^pool-example/.*$");
    assert_eq!(body["tags"], json!([]));
    assert!(body.get("relays").is_none_or(Value::is_null), "{body}");

    send(app, reqwest::Method::DELETE, &path, None).await;
}

/// `0x` + the hex digits upper-cased
fn upper(key: &str) -> String {
    format!("0x{}", key[2..].to_uppercase())
}

#[tokio::test]
async fn test_upper_case_proposer_key_is_normalized_and_served() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    let key = TestApp::test_bls_pubkey(&format!("ca{id}"));
    let config = format!("test_case_{id}");

    let (status, body) = send(
        app,
        reqwest::Method::PUT,
        &format!("/api/admin/vouch/proposers/{}", upper(&key)),
        Some(json!({"fee_recipient": TestApp::test_eth_address("02")})),
    )
    .await;
    assert_eq!(status, 201);
    assert_eq!(body["public_key"], key, "stored lower-case");

    // GET under the other case finds it
    let (status, body) = send(
        app,
        reqwest::Method::GET,
        &format!("/api/admin/vouch/proposers/{}", upper(&key)),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["public_key"], key);

    // Vouch sends lower-case keys and gets the proposer's entry
    send(
        app,
        reqwest::Method::POST,
        "/api/admin/vouch/configs/default",
        Some(json!({"name": config, "active": true})),
    )
    .await;
    let response = app
        .client_unauthenticated()
        .post(format!(
            "{}/vouch/v2/execution-config/{config}",
            app.address
        ))
        .json(&json!([key]))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let served: Value = response.json().await.unwrap();
    assert_eq!(served["proposers"][0]["proposer"], key);

    send(
        app,
        reqwest::Method::DELETE,
        &format!("/api/admin/vouch/proposers/{}", upper(&key)),
        None,
    )
    .await;
    send(
        app,
        reqwest::Method::DELETE,
        &format!("/api/admin/vouch/configs/default/{config}"),
        None,
    )
    .await;
}

#[tokio::test]
async fn test_invalid_proposer_key_rejected() {
    let app = TestApp::get().await;
    let (status, body) = send(
        app,
        reqwest::Method::PUT,
        "/api/admin/vouch/proposers/not-a-key",
        Some(json!({})),
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(body["error"]["code"], "INVALID_DATA");
}
