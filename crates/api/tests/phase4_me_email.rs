//! Integration tests for `POST /api/v1/auth/me/email` (Task 15, Phase 4).
//!
//! Covers the authenticated set/replace-email endpoint:
//!   - happy path sets the new email + mints a fresh verification row
//!   - the change does NOT block login (existing users aren't gated by
//!     `requires_email_verification`)
//!   - 409 `email_in_use` when the address is already claimed, and the
//!     caller's own email is left untouched (the duplicate check runs before
//!     any write)
//!   - 400 on a malformed address
//!
//! Uses the NoopMailer wired in `common::make_state` — we assert
//! storage-side state (verification row exists, user row updated) rather than
//! observing the dispatched email.

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

async fn post_authed(
    app: &axum::Router,
    uri: &str,
    token: &str,
    body: Value,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", bearer(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Register a user, peek their initial verification token via direct SQL,
/// consume it through the public `/auth/verify-email` endpoint, then log in.
/// Returns the login bearer token + the user's username.
async fn register_verify_and_login(
    app: &axum::Router,
    pool: &PgPool,
    username: &str,
    email: &str,
) -> String {
    let resp = post(
        app,
        "/api/v1/auth/register",
        json!({
            "username": username,
            "password": "password123",
            "email": email,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "register failed for {username}");

    // Peek the verification token (same approach as `verify_email_round_trip`
    // in phase4_auth.rs — no public list method on the storage trait).
    let token: String = sqlx::query_scalar(
        "SELECT token FROM email_verifications WHERE user_id = \
         (SELECT id FROM users WHERE username = $1) \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(username)
    .fetch_one(pool)
    .await
    .expect("verification row");

    let resp = post(
        app,
        "/api/v1/auth/verify-email",
        json!({ "token": token }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "verify failed for {username}");

    // Login now that the email is verified.
    let resp = post(
        app,
        "/api/v1/auth/login",
        json!({"username": username, "password": "password123"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "login failed for {username}");
    let body = body_json(resp).await;
    body["token"].as_str().unwrap().to_string()
}

/// 1. Happy path: setting a new email updates the user row + mints a fresh
///    verification row for the new address. Response carries the new email.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_email_dispatches_verification(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let token = register_verify_and_login(&app, &pool, "alice", "alice@example.com").await;

    // Set a new email.
    let resp = post_authed(
        &app,
        "/api/v1/auth/me/email",
        &token,
        json!({ "email": "alice-new@example.com" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(
        body["email"], "alice-new@example.com",
        "response should carry the new email: {body}"
    );

    // A fresh verification row exists for the new address.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_verifications \
         WHERE user_id = (SELECT id FROM users WHERE username = 'alice') \
         AND email = 'alice-new@example.com'",
    )
    .fetch_one(&pool)
    .await
    .expect("verification count");
    assert!(
        count >= 1,
        "expected a verification row for the new email; got count={count}"
    );

    // The user row picked up the new email. The handler resets
    // email_verified_at to NULL (the new address isn't verified yet) — we
    // don't assert that here, but the row at minimum must reflect the new
    // email value.
    let stored: Option<String> = sqlx::query_scalar(
        "SELECT email FROM users WHERE username = 'alice'",
    )
    .fetch_one(&pool)
    .await
    .expect("user email");
    assert_eq!(stored.as_deref(), Some("alice-new@example.com"));
}

/// 2. Setting a new (unverified) email does NOT block a subsequent login —
///    existing users aren't gated by `requires_email_verification`.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_email_does_not_block_login(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let token =
        register_verify_and_login(&app, &pool, "bob", "bob@example.com").await;

    // Change the email — the new address is pending verification.
    let resp = post_authed(
        &app,
        "/api/v1/auth/me/email",
        &token,
        json!({ "email": "bob-new@example.com" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // A fresh login must still succeed (the gate doesn't flip on for an
    // existing user who already cleared it once).
    let resp = post(
        &app,
        "/api/v1/auth/login",
        json!({"username": "bob", "password": "password123"}),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "login must still succeed after an unverified email change"
    );
}

/// 3. Duplicate detection: claiming another user's email yields 409
///    `email_in_use`, and the caller's own email is unchanged (the duplicate
///    check ran before any write).
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_email_rejects_duplicate(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let _token_a =
        register_verify_and_login(&app, &pool, "alice", "alice@example.com").await;
    let token_b =
        register_verify_and_login(&app, &pool, "bob", "bob@example.com").await;

    // Bob tries to claim Alice's email.
    let resp = post_authed(
        &app,
        "/api/v1/auth/me/email",
        &token_b,
        json!({ "email": "alice@example.com" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = body_json(resp).await;
    assert_eq!(
        body["error"]["code"], "email_in_use",
        "expected code=email_in_use, got: {body}"
    );

    // Bob's own email is untouched.
    let bob_email: Option<String> =
        sqlx::query_scalar("SELECT email FROM users WHERE username = 'bob'")
            .fetch_one(&pool)
            .await
            .expect("bob email");
    assert_eq!(
        bob_email.as_deref(),
        Some("bob@example.com"),
        "duplicate check must run before the write"
    );
}

/// 4. Malformed email → 400.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_email_rejects_invalid(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    let token =
        register_verify_and_login(&app, &pool, "carol", "carol@example.com").await;

    let resp = post_authed(
        &app,
        "/api/v1/auth/me/email",
        &token,
        json!({ "email": "not-an-email" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
