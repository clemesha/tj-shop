use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use tj_shop::{
    AppState,
    config::{AppEnv, Config},
    router,
};

#[sqlx::test]
async fn list_categories_returns_seeded_set(pool: PgPool) {
    let state = AppState {
        db: pool,
        config: Arc::new(Config {
            database_url: String::new(),
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            db_pool_max: 1,
            app_env: AppEnv::Dev,
        }),
    };

    let response = router(state)
        .oneshot(
            Request::builder()
                .uri("/api/categories")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    let categories = body.as_array().expect("categories should be a JSON array");

    assert_eq!(
        categories.len(),
        55,
        "expected the seeded 55 categories, got {}",
        categories.len()
    );

    let sliced_bread = categories
        .iter()
        .find(|c| c["tj_id"] == "14")
        .expect("category tj_id=14 (Sliced Bread) should exist");
    assert_eq!(sliced_bread["name"], "Sliced Bread");
    assert!(
        sliced_bread["position"].as_i64().is_some(),
        "position should be an integer"
    );
    assert!(
        sliced_bread["id"].as_str().is_some(),
        "id should be a UUID string"
    );

    let positions: Vec<i64> = categories
        .iter()
        .map(|c| c["position"].as_i64().unwrap())
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0] <= w[1]),
        "categories should be sorted by position ascending"
    );
}
