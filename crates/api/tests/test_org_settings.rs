mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

const OTHER_ORG_ID: &str = "org_other";
const OTHER_ORG_SLUG: &str = "other";

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

/// Insert a user row + member row in org_default with the given role.
async fn seed_default_member(
    pool: &PgPool,
    user_id: &str,
    username: &str,
    role: &str,
) -> String {
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, $3, true, NOW(), NOW())
           ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(common::TEST_ORG)
    .execute(pool)
    .await
    .expect("seed user");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ($1, $2, $3, 'admin-1', NOW())
           ON CONFLICT (user_id, org_id) DO UPDATE SET role = EXCLUDED.role"#,
    )
    .bind(user_id)
    .bind(common::TEST_ORG)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed member");
    user_id.to_string()
}

/// Insert org_other with slug "other" (for duplicate-slug tests).
async fn seed_other_org(pool: &PgPool) {
    sqlx::query(
        r#"INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
           VALUES ($1, $2, 'Other Org', NULL, NOW(), NOW())"#,
    )
    .bind(OTHER_ORG_ID)
    .bind(OTHER_ORG_SLUG)
    .execute(pool)
    .await
    .expect("seed org_other");
}

/// Make a plain-member JWT (no platform role) for the given user id.
fn member_token(user_id: &str) -> String {
    llm_gateway_auth::create_jwt(user_id, common::TEST_ORG, None, common::TEST_JWT_SECRET).unwrap()
}

// =====================================================================
// Task 5: PATCH /{org_slug} — update name / slug
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_renames_name(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "New Name"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "New Name");
    // Slug should be unchanged.
    assert_eq!(body["slug"], "default");
    // Role is echoed back from the caller's context.
    assert_eq!(body["role"], "owner");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_changes_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"slug": "new-slug"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["slug"], "new-slug");
    // Name should be unchanged.
    assert_eq!(body["name"], "Default Org");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_rejects_invalid_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"slug": "UPPER CASE"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_rejects_duplicate_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_other_org(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"slug": "other"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_forbidden_for_member(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-plain", "plain", "member").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-plain");

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&tok))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "Hacked"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_admin_role_can_patch(pool: PgPool) {
    // Admin (not owner) should be allowed by can_manage_org_settings.
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-admin", "adminuser", "admin").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-admin");

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&tok))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "Admin Rename"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Admin Rename");
}
