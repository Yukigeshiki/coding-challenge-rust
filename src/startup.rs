use std::net::TcpListener;

use axum::http::Method;
use axum::{
    http::Request,
    routing::{get, IntoMakeService},
    Router, Server,
};
use hyper::server::conn::AddrIncoming;
use reqwest::Client;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::{
    request_id::{MakeRequestId, RequestId},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::Level;
use uuid::Uuid;

use crate::handlers::{get_animal_fact, health_check};

pub type App = Server<AddrIncoming, IntoMakeService<Router>>;

#[derive(Clone)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        let request_id = Uuid::new_v4().to_string();

        Some(RequestId::new(request_id.parse().unwrap()))
    }
}

pub fn run(listener: TcpListener) -> hyper::Result<App> {
    let client = Client::new();
    let app = Router::new()
        .route("/health-check", get(health_check))
        .route("/fact", get(get_animal_fact))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET]),
        )
        .layer(
            ServiceBuilder::new()
                .set_x_request_id(MakeRequestUuid)
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(
                            DefaultMakeSpan::new()
                                .include_headers(true)
                                .level(Level::INFO),
                        )
                        .on_response(DefaultOnResponse::new().include_headers(true)),
                )
                .propagate_x_request_id(),
        )
        .with_state(client);

    Ok(Server::from_tcp(listener)?.serve(app.into_make_service()))
}
