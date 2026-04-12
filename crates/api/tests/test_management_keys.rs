mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::{management, AppState};
use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt; // for oneshot()

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn make_state(db: Arc<llm_gateway_storage::sqlite::SqliteStorage>) -> Arc<AppState> {
    Arc::new(AppState {
        storage: db.clone() as Arc<dyn Storage>,
        rate_limiter: Arc::new(RateLimiter::new(60)),
        audit_logger: Arc::new(AuditLogger::new(db as Arc<dyn Storage>)),
        admin_token: "test-token".to_string(),
    })
}

#[tokio::test]
async fn test_create_key() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "test-key"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["name"], "test-key");
    assert!(body["key_hash"].is_string());
    assert_eq!(body["enabled"], true);
}

#[tokio::test]
async fn test_create_key_with_limits() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "limited-key", "rate_limit": 100, "budget_monthly": 50.0}).to_string(),
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
    assert_eq!(body["rate_limit"], 100);
    assert!((body["budget_monthly"].as_f64().unwrap() - 50.0).abs() < 0.001);
}

#[tokio::test]
async fn test_list_keys() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Create a key first
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "key1"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer test-token")
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
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_unauthorized_access() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer wrong-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_key() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Create
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "original"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let key_id = body["id"].as_str().unwrap();

    // Update
    let update_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/keys/{}", key_id))
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "updated"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated: Value = serde_json::from_slice(
        &to_bytes(update_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(updated["name"], "updated");
}

#[tokio::test]
async fn test_delete_key() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Create
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "to-delete"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let key_id = body["id"].as_str().unwrap();

    // Delete
    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/keys/{}", key_id))
                .header("authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted
    let get_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/keys/{}", key_id))
                .header("authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}
