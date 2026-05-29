use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;
use crate::error::Result;

#[derive(Debug, Serialize)]
struct Store {
    id: Uuid,
    tj_code: String,
    name: String,
    city: Option<String>,
    state: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/stores", get(list_stores))
}

async fn list_stores(State(state): State<AppState>) -> Result<Json<Vec<Store>>> {
    let stores = sqlx::query_as!(
        Store,
        r#"
        select id, tj_code, name, city, state
        from stores
        order by tj_code
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(stores))
}
