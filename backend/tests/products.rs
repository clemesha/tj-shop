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
async fn list_products_returns_seeded_catalog(pool: PgPool) {
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
                .uri("/api/products")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    let products = body.as_array().expect("products should be a JSON array");

    assert!(
        products.len() > 2000,
        "expected the full seed catalog, got {} products",
        products.len()
    );

    let bananas = products
        .iter()
        .find(|p| p["sku"] == "048053")
        .expect("Bananas (sku 048053) should be in the catalog");

    assert_eq!(bananas["name"], "Bananas");
    assert_eq!(bananas["size"], "1 Each");
    assert_eq!(bananas["is_manual"], false);
    assert!(bananas["id"].as_str().is_some(), "id should be a UUID string");
    assert!(
        bananas["category_id"].as_str().is_some(),
        "category_id should be set"
    );
    assert!(
        bananas["image_url"].as_str().unwrap().contains("48053.png"),
        "image_url should contain the SKU"
    );
}
