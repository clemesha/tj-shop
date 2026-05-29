pub mod categories;
pub mod health;
pub mod products;
pub mod stores;

use axum::Router;

use crate::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(stores::router())
        .merge(products::router())
        .merge(categories::router())
}

pub fn root_router() -> Router<AppState> {
    Router::new().merge(health::router())
}
