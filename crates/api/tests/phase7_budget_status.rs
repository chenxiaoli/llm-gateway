//! Integration tests for Phase 7 budget observability endpoints.
//!
//! Covers `GET /api/v1/{slug}/budget-status` (new) and the `mtd_units` field
//! on the existing `GET /api/v1/{slug}/keys` response.

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::Value;
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

async fn get(app: &axum::Router, uri: &str, token: &str) -> axum::http::Response<Body> {
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

/// Helper: insert a usage_records row + matching budget_counters row directly.
/// Costs are in 10^8 subunits per USD. Uses raw INSERTs to seed test data
/// without going through the proxy's record_usage path (which requires a full
/// request context).
async fn seed_spend(pool: &PgPool, org_id: &str, key_id: &str, cost_units: i64) {
    let now = chrono::Utc::now();
    let month_bucket = format!("{}", now.format("%Y-%m"));
    sqlx::query(
        "INSERT INTO usage_records (id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost, pricing_policy, weighted_tokens, user_id, created_at)
         VALUES ($1, $2, NULL, $3, 'test-model', 'test-provider', NULL, 'openai', 0, 0, NULL, NULL, $4, NULL, 0, NULL, $5)",
    )
    .bind(format!("rec-{}", uuid::Uuid::new_v4()))
    .bind(org_id)
    .bind(key_id)
    .bind(cost_units)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed usage_records");

    sqlx::query(
        "INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (key_id, month_bucket)
         DO UPDATE SET accrued = budget_counters.accrued + EXCLUDED.accrued, updated_at = NOW()",
    )
    .bind(key_id)
    .bind(&month_bucket)
    .bind(cost_units)
    .execute(pool)
    .await
    .expect("seed budget_counters");
}

/// 1. Fresh org → 200 with accrued_units: 0.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_zero_for_fresh_org(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool).await;

    let resp = get(&app, &format!("/api/v1/{slug}/budget-status"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["accrued_units"], 0, "fresh org has 0 MTD");
    // month_bucket is YYYY-MM shaped.
    let bucket = body["month_bucket"].as_str().unwrap();
    assert!(
        bucket.len() == 7
            && bucket.as_bytes()[0..4].iter().all(u8::is_ascii_digit)
            && bucket.as_bytes()[4] == b'-'
            && bucket.as_bytes()[5..7].iter().all(u8::is_ascii_digit),
        "month_bucket must look like YYYY-MM: got {bucket}"
    );
}

/// 2. Seeded spend → 200 with correct accrued_units.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_returns_accrued(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool).await;

    // Look up the seeded org_id + create a key so we can attach spend.
    let org_id: String = sqlx::query_scalar("SELECT id FROM orgs WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("org by slug");
    let key_id = format!("key-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO api_keys (id, org_id, name, key_hash, key_prefix, enabled, created_at, updated_at)
         VALUES ($1, $2, 'test', $3, NULL, true, $4, $5)",
    )
    .bind(&key_id)
    .bind(&org_id)
    .bind(format!("{key_id:0>64}"))
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed api_key");

    let five_usd = 500_000_000_i64;
    seed_spend(&pool, &org_id, &key_id, five_usd).await;

    let resp = get(&app, &format!("/api/v1/{slug}/budget-status"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["accrued_units"], five_usd, "endpoint must reflect seeded spend");
}

/// 3. Non-member → 403.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_403_for_non_member(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (_owner_token, slug) = common::seed_org_with_admin(&pool).await;
    // Seed a second org + a member of THAT org; their JWT resolves to org B
    // so they cannot read org A's budget-status even with a valid token.
    let (other_token, _other_slug) = common::seed_org_with_admin(&pool).await;

    let resp = get(&app, &format!("/api/v1/{slug}/budget-status"), &other_token).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// 4. No bearer → 401.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_401_unauthenticated(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (_token, slug) = common::seed_org_with_admin(&pool).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/{slug}/budget-status"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 5. list_keys response includes mtd_units field per key, value matches seeded spend.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_keys_includes_mtd_field(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool).await;
    let org_id: String = sqlx::query_scalar("SELECT id FROM orgs WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("org by slug");

    // Create a key with $3 of spend this month.
    let key_id = format!("key-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO api_keys (id, org_id, name, key_hash, key_prefix, enabled, created_at, updated_at)
         VALUES ($1, $2, 'test', $3, NULL, true, $4, $5)",
    )
    .bind(&key_id)
    .bind(&org_id)
    .bind(format!("{key_id:0>64}"))
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed api_key");
    let three_usd = 300_000_000_i64;
    seed_spend(&pool, &org_id, &key_id, three_usd).await;

    let resp = get(&app, &format!("/api/v1/{slug}/keys?page=1&page_size=50"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let items = body["items"].as_array().expect("items is array");
    let target = items
        .iter()
        .find(|k| k["id"] == key_id)
        .expect("seeded key must be in response");
    assert_eq!(target["mtd_units"], three_usd, "mtd_units must equal seeded spend");
}
