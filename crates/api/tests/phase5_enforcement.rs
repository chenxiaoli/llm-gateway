//! Integration tests for proxy rate-limit enforcement (Phase 5).
//!
//! Verifies the resolution order:
//!   effective_rpm = api_key.rate_limit ?? org.default_rate_limit_rpm ?? None
//! and that exceeding returns 429 with Retry-After.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::Router;
use llm_gateway_api::management;
use llm_gateway_api::proxy;
use llm_gateway_api::AppState;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

/// Build the FULL app (management + proxy routes), mirroring the assembly in
/// `crates/gateway/src/main.rs`. The management router alone does not include
/// `/v1/chat/completions`, so we layer the proxy routes onto the same state.
fn build_full_app(state: Arc<AppState>) -> Router {
    Router::new()
        // Proxy routes — same paths the gateway binary mounts.
        .route(
            "/v1/chat/completions",
            post(proxy::proxy_with_protocol),
        )
        .route("/v1/messages", post(proxy::messages))
        .route("/v1/responses", post(proxy::responses))
        // Management API (org-scoped + global auth routes).
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

/// 1. Org default = 5; key has no per-key limit; 6th request → 429.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn org_default_enforces(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    // Seed: org with default_rate_limit_rpm = 5; admin user; key with no
    // per-key rate_limit set.
    let api_key =
        common::seed_org_with_default_and_key(&pool, &state, Some(5), None).await;

    for _ in 0..5 {
        let resp = chat_completion(&app, &api_key).await;
        // The first 5 should NOT be 429. (They may be other statuses —
        // upstream will fail since there's no real provider — but enforcement
        // must not reject them.)
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    let sixth = chat_completion(&app, &api_key).await;
    assert_eq!(sixth.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = sixth
        .headers()
        .get("retry-after")
        .expect("Retry-After header")
        .to_str()
        .unwrap();
    assert!(retry_after.parse::<u64>().unwrap() > 0);
}

/// 2. Per-key rate_limit = 10 wins over no org default; 11th → 429.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn per_key_enforces_without_org_default(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    let api_key =
        common::seed_org_with_default_and_key(&pool, &state, None, Some(10)).await;

    for _ in 0..10 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    let eleventh = chat_completion(&app, &api_key).await;
    assert_eq!(eleventh.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// 3. No per-key, no org default → unlimited (no 429s across many requests).
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn unlimited_path(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    let api_key =
        common::seed_org_with_default_and_key(&pool, &state, None, None).await;

    for _ in 0..20 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}

/// 4. Per-key (10) wins over org default (5) — 6 succeeds, 11 fails.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn per_key_overrides_org_default(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let app = build_full_app(state.clone());

    let api_key =
        common::seed_org_with_default_and_key(&pool, &state, Some(5), Some(10)).await;

    for _ in 0..6 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(
            resp.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "per-key must override org default — 6th should pass under per-key limit of 10"
        );
    }
    // Now exhaust the remaining 4 calls (total 10).
    for _ in 0..4 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    let eleventh = chat_completion(&app, &api_key).await;
    assert_eq!(eleventh.status(), StatusCode::TOO_MANY_REQUESTS);
}
