mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use llm_gateway_storage::Storage;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router(state.clone()).with_state(state)
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_register_first_user_becomes_admin(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "admin", "password": "password123", "email": "admin@example.com"}).to_string(),
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
    // Phase 3: first user is platform_admin BUT in limbo (no org yet).
    // They must complete the onboarding wizard to get a real org.
    assert_eq!(body["user"]["platform_role"], "platform_admin");
    assert!(
        body["current_org"].is_null(),
        "expected current_org null, got {}",
        body["current_org"]
    );
    assert!(body["orgs"].as_array().unwrap().is_empty());
    assert!(body["token"].is_string());
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_register_second_user_becomes_regular(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register first user (admin)
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "admin", "password": "password123", "email": "admin@example.com"}).to_string(),
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
                    json!({"username": "regular", "password": "password123", "email": "regular@example.com"}).to_string(),
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
    // Phase 3: second user is also in limbo.
    assert_eq!(body["user"]["platform_role"], serde_json::Value::Null);
    assert!(body["current_org"].is_null());
    assert!(body["orgs"].as_array().unwrap().is_empty());
    assert!(body["token"].is_string());
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_login_with_valid_credentials(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    // Register a user first
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Phase 4: bypass the verification gate (this test isn't about that).
    common::mark_user_verified(&pool, "testuser").await;

    // Login
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_login_with_wrong_password_returns_401(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register a user first
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_login_with_nonexistent_user_returns_401(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_register_duplicate_username_returns_400(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register first user
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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
                    json!({"username": "testuser", "password": "differentpass", "email": "testuser2@example.com"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_auth_config_returns_allow_registration_true(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_auth_me_returns_user_info_when_authenticated(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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
    // Phase 3: limbo user has no current_org and no memberships.
    assert!(me_body["current_org"].is_null());
    assert!(me_body["orgs"].as_array().unwrap().is_empty());
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_auth_me_returns_401_when_not_authenticated(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_auth_me_returns_null_current_org_when_no_memberships(pool: PgPool) {
    // A user who has self-left their last org should see current_org: null
    // + orgs: [] from /auth/me, not a 500. This is the recovery path the
    // frontend uses after self-leave to bounce to /login.
    common::seed_admin_user(&pool).await;
    // Wipe the admin's only membership — leaves the user row intact but
    // membership-less, simulating the post-self-leave state.
    sqlx::query("DELETE FROM members WHERE user_id = 'admin-1'")
        .execute(&pool)
        .await
        .unwrap();

    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
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
    assert_eq!(body["username"], "admin");
    // current_org must be null (not absent, not an empty object).
    assert!(
        body["current_org"].is_null(),
        "expected current_org to be null, got: {}",
        body["current_org"]
    );
    assert_eq!(body["orgs"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_refresh_returns_new_tokens(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register a user first
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_refresh_with_invalid_token_returns_401(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_change_password_success(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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
    // Phase 4: bypass the verification gate so the post-change-password
    // login attempt succeeds.
    common::mark_user_verified(&pool, "testuser").await;

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
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_login_resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_change_password_wrong_current_returns_400(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_change_password_unauthenticated_returns_401(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_register_short_password_returns_400(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_register_invalid_username_returns_400(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_refresh_with_revoked_token_returns_401(pool: PgPool) {
    let db = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool.clone());
    let app = build_app(common::make_state(pool));

    // Register a user
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "testuser", "password": "password123", "email": "testuser@example.com"}).to_string(),
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
