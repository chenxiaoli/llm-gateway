mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Utc};
use llm_gateway_api::{management, AppState};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

async fn seed_audit_log(pool: &PgPool, id: &str, created_at: DateTime<Utc>) {
    sqlx::query(
        "INSERT INTO api_keys (id, name, key_hash, created_at, updated_at) \
         VALUES ('key-1', 'k', 'hash', NOW(), NOW()) \
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO audit_logs \
         (id, key_id, model_name, provider_id, channel_id, protocol, stream, \
          request_body, response_body, status_code, latency_ms, input_tokens, output_tokens, created_at) \
         VALUES ($1, 'key-1', 'm', 'p', NULL, 'openai', false, '', '', 200, 1, 0, 0, $2)",
    )
    .bind(id)
    .bind(created_at)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_admin_logs_with_since_and_until_returns_200(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let admin = common::make_admin_token();

    seed_audit_log(
        &pool,
        "log-in-range",
        "2026-07-12T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
    )
    .await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/logs?page=1&page_size=20&since=2026-07-10T00:00:00Z&until=2026-07-15T00:00:00Z")
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert_eq!(status, StatusCode::OK, "body was: {body_str}");
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["id"], "log-in-range");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_admin_logs_with_only_since_returns_200(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let admin = common::make_admin_token();

    seed_audit_log(&pool, "log-since", "2026-07-12T00:00:00Z".parse::<DateTime<Utc>>().unwrap()).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/logs?page=1&page_size=20&since=2026-07-10T00:00:00Z")
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
    assert_eq!(body["total"], 1);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_admin_logs_with_only_until_returns_200(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let admin = common::make_admin_token();

    seed_audit_log(&pool, "log-until", "2026-07-12T00:00:00Z".parse::<DateTime<Utc>>().unwrap()).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/logs?page=1&page_size=20&until=2026-07-15T00:00:00Z")
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
    assert_eq!(body["total"], 1);
}
