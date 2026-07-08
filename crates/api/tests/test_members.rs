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

/// Insert a user row + member row in org_default. Returns the user id.
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

/// Insert org_other with slug "other".
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

/// Make a plain-member token (no platform role) for the given user id.
fn member_token(user_id: &str) -> String {
    llm_gateway_auth::create_jwt(user_id, common::TEST_ORG, None, common::TEST_JWT_SECRET).unwrap()
}

/// Insert a user that exists but has no membership in any org (for invite tests).
async fn seed_user_no_membership(pool: &PgPool, user_id: &str, username: &str) {
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, NULL, true, NOW(), NOW())
           ON CONFLICT (id) DO NOTHING"#,
    )
    .bind(user_id)
    .bind(username)
    .execute(pool)
    .await
    .expect("seed user no membership");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_members_returns_only_current_org_members(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_other_org(&pool).await;
    // Two members in org_default (admin-1 already seeded as owner).
    seed_default_member(&pool, "u-alice", "alice", "member").await;
    seed_default_member(&pool, "u-bob", "bob", "admin").await;

    // One member in org_other only — must NOT appear.
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-carol', 'carol', 'x', NULL, $1, true, NOW(), NOW())
           ON CONFLICT (id) DO NOTHING"#,
    )
    .bind(OTHER_ORG_ID)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ('u-carol', $1, 'member', 'admin-1', NOW())
           ON CONFLICT (user_id, org_id) DO NOTHING"#,
    )
    .bind(OTHER_ORG_ID)
    .execute(&pool)
    .await
    .unwrap();

    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let arr = body.as_array().expect("response is a JSON array");
    let ids: Vec<&str> = arr
        .iter()
        .map(|m| m["user_id"].as_str().unwrap())
        .collect();
    // Should include admin-1, alice, bob — NOT carol.
    assert!(ids.contains(&"admin-1"), "admin-1 must be listed");
    assert!(ids.contains(&"u-alice"), "alice must be listed");
    assert!(ids.contains(&"u-bob"), "bob must be listed");
    assert!(
        !ids.contains(&"u-carol"),
        "carol (member of org_other) must NOT be listed"
    );
    // Role serialization: lowercase string.
    let admin_row = arr
        .iter()
        .find(|m| m["user_id"] == "admin-1")
        .unwrap();
    assert_eq!(admin_row["role"], "owner");
    assert_eq!(admin_row["username"], "admin");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_members_forbidden_for_plain_member(pool: PgPool) {
    seed_default_member(&pool, "u-plain", "plain", "member").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-plain");

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&tok))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// =====================================================================
// Task 2: invite_member
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn invite_member_adds_existing_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_user_no_membership(&pool, "u-dave", "dave").await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "dave", "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["username"], "dave");
    assert_eq!(body["role"], "member");
    assert_eq!(body["user_id"], "u-dave");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn invite_member_404_for_unknown_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "ghost", "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn invite_member_409_if_already_member(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-eve", "eve", "member").await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "eve", "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
