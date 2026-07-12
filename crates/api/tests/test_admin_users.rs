mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::{management, AppState};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn bearer(token: &str) -> String { format!("Bearer {}", token) }

fn build(state: Arc<AppState>) -> axum::Router {
    management::management_router(state.clone()).with_state(state)
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_platform_users_requires_platform_admin(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let user = common::make_user_token("non-admin-user");

    let resp = app.oneshot(
        Request::builder()
            .method("GET")
            .uri("/api/v1/admin/platform-users")
            .header("authorization", bearer(&user.token))
            .body(Body::empty()).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_platform_users_returns_admins(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("GET")
            .uri("/api/v1/admin/platform-users")
            .header("authorization", bearer(&admin.token))
            .body(Body::empty()).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let admins = body["admins"].as_array().expect("admins array");
    assert_eq!(admins.len(), 1);
    assert_eq!(admins[0]["username"], "admin");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_platform_role_grants_to_non_admin(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-target', 'target', 'x', NULL, $1, true, NOW(), NOW())"#,
    ).bind(common::TEST_ORG).execute(&pool).await.unwrap();

    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri("/api/v1/admin/users/u-target/platform-role")
            .header("authorization", bearer(&admin.token))
            .header("content-type", "application/json")
            .body(Body::from(json!({"platform_role": "platform_admin"}).to_string())).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_platform_role_returns_409_on_last_admin_self_demote(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri("/api/v1/admin/users/admin-1/platform-role")
            .header("authorization", bearer(&admin.token))
            .header("content-type", "application/json")
            .body(Body::from(json!({"platform_role": null}).to_string())).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["error"]["code"], "last_platform_admin");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_platform_role_returns_404_for_missing_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri("/api/v1/admin/users/nonexistent/platform-role")
            .header("authorization", bearer(&admin.token))
            .header("content-type", "application/json")
            .body(Body::from(json!({"platform_role": "platform_admin"}).to_string())).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
