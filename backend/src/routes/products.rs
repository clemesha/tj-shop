use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;
use crate::error::Result;

#[derive(Debug, Serialize)]
struct Product {
    id: Uuid,
    sku: Option<String>,
    name: String,
    size: Option<String>,
    image_url: Option<String>,
    category_id: Option<Uuid>,
    is_manual: bool,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/products", get(list_products))
}

async fn list_products(State(state): State<AppState>) -> Result<Json<Vec<Product>>> {
    let products = sqlx::query_as!(
        Product,
        r#"
        select id, sku, name, size, image_url, category_id, is_manual
        from products
        order by name
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(products))
}
