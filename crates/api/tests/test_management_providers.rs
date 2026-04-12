mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::{management, AppState};
use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn make_state(db: Arc<llm_gateway_storage::sqlite::SqliteStorage>) -> Arc<AppState> {
    Arc::new(AppState {
        storage: db.clone() as Arc<dyn Storage>,
        rate_limiter: Arc::new(RateLimiter::new(60)),
        audit_logger: Arc::new(AuditLogger::new(db as Arc<dyn Storage>)),
        jwt_secret: common::TEST_JWT_SECRET.to_string(),
    })
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

#[tokio::test]
async fn test_create_provider() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/providers")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "OpenAI",
                    "api_key": "sk-test",
                    "openai_base_url": "https://api.openai.com/v1"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["name"], "OpenAI");
    assert_eq!(body["openai_base_url"], "https://api.openai.com/v1");
}

#[tokio::test]
async fn test_create_provider_dual_protocol() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/providers")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "MiniMax",
                    "api_key": "sk-test",
                    "openai_base_url": "https://api.minimax.chat/v1",
                    "anthropic_base_url": "https://api.minimax.chat/v1/anthropic"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert!(body["openai_base_url"].is_string());
    assert!(body["anthropic_base_url"].is_string());
}

#[tokio::test]
async fn test_list_providers() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    // Create two providers
    for name in ["Provider-A", "Provider-B"] {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/providers")
                    .header("authorization", bearer_token(&admin.token))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"name": name, "api_key": "sk-test", "openai_base_url": "https://example.com"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/providers")
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
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_provider_model_lifecycle() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    // Create provider
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/providers")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "TestProvider",
                    "api_key": "sk-test",
                    "openai_base_url": "https://example.com"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let provider_id = body["id"].as_str().unwrap().to_string();

    // Add model
    let model_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/providers/{}/models", provider_id))
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "test-model",
                    "billing_type": "token",
                    "input_price": 3.0,
                    "output_price": 15.0
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(model_resp.status(), StatusCode::OK);

    // Update model
    let update_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/providers/{}/models/test-model", provider_id))
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"output_price": 20.0}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);

    // Delete model
    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/providers/{}/models/test-model", provider_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    // Delete provider
    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/providers/{}", provider_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);
}
