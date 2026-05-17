pub mod health;

use axum::Router;

use crate::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
}

pub fn root_router() -> Router<AppState> {
    Router::new().merge(health::router())
}
