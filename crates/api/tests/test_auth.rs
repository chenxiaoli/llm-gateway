mod common;

use common::MockChannelRegistry;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::{AppState, SettlementTrigger, SystemInfo};
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
        system_info: SystemInfo {
            server_bind_address: "0.0.0.0:8080".to_string(),
            database_driver: "sqlite".to_string(),
            rate_limit_window_secs: 60,
            rate_limit_flush_interval_secs: 30,
            upstream_timeout_secs: 30,
            audit_retention_days: Some(90),
        },
    })
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

#[tokio::test]
async fn test_register_first_user_becomes_admin() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "admin", "password": "password123"}).to_string(),
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
    assert_eq!(body["user"]["role"], "admin");
    assert!(body["token"].is_string());
}

#[tokio::test]
async fn test_register_second_user_becomes_regular() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register first user (admin)
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "admin", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Register second user
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "regular", "password": "password123"}).to_string(),
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
    assert_eq!(body["user"]["role"], "user");
    assert!(body["token"].is_string());
}

#[tokio::test]
async fn test_login_with_valid_credentials() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register a user first
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Login
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
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
    assert!(body["token"].is_string());
    assert_eq!(body["user"]["username"], "testuser");
}

#[tokio::test]
async fn test_login_with_wrong_password_returns_401() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register a user first
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Login with wrong password
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "wrongpassword"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_with_nonexistent_user_returns_401() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "nonexistent", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_register_duplicate_username_returns_400() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register first user
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try to register with same username
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "differentpass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_auth_config_returns_allow_registration_true() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/config")
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
async fn test_auth_me_returns_user_info_when_authenticated() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(register_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let token = body["token"].as_str().unwrap();

    // Get me
    let me_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header("authorization", bearer_token(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(me_resp.status(), StatusCode::OK);
    let me_body: Value = serde_json::from_slice(
        &to_bytes(me_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(me_body["username"], "testuser");
    assert!(me_body["id"].is_string());
    assert!(me_body["role"].is_string());
}

#[tokio::test]
async fn test_auth_me_returns_401_when_not_authenticated() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_returns_new_tokens() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register a user first
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body: Value = serde_json::from_slice(
        &to_bytes(register_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let _original_token = body["token"].as_str().unwrap().to_string();
    let original_refresh_token = body["refresh_token"].as_str().unwrap().to_string();

    // Refresh
    let refresh_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"refresh_token": original_refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(refresh_resp.status(), StatusCode::OK);
    let refresh_body: Value = serde_json::from_slice(
        &to_bytes(refresh_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();

    // Should get new tokens (access token works for /me)
    let new_token = refresh_body["token"].as_str().unwrap().to_string();
    let new_refresh_token = refresh_body["refresh_token"].as_str().unwrap().to_string();
    assert!(new_token.len() > 0);
    assert!(new_refresh_token.len() > 0);

    // New access token should work for /me
    let me_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header("authorization", bearer_token(&new_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me_resp.status(), StatusCode::OK);

    // Old refresh token should no longer work (rotation)
    let old_refresh_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"refresh_token": original_refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_refresh_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_with_invalid_token_returns_401() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"refresh_token": "invalid-token-here"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_success() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(register_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let token = body["token"].as_str().unwrap().to_string();

    // Change password
    let change_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/change-password")
                .header("content-type", "application/json")
                .header("authorization", bearer_token(&token))
                .body(Body::from(
                    json!({"current_password": "password123", "new_password": "newpass456"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(change_resp.status(), StatusCode::OK);

    // Login with new password
    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "newpass456"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK);

    // Old password no longer works
    let old_login_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_login_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_wrong_current_returns_400() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(register_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let token = body["token"].as_str().unwrap().to_string();

    // Try to change with wrong current password
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/change-password")
                .header("content-type", "application/json")
                .header("authorization", bearer_token(&token))
                .body(Body::from(
                    json!({"current_password": "wrongpassword", "new_password": "newpass456"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_change_password_unauthenticated_returns_401() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/change-password")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"current_password": "password123", "new_password": "newpass456"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_register_short_password_returns_400() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "short"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_invalid_username_returns_400() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "invalid user!", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_refresh_with_revoked_token_returns_401() {
    let db = common::setup_test_db().await;
    let state = make_state(db.clone());
    let app = build_app(state);

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body: Value = serde_json::from_slice(
        &to_bytes(register_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let refresh_token = body["refresh_token"].as_str().unwrap().to_string();

    // Revoke the refresh token by clearing it in the DB directly
    let user = db.get_user_by_username("testuser").await.unwrap().unwrap();
    let mut revoked_user = user.clone();
    revoked_user.refresh_token = None;
    revoked_user.updated_at = chrono::Utc::now();
    db.update_user(&revoked_user).await.unwrap();

    // Try to use the old refresh token
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"refresh_token": refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
