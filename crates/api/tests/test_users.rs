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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_list_users_admin(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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
                .uri("/api/v1/default/admin/users")
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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_list_users_without_auth_returns_401(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/admin/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_role_admin_to_user(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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
                .uri(&format!("/api/v1/default/admin/users/{}", user_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(promote_resp.status(), StatusCode::OK);

    // Now demote the original admin to a regular member.
    // Phase 1: legacy role string "user" is accepted as an alias for "member"
    // (the closest equivalent in the membership model).
    let admin_id = admin_body["user"]["id"].as_str().unwrap();
    let demote_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/default/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "member"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(demote_resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(demote_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["role"], "member");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_role_user_to_admin(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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
                .uri(&format!("/api/v1/default/admin/users/{}", user_id))
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

// TODO(Task 11/12): "cannot disable last admin" was a Phase 0 invariant
// enforced via users.role='admin'. With multi-tenant membership, the
// equivalent invariant is "cannot disable the last owner of an org",
// which needs to count MemberRole::Owner rows in the members table.
// Phase 1 deliberately drops the check; restore once /members endpoints
// exist with a count_owners-backed guard.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
#[ignore = "Phase 1 drops last-admin guard; restore with last-owner guard in /members (Task 11/12)"]
async fn test_cannot_disable_last_admin_user(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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
                .uri(&format!("/api/v1/default/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"enabled": false}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// TODO(Task 11/12): see test_cannot_disable_last_admin_user — Phase 1 drops
// the last-admin protection until a membership-aware /members endpoint
// exists with a count_owners-backed guard.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
#[ignore = "Phase 1 drops last-admin guard; restore with last-owner guard in /members (Task 11/12)"]
async fn test_cannot_demote_last_admin_user(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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
                .uri(&format!("/api/v1/default/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "user"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_delete_user(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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
                .uri(&format!("/api/v1/default/admin/users/{}", user_id))
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
                .uri("/api/v1/default/admin/users")
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

// TODO(Task 11/12): see test_cannot_disable_last_admin_user — Phase 1 drops
// the last-admin protection until a membership-aware /members endpoint
// exists with a count_owners-backed guard.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
#[ignore = "Phase 1 drops last-admin guard; restore with last-owner guard in /members (Task 11/12)"]
async fn test_cannot_delete_last_admin_user(pool: PgPool) {
    let app = build_app(common::make_state(pool));

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
                .uri(&format!("/api/v1/default/admin/users/{}", admin_id))
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_nonexistent_user_returns_404(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default/admin/users/nonexistent-id")
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_delete_nonexistent_user_returns_404(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    // Register admin
    let admin_body = register_user(&app, "admin", "password123").await;
    let admin_token = admin_body["token"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default/admin/users/nonexistent-id")
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
