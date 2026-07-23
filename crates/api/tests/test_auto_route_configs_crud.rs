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

/// Platform admin can exercise the full CRUD lifecycle:
/// POST → GET → PATCH → DELETE → GET (404).
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn platform_admin_can_crud_auto_route_config(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    // POST → 200, capture id
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/auto-route-configs")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "test-pool", "config": {"model_names": ["gpt-4o"]}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["name"], "test-pool");
    let config_id = body["id"].as_str().unwrap().to_string();

    // GET → 200, name matches
    let get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/default/auto-route-configs/{}", config_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body: Value = serde_json::from_slice(
        &to_bytes(get_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(get_body["name"], "test-pool");

    // PATCH → 200, name updated
    let patch_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/default/auto-route-configs/{}", config_id))
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "renamed-pool"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patch_resp.status(), StatusCode::OK);
    let patch_body: Value = serde_json::from_slice(
        &to_bytes(patch_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(patch_body["name"], "renamed-pool");

    // DELETE → 204
    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/default/auto-route-configs/{}", config_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    // GET (again) → 404
    let get_again_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/default/auto-route-configs/{}", config_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_again_resp.status(), StatusCode::NOT_FOUND);
}

/// Regular members (not platform_admin) cannot create configs — 403.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn non_admin_create_returns_403(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    common::seed_member(&pool, "member-1", None).await;
    let app = build_app(common::make_state(pool));
    let member = common::make_user_token("member-1");

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/auto-route-configs")
                .header("authorization", bearer_token(&member.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "test-pool", "config": {"model_names": ["gpt-4o"]}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// Non-creator non-admin GET → 404 (we hide existence).
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn non_admin_get_returns_404_for_others_configs(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    common::seed_member(&pool, "member-1", None).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    // Admin POSTs a config
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/auto-route-configs")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "admin-pool", "config": {"model_names": ["gpt-4o"]}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let config_id = body["id"].as_str().unwrap().to_string();

    // Regular member GETs that config id → 404
    let member = common::make_user_token("member-1");
    let get_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/default/auto-route-configs/{}", config_id))
                .header("authorization", bearer_token(&member.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}
