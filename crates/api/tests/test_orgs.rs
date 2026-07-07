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

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_org_returns_details_for_member(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["slug"], "default");
    assert_eq!(body["name"], "Default Org");
    // admin-1 is owner of org_default
    assert_eq!(body["role"], "owner");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_org_returns_403_for_non_member(pool: PgPool) {
    // Insert a user with no member row in org_default
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('outsider-1', 'outsider', 'x', NULL, $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .unwrap();

    let app = build_app(common::make_state(pool));
    let token =
        llm_gateway_auth::create_jwt("outsider-1", common::TEST_ORG, None, common::TEST_JWT_SECRET)
            .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default")
                .header("authorization", bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // membership_layer rejects with 403 before get_org runs
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_org_returns_404_for_unknown_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ghost-org")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // org_resolve_layer rejects with 404
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn create_org_works_and_makes_caller_owner(pool: PgPool) {
    common::seed_admin_user(&pool).await;

    // Create a fresh user (not admin) to be the org creator
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('creator-1', 'creator', 'x', NULL, $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .unwrap();

    let app = build_app(common::make_state(pool));
    let token =
        llm_gateway_auth::create_jwt("creator-1", common::TEST_ORG, None, common::TEST_JWT_SECRET)
            .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orgs")
                .header("authorization", bearer(&token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"slug": "acme-inc", "name": "Acme Inc"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["slug"], "acme-inc");
    assert_eq!(body["name"], "Acme Inc");
    assert_eq!(body["role"], "owner");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn create_org_rejects_invalid_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orgs")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"slug": "ACME INC!!", "name": "Acme"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn create_org_rejects_duplicate_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orgs")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"slug": "default", "name": "Default"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
