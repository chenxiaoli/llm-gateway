//! Integration tests for `GET`/`PUT /api/v1/orgs/{slug}/defaults` (Phase 5).

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

async fn get(
    app: &axum::Router,
    uri: &str,
    token: &str,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", bearer(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn put(
    app: &axum::Router,
    uri: &str,
    token: &str,
    body: Value,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", bearer(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// 1. GET on an org with no defaults set → both fields null.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_defaults_initial_empty(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    // Seed an org + admin user (use the same pattern as phase2_orgs tests).
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = get(&app, &format!("/api/v1/{slug}/defaults"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], Value::Null);
    assert_eq!(body["default_budget_monthly_usd"], Value::Null);
}

/// 2. PUT both fields → GET reflects them.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_then_get_round_trip(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({
            "default_rate_limit_rpm": 100,
            "default_budget_monthly_usd": 50.00,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], 100);
    assert_eq!(body["default_budget_monthly_usd"], 50.00);

    // GET confirms persistence.
    let resp = get(&app, &format!("/api/v1/{slug}/defaults"), &token).await;
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], 100);
    assert_eq!(body["default_budget_monthly_usd"], 50.00);
}

/// 3. PUT with null clears that field.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_null_clears_field(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    // Set both.
    let _ = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": 100, "default_budget_monthly_usd": 50.0 }),
    )
    .await;

    // Clear rate limit only.
    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": null, "default_budget_monthly_usd": 50.0 }),
    )
    .await;
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], Value::Null);
    assert_eq!(body["default_budget_monthly_usd"], 50.0);
}

/// 4. Validation: rpm < 1 → 400.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_rejects_zero_rpm(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": 0, "default_budget_monthly_usd": null }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// 5. Validation: budget < 0 → 400.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_rejects_negative_budget(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": null, "default_budget_monthly_usd": -1.0 }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// 6. Non-admin member → 403 on PUT.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_forbidden_for_member(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (_admin_token, slug) = common::seed_org_with_admin(&pool, &app).await;
    let member_token = common::seed_member_in_org(&pool, &app, &slug).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &member_token,
        json!({ "default_rate_limit_rpm": 100, "default_budget_monthly_usd": null }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Member CAN still GET.
    let resp = get(&app, &format!("/api/v1/{slug}/defaults"), &member_token).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
