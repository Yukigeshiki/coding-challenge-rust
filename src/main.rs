use coding_challenge::{
    config::get_config,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let sub = get_subscriber("coding-challenge".into(), "info".into(), std::io::stdout);
    init_subscriber(sub);

    // halt the program if there are any errors reading config or binding a port
    let conf = get_config().expect("Cannot read config");
    let addr = format!("{}:{}", conf.application.host, conf.application.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Unable to bind to port");

    tracing::info!("Application starting on: {addr}!");

    run(listener)
        .unwrap_or_else(|e| panic!("Application failed to start: {e}"))
        .await
        .unwrap()
}
