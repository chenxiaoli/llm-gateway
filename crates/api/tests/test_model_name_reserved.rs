mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router(state.clone()).with_state(state)
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

/// The model name "auto" is reserved for the model=auto routing feature
/// (the proxy intercepts `model: "auto"` and rewrites it to a real model
/// from the key's auto_route_config). Letting an admin create a catalog
/// entry literally named "auto" would shadow that route and break the
/// proxy rewrite, so the create handler rejects it with a typed 400.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn create_model_named_auto_is_rejected(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/admin/models")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "auto"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8_lossy(&body_bytes);
    assert!(
        body.contains("model_name_reserved"),
        "expected body to contain 'model_name_reserved', got: {}",
        body
    );
}

/// Case-insensitive: "Auto", "AUTO", etc. must also be rejected, since the
/// proxy matches the literal string "auto" and a differently-cased catalog
/// entry would still collide after normalization.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn create_model_named_auto_case_insensitive(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    for name in ["Auto", "AUTO", "aUtO"] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/default/admin/models")
                    .header("authorization", bearer_token(&admin.token))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"name": name}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "name {:?} should be rejected",
            name
        );
    }
}

/// Sanity check: a normal, non-reserved model name still creates successfully.
/// Guards against the guard accidentally being too broad.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn create_model_with_normal_name_still_works(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/admin/models")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "gpt-4o"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
