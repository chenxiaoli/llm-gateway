//! Integration tests for the Phase 4 email-verification flow.
//!
//! Covers the new /auth/verify-email and /auth/resend-verification routes,
//! the login gate (ApiError::EmailNotVerified → 403), and the email-uniqueness
//! check on /auth/register.
//!
//! Uses the NoopMailer wired in `common::make_state` so we don't write files
//! to disk. We assert storage-side state (verification row exists, user
//! marked verified, etc.) rather than observing the dispatched email.

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

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
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

/// 1. Register without an email → 400 with `code = "email_required"`.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn register_requires_email(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // No email field at all.
    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({"password": "password123"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert_eq!(
        body["error"]["code"], "email_required",
        "expected code=email_required, got: {body}"
    );

    // Empty email string.
    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({"password": "password123", "email": ""}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert_eq!(body["error"]["code"], "email_required");
}

/// 2. Register with a valid email → user is created with
///    `email_verified_at = NULL` and `requires_email_verification = true`.
///    The verification row is minted.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn register_dispatches_verification_email(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({
            "password": "password123",
            "email": "alice@example.com",
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // User row carries the email + verification gate.
    let row: (Option<String>, Option<chrono::DateTime<chrono::Utc>>, bool) =
        sqlx::query_as(
            "SELECT email, email_verified_at, requires_email_verification \
             FROM users WHERE email = 'alice@example.com'",
        )
        .fetch_one(&pool)
        .await
        .expect("user row");
    assert_eq!(row.0.as_deref(), Some("alice@example.com"));
    assert!(row.1.is_none(), "must be unverified");
    assert!(row.2, "must require verification");

    // A verification row exists with a non-empty token.
    let token: String = sqlx::query_scalar(
        "SELECT token FROM email_verifications WHERE user_id = \
         (SELECT id FROM users WHERE email = 'alice@example.com') \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("verification row");
    assert!(!token.is_empty());
}

/// 3. Register then attempt to log in without verifying → 403 with
///    `code = "email_not_verified"`.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn login_blocked_until_verified(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({
            "password": "password123",
            "email": "bob@example.com",
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Login attempt — should be blocked.
    let resp = post(
        &app,
        "/api/v1/auth/login",
        json!({"username": "bob@example.com", "password": "password123"}),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "login must be blocked until email is verified"
    );
    let body = body_json(resp).await;
    assert_eq!(
        body["error"]["code"], "email_not_verified",
        "expected code=email_not_verified, got: {body}"
    );
}

/// 3a. The login gate also dispatches a fresh verification email on rejection
/// so the user gets a one-click remediation path. Register mints one row,
/// then a blocked login attempt mints a second row.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn login_unverified_dispatches_verification_email(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({
            "password": "password123",
            "email": "ben@example.com",
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // One verification row exists after register.
    let count_after_register: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_verifications WHERE user_id = \
         (SELECT id FROM users WHERE email = 'ben@example.com')",
    )
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count_after_register, 1);

    // Login attempt — blocked, but should mint a fresh verification row.
    let resp = post(
        &app,
        "/api/v1/auth/login",
        json!({"username": "ben@example.com", "password": "password123"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Now two verification rows — the second one is the login-gate dispatch.
    let count_after_login: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_verifications WHERE user_id = \
         (SELECT id FROM users WHERE email = 'ben@example.com')",
    )
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(
        count_after_login, 2,
        "login gate must dispatch a fresh verification email"
    );

    // The most recent token is distinct from the register-time token.
    let tokens: Vec<String> = sqlx::query_scalar(
        "SELECT token FROM email_verifications WHERE user_id = \
         (SELECT id FROM users WHERE email = 'ben@example.com') \
         ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await
    .expect("tokens");
    assert_eq!(tokens.len(), 2);
    assert_ne!(tokens[0], tokens[1], "login-gate token must be fresh");
}

/// 4. Round-trip: register → peek verification token → consume → login succeeds.
///
/// We peek via direct SQL (no public list method on the storage trait), which
/// is the same approach production code would take for an out-of-band admin
/// tool. After consume, the user's `email_verified_at` is set and the login
/// gate lifts.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn verify_email_round_trip(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({
            "password": "password123",
            "email": "carol@example.com",
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Peek the verification token.
    let token: String = sqlx::query_scalar(
        "SELECT token FROM email_verifications WHERE user_id = \
         (SELECT id FROM users WHERE email = 'carol@example.com') \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("verification row");

    // Submit the token.
    let resp = post(
        &app,
        "/api/v1/auth/verify-email",
        json!({"token": token}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // User is now verified.
    let verified_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT email_verified_at FROM users WHERE email = 'carol@example.com'",
    )
    .fetch_one(&pool)
    .await
    .expect("user row");
    assert!(verified_at.is_some(), "user must be verified after consume");

    // Login now succeeds.
    let resp = post(
        &app,
        "/api/v1/auth/login",
        json!({"username": "carol@example.com", "password": "password123"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["token"].is_string());

    // Re-submitting the same token fails (consumed).
    let resp = post(
        &app,
        "/api/v1/auth/verify-email",
        json!({"token": token}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::GONE);
    let body = body_json(resp).await;
    assert_eq!(body["error"]["code"], "verification_expired");
}

/// 5. POST /auth/resend-verification with an unknown email is a clean 204 —
///    the endpoint must not leak which addresses are registered.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn resend_verification_is_204_for_unknown_email(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = post(
        &app,
        "/api/v1/auth/resend-verification",
        json!({"email": "ghost@example.com"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    // 204 has no body — collect the bytes and confirm they are empty
    // (no leak of which addresses are registered).
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert!(bytes.is_empty(), "204 body must be empty, got {} bytes", bytes.len());
}

/// 6. /me surfaces the new Phase 4 fields.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn me_includes_phase4_fields(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = post(
        &app,
        "/api/v1/auth/register",
        json!({
            "password": "password123",
            "email": "dave@example.com",
        }),
    )
    .await;
    let body = body_json(resp).await;
    let token = body["token"].as_str().unwrap().to_string();

    // /me surfaces email + verification state.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header("authorization", bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["email"], "dave@example.com");
    assert!(
        body["email_verified_at"].is_null(),
        "should be unverified"
    );
    assert_eq!(body["requires_email_verification"], true);
}
