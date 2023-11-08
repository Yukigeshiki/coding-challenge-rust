use std::net::TcpListener;

use coding_challenge::{
    config::get_config,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> hyper::Result<()> {
    let sub = get_subscriber("coding-challenge".into(), "info".into(), std::io::stdout);
    init_subscriber(sub);

    // halt the program if there are any errors reading config or binding a port
    let conf = get_config().expect("Cannot read config");
    let addr = format!("{}:{}", conf.application.host, conf.application.port);
    let listener = TcpListener::bind(&addr).expect("Unable to bind to port");

    tracing::info!("Application starting on: {addr}!");

    run(listener)?.await
}
