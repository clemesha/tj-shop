use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

use tj_shop::{
    AppState,
    config::{AppEnv, Config},
    router,
};

#[sqlx::test]
async fn health_returns_ok(pool: PgPool) {
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
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body, json!({ "status": "ok" }));
}
