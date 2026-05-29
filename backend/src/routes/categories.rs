use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;
use crate::error::Result;

#[derive(Debug, Serialize)]
struct Category {
    id: Uuid,
    tj_id: String,
    name: String,
    position: i32,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/categories", get(list_categories))
}

async fn list_categories(State(state): State<AppState>) -> Result<Json<Vec<Category>>> {
    let categories = sqlx::query_as!(
        Category,
        r#"
        select id, tj_id, name, position
        from categories
        order by position
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(categories))
}
