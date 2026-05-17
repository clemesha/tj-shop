pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod routes;
pub mod telemetry;

use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveHeadersLayer;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
}

pub fn router(state: AppState) -> Router {
    let request_id_header = axum::http::HeaderName::from_static("x-request-id");

    let mut router = Router::new()
        .merge(routes::root_router())
        .nest("/api", routes::api_router())
        .layer(middleware::trace_layer())
        .layer(middleware::timeout_layer())
        .layer(SetRequestIdLayer::new(
            request_id_header.clone(),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(request_id_header))
        .layer(SetSensitiveHeadersLayer::new(std::iter::once(
            axum::http::header::AUTHORIZATION,
        )));

    if let Some(cors) = middleware::dev_cors_layer(state.config.app_env) {
        router = router.layer(cors);
    }

    router.with_state(state)
}
