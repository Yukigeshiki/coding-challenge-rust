use hyper::StatusCode;

#[allow(clippy::async_yields_async)]
#[tracing::instrument(name = "Performing health check")]
pub async fn health_check() -> StatusCode {
    tracing::info!("Health check performed!");
    StatusCode::OK
}
