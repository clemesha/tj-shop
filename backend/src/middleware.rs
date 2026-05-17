use std::time::Duration;

use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::AppEnv;

pub fn trace_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>
{
    TraceLayer::new_for_http()
}

pub fn timeout_layer() -> TimeoutLayer {
    TimeoutLayer::new(Duration::from_secs(15))
}

pub fn dev_cors_layer(app_env: AppEnv) -> Option<CorsLayer> {
    if !app_env.is_dev() {
        return None;
    }
    Some(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
}
