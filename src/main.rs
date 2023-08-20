use shuttle_assignment::{
    config::get_config,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};
use std::net::TcpListener;

#[tokio::main]
async fn main() -> hyper::Result<()> {
    let sub = get_subscriber("shuttle-assignment".into(), "info".into(), std::io::stdout);
    init_subscriber(sub);

    let conf = get_config().expect("Cannot read config");
    let addr = format!("{}:{}", conf.application.host, conf.application.port);
    let listener = TcpListener::bind(addr).expect("Unable to bind to port");

    run(listener)?.await
}
