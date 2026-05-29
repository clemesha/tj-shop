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
async fn list_stores_returns_seeded_stores(pool: PgPool) {
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
                .uri("/api/stores")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    let stores = body.as_array().expect("stores should be a JSON array");

    let codes: Vec<&str> = stores
        .iter()
        .map(|s| s["tj_code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, vec!["20", "21"], "expected La Jolla then Pacific Beach (sorted by tj_code)");

    let pb = stores.iter().find(|s| s["tj_code"] == "21").unwrap();
    assert_eq!(pb["name"], "Pacific Beach");
    assert_eq!(pb["city"], "San Diego");
    assert_eq!(pb["state"], "CA");
    assert!(pb["id"].as_str().is_some(), "id should be a UUID string");
}
