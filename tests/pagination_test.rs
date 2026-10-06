// tests/pagination_test.rs - Keyset pagination contract of the admin lists
mod common;

use common::TestApp;
use serde_json::{Value, json};

const LISTS: [&str; 4] = [
    "/api/admin/vouch/proposers",
    "/api/admin/vouch/configs/default",
    "/api/admin/vouch/proposer-patterns",
    "/api/admin/commit-boost/mux",
];

#[tokio::test]
async fn test_limit_bounds() {
    let app = TestApp::get().await;
    for list in LISTS {
        for limit in [0, 1001] {
            let response = app
                .client()
                .get(format!("{}{list}?limit={limit}", app.address))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 400, "{list} limit={limit}");
            let body: Value = response.json().await.unwrap();
            assert_eq!(body["error"]["code"], "INVALID_QUERY");
        }
        let response = app
            .client()
            .get(format!("{}{list}?limit=1000", app.address))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "{list} limit=1000");
        let body: Value = response.json().await.unwrap();
        assert!(body["items"].is_array());
    }
}

async fn put_proposer(app: &TestApp, key: &str) {
    let response = app
        .client()
        .put(format!("{}/api/admin/vouch/proposers/{key}", app.address))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn test_insert_during_walk_visits_each_once() {
    let app = TestApp::get().await;
    let id = TestApp::unique_id();
    // Keys share a test-only prefix so the filter isolates them
    let prefix = format!("0xdeadc0{id}");
    let keys: Vec<String> = (1..=4)
        .map(|i| TestApp::test_bls_pubkey(&format!("c0{id}{i}")))
        .collect();
    for key in &keys {
        put_proposer(app, key).await;
    }
    let url = format!(
        "{}/api/admin/vouch/proposers?public_key={prefix}&limit=1",
        app.address
    );

    // First two pages by hand, then insert a key that sorts before all of them
    let mut seen = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..2 {
        let page_url = match &after {
            Some(c) => format!("{url}&after={c}"),
            None => url.clone(),
        };
        let body: Value = app
            .client()
            .get(page_url)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        seen.push(body["items"][0]["public_key"].as_str().unwrap().to_string());
        after = body["next_cursor"].as_str().map(String::from);
    }
    let early = TestApp::test_bls_pubkey(&format!("c0{id}0"));
    put_proposer(app, &early).await;

    // Rest of the walk
    loop {
        let page_url = format!("{url}&after={}", after.clone().unwrap());
        let body: Value = app
            .client()
            .get(page_url)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if let Some(item) = body["items"].as_array().unwrap().first() {
            seen.push(item["public_key"].as_str().unwrap().to_string());
        }
        after = body["next_cursor"].as_str().map(String::from);
        if after.is_none() {
            break;
        }
    }

    let mut expected = keys.clone();
    expected.sort();
    assert_eq!(seen, expected, "each pre-existing proposer exactly once");

    for key in keys.iter().chain([&early]) {
        let _ = app
            .client()
            .delete(format!("{}/api/admin/vouch/proposers/{key}", app.address))
            .send()
            .await;
    }
}
