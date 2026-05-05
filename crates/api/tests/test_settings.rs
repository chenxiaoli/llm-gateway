mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::{management, AppState};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_get_settings_default_allow_registration_true(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/settings")
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
    assert_eq!(body["allow_registration"], true);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_get_settings_without_admin_auth_returns_401(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_settings_disable_registration(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/admin/settings")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"allow_registration": false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["allow_registration"], false);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_get_settings_after_update(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    // Update settings to disable registration
    app.clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/admin/settings")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"allow_registration": false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Get settings and verify the new value
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/settings")
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
    assert_eq!(body["allow_registration"], false);
}
