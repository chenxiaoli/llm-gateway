mod common;

use common::MockChannelRegistry;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::{AppState, SettlementTrigger};
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
    let (settlement_tx, _rx2) = mpsc::channel(1);
    Arc::new(AppState {
        storage: db.clone() as Arc<dyn Storage>,
        rate_limiter: Arc::new(RateLimiter::new(60)),
        audit_logger: Arc::new(AuditLogger::new(db as Arc<dyn Storage>)),
        jwt_secret: common::TEST_JWT_SECRET.to_string(),
        encryption_key: [0u8; 32],
        audit_tx,
        registry: Arc::new(MockChannelRegistry),
        settlement_tx,
    })
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

/// Helper: register a user via the auth endpoint and return the parsed response body.
async fn register_user(
    app: &axum::Router,
    username: &str,
    password: &str,
) -> Value {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": username, "password": password}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn test_list_users_admin() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register first user (admin)
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Register a second user
    register_user(&app, "regular", "password123").await;

    // List users
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header("authorization", bearer_token(admin_token))
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
    assert_eq!(body["total"], 2);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_list_users_without_auth_returns_401() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_user_role_admin_to_user() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Register a second user to become admin (so we can demote the first)
    let user_body = register_user(&app, "regular", "password123").await;
    let user_id = user_body["user"]["id"].as_str().unwrap();

    // Promote the second user to admin first
    let promote_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(promote_resp.status(), StatusCode::OK);

    // Now demote the original admin to user
    let admin_id = admin_body["user"]["id"].as_str().unwrap();
    let demote_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "user"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(demote_resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(demote_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["role"], "user");
}

#[tokio::test]
async fn test_update_user_role_user_to_admin() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Register a regular user
    let user_body = register_user(&app, "regular", "password123").await;
    let user_id = user_body["user"]["id"].as_str().unwrap();

    // Promote to admin
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["role"], "admin");
}

#[tokio::test]
async fn test_cannot_disable_last_admin_user() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();
    let admin_id = admin_body["user"]["id"].as_str().unwrap();

    // Try to disable the only admin
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"enabled": false}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cannot_demote_last_admin_user() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();
    let admin_id = admin_body["user"]["id"].as_str().unwrap();

    // Try to demote the only admin
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "user"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_delete_user() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Register a regular user to delete
    let user_body = register_user(&app, "regular", "password123").await;
    let user_id = user_body["user"]["id"].as_str().unwrap();

    // Delete the regular user
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify the user is gone by listing users
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body: Value = serde_json::from_slice(
        &to_bytes(list_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(list_body["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_cannot_delete_last_admin_user() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();
    let admin_id = admin_body["user"]["id"].as_str().unwrap();

    // Try to delete the only admin
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_nonexistent_user_returns_404() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/admin/users/nonexistent-id")
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_nonexistent_user_returns_404() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/admin/users/nonexistent-id")
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
