use crate::app::util::error::ServiceError;
use axum::response::IntoResponse;
use hyper::StatusCode;
use tracing::info;

pub async fn root_handler() -> impl IntoResponse {
    info!("root handler called");
    Ok::<_, ServiceError>(StatusCode::OK)
}

pub async fn root_failure_handler() -> impl IntoResponse {
    info!("root failure handler called");
    Err::<StatusCode, _>(ServiceError::BadCredential)
}
