//! Integration tests for proxy budget enforcement (Phase 6).
//!
//! Verifies the resolution order:
//!   effective_budget = api_key.budget_monthly ?? org.default_budget_monthly_usd ?? None
//! and that exceeding returns 429 with budget_exceeded body.

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::Router;
use llm_gateway_api::management;
use llm_gateway_api::proxy;
use llm_gateway_api::AppState;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

/// Build the FULL app (management + proxy routes), mirroring the assembly in
/// `crates/gateway/src/main.rs`. The management router alone does not include
/// `/v1/chat/completions`, so we layer the proxy routes onto the same state.
///
/// Same shape as `phase5_enforcement::build_full_app`; duplicated here so the
/// phase6 test stays self-contained (a sibling test module, not a shared
/// helper) per the project's per-phase test convention.
fn build_full_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(proxy::proxy_with_protocol))
        .route("/v1/messages", post(proxy::messages))
        .route("/v1/responses", post(proxy::responses))
        .merge(management::management_router(state.clone()))
        .with_state(state)
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

async fn chat_completion(app: &Router, api_key: &str) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .header("authorization", bearer(api_key))
                .body(Body::from(
                    json!({
                        "model": "gpt-test",
                        "messages": [{"role": "user", "content": "hi"}],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// 1. Per-key budget = $5; prior MTD = $3 → allowed. Bump to $6 → 429.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn per_key_budget_enforces(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    let api_key = common::seed_org_with_budget_and_key(
        &pool,
        &state,
        None,
        Some(500_000_000), // org: none, key: $5
    )
    .await;

    // Seed prior MTD = $3 via record_usage.
    common::seed_usage_record(&pool, &state.storage, &api_key, 300_000_000).await;

    // Allowed — MTD $3 < budget $5.
    let resp = chat_completion(&app, &api_key).await;
    assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    // Bump MTD to $6.
    common::seed_usage_record(&pool, &state.storage, &api_key, 300_000_000).await;

    // Now MTD $6 > budget $5 → 429.
    let resp = chat_completion(&app, &api_key).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(resp.into_body(), 1024 * 16).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"]["type"], "budget_exceeded");
    assert_eq!(v["error"]["limit"], 5.0);
    assert_eq!(v["error"]["accrued"], 6.0);
}

/// 2. Org default = $10; key has no budget; same flow.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn org_default_budget_enforces(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    let api_key = common::seed_org_with_budget_and_key(
        &pool,
        &state,
        Some(1_000_000_000),
        None, // org: $10, key: none
    )
    .await;

    common::seed_usage_record(&pool, &state.storage, &api_key, 500_000_000).await; // MTD $5
    let resp = chat_completion(&app, &api_key).await;
    assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    common::seed_usage_record(&pool, &state.storage, &api_key, 600_000_000).await; // MTD $11
    let resp = chat_completion(&app, &api_key).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(resp.into_body(), 1024 * 16).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"]["type"], "budget_exceeded");
    assert_eq!(v["error"]["limit"], 10.0); // org default $10
    assert!(v["error"]["accrued"].as_f64().unwrap() > 10.0);
}

/// 3. No per-key, no org default → unlimited path (20 requests, none 429).
///
/// Note: without a working upstream, requests fail at upstream-proxy — NOT at
/// the budget check. The budget check happens FIRST, so passing through it
/// (non-429) is what we're asserting. Same shape as phase5's `unlimited_path`.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn unlimited_budget_path(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    let api_key = common::seed_org_with_budget_and_key(&pool, &state, None, None).await;

    for _ in 0..20 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}

/// 4. Per-key ($5) overrides org default ($10) — per-key wins, MTD $6 > $5 → 429.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn per_key_overrides_org_default_budget(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    let api_key = common::seed_org_with_budget_and_key(
        &pool,
        &state,
        Some(1_000_000_000),
        Some(500_000_000),
    )
    .await;

    common::seed_usage_record(&pool, &state.storage, &api_key, 600_000_000).await; // MTD $6
    let resp = chat_completion(&app, &api_key).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(resp.into_body(), 1024 * 16).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"]["limit"], 5.0); // per-key wins, not $10
}
