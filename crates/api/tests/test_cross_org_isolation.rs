//! Cross-org isolation test: a member of `org_default` must NOT be able to
//! access resources under another org's slug.
//!
//! Exercises the full `management_router` so we test the real routing path
//! (global routes -> `/{org_slug}` nest -> auth -> org_resolve -> membership),
//! not just the middleware stack in isolation. This catches regressions where
//! someone re-arranges the layers and accidentally lets membership checks be
//! skipped (e.g. by moving them to a per-handler check that some routes
//! forget to call).
//!
//! See `crates/api/tests/test_middleware.rs::membership_layer_403s_non_member`
//! for the equivalent test against the raw middleware stack.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use sqlx::PgPool;
use tower::ServiceExt;

const OTHER_ORG_ID: &str = "org_other";
const OTHER_ORG_SLUG: &str = "other";

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn cross_org_access_is_rejected_with_403(pool: PgPool) {
    // --- Set up: a second org + a member of org_default only ---
    //
    // We seed org_default via the migrations (org_default is created in
    // 20260708000000_saas_orgs.sql) and we just insert org_other directly
    // into the same tables.
    sqlx::query(
        r#"INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
           VALUES ($1, $2, 'Other Org', NULL, NOW(), NOW())"#,
    )
    .bind(OTHER_ORG_ID)
    .bind(OTHER_ORG_SLUG)
    .execute(&pool)
    .await
    .expect("insert org_other");

    // Insert a user who is a member of org_default ONLY. Their
    // current_org_id is org_default (matches the JWT we'll issue). They have
    // no row in `members` for org_other, so membership_layer must reject
    // any request scoped to /api/v1/other/...
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('outsider-1', 'outsider', 'x', NULL, $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .expect("insert outsider-1 user");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ('outsider-1', $1, 'member', 'system', NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .expect("insert outsider-1 member in org_default");

    // Sanity: no membership in org_other.
    let other_member_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM members WHERE user_id = 'outsider-1' AND org_id = $1",
    )
    .bind(OTHER_ORG_ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        other_member_count, 0,
        "test setup invariant: outsider-1 must not be a member of org_other"
    );

    let state = common::make_state(pool);
    let app = management::management_router(state.clone()).with_state(state);

    // Issue a JWT for outsider-1. The JWT's current_org_id is org_default,
    // but the *path* is scoped to /api/v1/other — membership_layer must
    // resolve the path slug (other) and look up (outsider-1, org_other),
    // find no row, and 403.
    let token = llm_gateway_auth::create_jwt(
        "outsider-1",
        Some(common::TEST_ORG),
        None,
        common::TEST_JWT_SECRET,
    )
    .unwrap();
    let bearer = format!("Bearer {token}");

    // --- Negative case: GET /api/v1/other/keys → 403 ---
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/other/keys")
                .header("authorization", bearer.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "member of org_default must be rejected when accessing /api/v1/other/keys"
    );

    // --- Positive case: same user, their own org → 200 ---
    //
    // Proves the 403 above is org-specific, not a general auth failure
    // (bad token, wrong secret, etc.).
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/keys")
                .header("authorization", bearer.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "same user must succeed on their own org /api/v1/default/keys"
    );

    // --- Negative case: POST /api/v1/other/keys also rejected ---
    // (POST is the other primary verb on this endpoint; covers the
    // method-dispatch surface too.)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/other/keys")
                .header("authorization", bearer.clone())
                .header("content-type", "application/json")
                .body(Body::from("{\"name\":\"sneaky\"}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "POST to /api/v1/other/keys must also be rejected"
    );
}
