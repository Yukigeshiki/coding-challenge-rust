#![warn(clippy::pedantic)]

use once_cell::sync::Lazy;
use reqwest::Client;
use shuttle_assignment::telemetry::{get_subscriber, init_subscriber};
use std::net::{SocketAddr, TcpListener};

static TRACING: Lazy<()> = Lazy::new(|| {
    let name = "test".to_string();
    let level = "debug".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let sub = get_subscriber(name, level, std::io::stdout);
        init_subscriber(sub);
    } else {
        let sub = get_subscriber(name, level, std::io::sink);
        init_subscriber(sub);
    }
});

struct TestApp {
    addr: SocketAddr,
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let addr = listener.local_addr().unwrap();

    let server = shuttle_assignment::startup::run(listener).expect("Failed to bind to address");

    tokio::spawn(server);

    TestApp { addr }
}

#[tokio::test]
async fn health_check_returns_200() {
    let TestApp { addr } = spawn_app().await;

    let client = Client::new();

    let resp = client
        .get(format!("http://{addr}/health-check"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(resp.status().is_success());
    assert_eq!(Some(0), resp.content_length());
}

#[tokio::test]
async fn get_animal_fact_returns_200() {
    let TestApp { addr } = spawn_app().await;

    let client = Client::new();

    let res = client
        .get(format!("http://{addr}/fact?animal=cat"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(res.status().is_success());
}

#[tokio::test]
async fn get_animal_fact_returns_non_empty_payload() {
    let TestApp { addr } = spawn_app().await;

    let client = Client::new();

    let res = client
        .get(format!("http://{addr}/fact?animal=cat"))
        .send()
        .await
        .expect("Failed to execute request.");

    let payload = res.text().await.expect("Failed to get text payload.");

    assert!(!payload.is_empty());

    let fact = serde_json::from_str::<Fact>(&payload).expect("Failed to deserialize payload");

    assert!(!fact.fact.is_empty());
    assert!(!fact.animal.is_empty());
}

#[derive(serde::Deserialize)]
struct Fact {
    fact: String,
    animal: String,
}

#[tokio::test]
async fn get_animal_fact_fails_when_no_param() {
    let TestApp { addr } = spawn_app().await;

    let client = Client::new();

    let res = client
        .get(format!("http://{addr}/fact"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(!res.status().is_success());
}
