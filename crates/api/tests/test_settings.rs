mod common;

use common::MockChannelRegistry;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::mpsc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn make_state(db: Arc<llm_gateway_storage::sqlite::SqliteStorage>) -> Arc<AppState> {
    let (audit_tx, _rx) = mpsc::channel(100);
    Arc::new(AppState {
        storage: db.clone() as Arc<dyn Storage>,
        rate_limiter: Arc::new(RateLimiter::new(60)),
        audit_logger: Arc::new(AuditLogger::new(db as Arc<dyn Storage>)),
        jwt_secret: common::TEST_JWT_SECRET.to_string(),
        encryption_key: [0u8; 32],
        audit_tx,
        registry: Arc::new(MockChannelRegistry),
    })
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

#[tokio::test]
async fn test_get_settings_default_allow_registration_true() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/settings")
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["allow_registration"], true);
}

#[tokio::test]
async fn test_get_settings_without_admin_auth_returns_401() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_settings_disable_registration() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/admin/settings")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"allow_registration": false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["allow_registration"], false);
}

#[tokio::test]
async fn test_get_settings_after_update() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    // Update settings to disable registration
    app.clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/admin/settings")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"allow_registration": false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Get settings and verify the new value
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/settings")
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["allow_registration"], false);
}
