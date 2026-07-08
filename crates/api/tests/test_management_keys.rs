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

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_create_key(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/keys")
                .header("authorization", bearer_token(&admin.token))
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
    assert!(body["key"].is_string());
    assert_eq!(body["enabled"], true);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_list_keys(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/keys")
                .header("authorization", bearer_token(&admin.token))
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
                .uri("/api/v1/default/keys")
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
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_unauthorized_access(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/keys")
                .header("authorization", "Bearer invalid-jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Pre-Phase-2 paths (no org slug in the URL) must return 410 Gone with a
/// pointer to the new path, not a 404 or fall-through to the SPA.
///
/// Covers both shapes:
/// - **Multi-segment** (e.g. `/api/v1/admin/users`): unambiguous, falls
///   through to the legacy catch-all via `.nest("/api/v1", legacy_router())`.
/// - **Single-segment** (e.g. `/api/v1/keys`): registered as literal 410
///   routes on the outer router so Axum's matchit doesn't capture them as
///   `{org_slug}` (which would run auth_layer → 401 first). See
///   `management_router` in `crates/api/src/management/mod.rs`.
///
/// Both unauthenticated AND authenticated requests must return 410 for the
/// single-segment roots — an authenticated user hitting a legacy path should
/// see the "moved" pointer, not a 401/403 that implies an authz failure.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn legacy_path_returns_410_gone(pool: PgPool) {
    // --- Single-segment legacy root, authenticated (must STILL be 410) ---
    //
    // Seed the admin first because the authenticated checks below need it;
    // the unauthenticated checks don't, but seeding here is harmless and
    // keeps the test self-contained.
    common::seed_admin_user(&pool).await;
    let admin = common::make_admin_token();
    let app = build_app(common::make_state(pool));

    // --- Multi-segment legacy path ---
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::GONE);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"], "gone");
    assert_eq!(body["new_path"], "/api/v1/{org_slug}/admin/users");

    // --- Single-segment legacy root, unauthenticated ---
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/keys")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::GONE);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"], "gone");

    // --- Single-segment legacy sub-path, unauthenticated ---
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/keys/abc-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::GONE);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"], "gone");

    // --- Single-segment legacy root, authenticated (must STILL be 410) ---
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/keys")
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::GONE);

    // --- Same for model-fallbacks and usage ---
    for path in ["/api/v1/model-fallbacks", "/api/v1/usage"] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(path)
                    .header("authorization", bearer_token(&admin.token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::GONE);
    }
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_key(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/keys")
                .header("authorization", bearer_token(&admin.token))
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

    let update_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/default/keys/{}", key_id))
                .header("authorization", bearer_token(&admin.token))
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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_delete_key(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/keys")
                .header("authorization", bearer_token(&admin.token))
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

    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/default/keys/{}", key_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let get_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/default/keys/{}", key_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_register_first_user_is_admin(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "admin", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    // Phase 1 multi-tenant: "first user is admin" now means (a) the user has
    // platform_role=platform_admin and (b) their auto-membership in the
    // default org has role=owner.
    assert_eq!(body["user"]["platform_role"], "platform_admin");
    assert_eq!(body["current_org"]["role"], "owner");
    assert!(body["token"].is_string());
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_login_and_me(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "testuser", "password": "password123"}).to_string()))
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
}
