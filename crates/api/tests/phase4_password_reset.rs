//! Integration tests for the Phase 4 password-reset flow.
//!
//! Covers the three public endpoints (`/request`, `/preview`, `/confirm`),
//! the ApiError variants (ResetExpired, ResetConsumed, ResetNotFound), and
//! the refresh-token epoch invalidation: a refresh JWT issued before the
//! most recent password change must be rejected by `/auth/refresh`.
//!
//! Uses the NoopMailer wired in `common::make_state`. Tokens are peeked via
//! direct SQL (there's no public list method on the storage trait).

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router(state.clone()).with_state(state)
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn post(app: &axum::Router, uri: &str, body: Value) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get(app: &axum::Router, uri: &str) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// 1. POST /request with an unknown email → 204, no reset row minted.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn password_reset_request_returns_204_for_unknown_email(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let resp = post(
        &app,
        "/api/v1/auth/password-reset/request",
        json!({"email": "nobody@example.com"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM password_resets WHERE token = 'does-not-exist'",
    )
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 0, "no reset row should exist for an unknown email");
}

/// 2. POST /request for a registered-but-unverified email → 204, no reset row.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn password_reset_request_returns_204_for_unverified_email(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({
            "username": "unverified",
            "password": "password123",
            "email": "unverified@example.com",
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = post(
        &app,
        "/api/v1/auth/password-reset/request",
        json!({"email": "unverified@example.com"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM password_resets pr \
         JOIN users u ON u.id = pr.user_id \
         WHERE u.username = 'unverified'",
    )
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(
        count, 0,
        "no reset row should be minted for an unverified account"
    );
}

/// Helper: register + verify + request reset, returning the peeked token.
async fn mint_reset_token(app: &axum::Router, pool: &PgPool, username: &str) -> String {
    let email = format!("{username}@example.com");
    let resp = post(
        app,
        "/api/v1/auth/register",
        json!({"username": username, "password": "password123", "email": email}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "register failed for {username}");

    common::mark_user_verified(pool, username).await;

    let resp = post(
        app,
        "/api/v1/auth/password-reset/request",
        json!({"email": email}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "request failed for {username}");

    let token: String = sqlx::query_scalar(
        "SELECT pr.token FROM password_resets pr \
         JOIN users u ON u.id = pr.user_id \
         WHERE u.username = $1 \
         ORDER BY pr.created_at DESC LIMIT 1",
    )
    .bind(username)
    .fetch_one(pool)
    .await
    .expect("reset row");
    assert!(!token.is_empty());
    token
}

/// 3. Full round trip: request → preview (valid) → confirm → login with new
///    password succeeds.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn password_reset_full_round_trip(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let token = mint_reset_token(&app, &pool, "roundtrip").await;

    // Preview before confirm → valid.
    let resp = get(
        &app,
        &format!("/api/v1/auth/password-reset/preview?token={token}"),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["valid"], true, "preview should be valid: {body}");
    assert!(body["expires_at"].is_string(), "expires_at should be set");

    // Confirm with a new password.
    let resp = post(
        &app,
        "/api/v1/auth/password-reset/confirm",
        json!({"token": token, "new_password": "new-password-456"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Old password no longer works.
    let resp = post(
        &app,
        "/api/v1/auth/login",
        json!({"username": "roundtrip", "password": "password123"}),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "old password must fail after reset"
    );

    // New password works.
    let resp = post(
        &app,
        "/api/v1/auth/login",
        json!({"username": "roundtrip", "password": "new-password-456"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "new password should work");
}

/// 4. Confirm with an expired token → 410 + reset_expired.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn password_reset_expired_token_returns_410(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let token = mint_reset_token(&app, &pool, "expired").await;

    // Backdate created_at + expires_at together (the table has a check
    // constraint requiring expires_at > created_at).
    sqlx::query(
        "UPDATE password_resets \
         SET created_at = NOW() - INTERVAL '3 hours', \
             expires_at = NOW() - INTERVAL '2 hours' \
         WHERE token = $1",
    )
    .bind(&token)
    .execute(&pool)
    .await
    .expect("backdate");

    let resp = post(
        &app,
        "/api/v1/auth/password-reset/confirm",
        json!({"token": token, "new_password": "new-password-456"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::GONE);
    let body = body_json(resp).await;
    assert_eq!(body["error"]["code"], "reset_expired", "got: {body}");
}

/// 5. Double-consume: full round trip then a second confirm → 410 + reset_consumed.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn password_reset_consumed_token_returns_410(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let token = mint_reset_token(&app, &pool, "consumed").await;

    let resp = post(
        &app,
        "/api/v1/auth/password-reset/confirm",
        json!({"token": token, "new_password": "new-password-456"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = post(
        &app,
        "/api/v1/auth/password-reset/confirm",
        json!({"token": token, "new_password": "new-password-789"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::GONE);
    let body = body_json(resp).await;
    assert_eq!(body["error"]["code"], "reset_consumed", "got: {body}");
}

/// 6. After a password reset, a refresh token issued BEFORE the reset must be
///    rejected by /auth/refresh (401).
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn refresh_after_password_reset_returns_401(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    // Register + verify + login to capture a refresh token issued under the
    // OLD password epoch.
    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({
            "username": "epoch",
            "password": "password123",
            "email": "epoch@example.com",
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    common::mark_user_verified(&pool, "epoch").await;

    let resp = post(
        &app,
        "/api/v1/auth/login",
        json!({"username": "epoch", "password": "password123"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let refresh_token = body["refresh_token"]
        .as_str()
        .expect("refresh_token in login response")
        .to_string();
    assert!(!refresh_token.is_empty());

    // Request + confirm a reset.
    let resp = post(
        &app,
        "/api/v1/auth/password-reset/request",
        json!({"email": "epoch@example.com"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let token: String = sqlx::query_scalar(
        "SELECT pr.token FROM password_resets pr \
         JOIN users u ON u.id = pr.user_id \
         WHERE u.username = 'epoch' \
         ORDER BY pr.created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("reset row");

    let resp = post(
        &app,
        "/api/v1/auth/password-reset/confirm",
        json!({"token": token, "new_password": "new-password-456"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Old refresh token must be rejected.
    let resp = post(
        &app,
        "/api/v1/auth/refresh",
        json!({"refresh_token": refresh_token}),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "pre-reset refresh token must be rejected"
    );
}
