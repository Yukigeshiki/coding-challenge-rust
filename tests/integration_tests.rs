#![warn(clippy::pedantic)]

use coding_challenge_rust::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use reqwest::Client;
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

    let server = coding_challenge_rust::startup::run(listener).expect("Failed to bind to address");

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
