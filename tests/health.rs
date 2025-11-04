use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let runner = run(listener);
    let _ = tokio::spawn(runner);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn live_works() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/live", &address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!("{\"service\":\"ok\"}", response.text().await.unwrap());
}

#[tokio::test]
async fn ready_works() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/ready", &address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!("{\"service\":\"ok\"}", response.text().await.unwrap());
}
