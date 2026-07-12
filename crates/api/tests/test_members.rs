mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

const OTHER_ORG_ID: &str = "org_other";
const OTHER_ORG_SLUG: &str = "other";

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

/// Insert a user row + member row in org_default. Returns the user id.
async fn seed_default_member(
    pool: &PgPool,
    user_id: &str,
    username: &str,
    role: &str,
) -> String {
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, $3, true, NOW(), NOW())
           ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(common::TEST_ORG)
    .execute(pool)
    .await
    .expect("seed user");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ($1, $2, $3, 'admin-1', NOW())
           ON CONFLICT (user_id, org_id) DO UPDATE SET role = EXCLUDED.role"#,
    )
    .bind(user_id)
    .bind(common::TEST_ORG)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed member");
    user_id.to_string()
}

/// Insert org_other with slug "other".
async fn seed_other_org(pool: &PgPool) {
    sqlx::query(
        r#"INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
           VALUES ($1, $2, 'Other Org', NULL, NOW(), NOW())"#,
    )
    .bind(OTHER_ORG_ID)
    .bind(OTHER_ORG_SLUG)
    .execute(pool)
    .await
    .expect("seed org_other");
}

/// Make a plain-member token (no platform role) for the given user id.
fn member_token(user_id: &str) -> String {
    llm_gateway_auth::create_jwt(user_id, Some(common::TEST_ORG), None, common::TEST_JWT_SECRET).unwrap()
}

/// Insert a user that exists but has no membership in any org (for invite tests).
async fn seed_user_no_membership(pool: &PgPool, user_id: &str, username: &str) {
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, NULL, true, NOW(), NOW())
           ON CONFLICT (id) DO NOTHING"#,
    )
    .bind(user_id)
    .bind(username)
    .execute(pool)
    .await
    .expect("seed user no membership");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_members_returns_only_current_org_members(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_other_org(&pool).await;
    // Two members in org_default (admin-1 already seeded as owner).
    seed_default_member(&pool, "u-alice", "alice", "member").await;
    seed_default_member(&pool, "u-bob", "bob", "admin").await;

    // One member in org_other only — must NOT appear.
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-carol', 'carol', 'x', NULL, $1, true, NOW(), NOW())
           ON CONFLICT (id) DO NOTHING"#,
    )
    .bind(OTHER_ORG_ID)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ('u-carol', $1, 'member', 'admin-1', NOW())
           ON CONFLICT (user_id, org_id) DO NOTHING"#,
    )
    .bind(OTHER_ORG_ID)
    .execute(&pool)
    .await
    .unwrap();

    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let arr = body.as_array().expect("response is a JSON array");
    let ids: Vec<&str> = arr
        .iter()
        .map(|m| m["user_id"].as_str().unwrap())
        .collect();
    // Should include admin-1, alice, bob — NOT carol.
    assert!(ids.contains(&"admin-1"), "admin-1 must be listed");
    assert!(ids.contains(&"u-alice"), "alice must be listed");
    assert!(ids.contains(&"u-bob"), "bob must be listed");
    assert!(
        !ids.contains(&"u-carol"),
        "carol (member of org_other) must NOT be listed"
    );
    // Role serialization: lowercase string.
    let admin_row = arr
        .iter()
        .find(|m| m["user_id"] == "admin-1")
        .unwrap();
    assert_eq!(admin_row["role"], "owner");
    assert_eq!(admin_row["username"], "admin");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_members_forbidden_for_plain_member(pool: PgPool) {
    seed_default_member(&pool, "u-plain", "plain", "member").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-plain");

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&tok))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// =====================================================================
// Task 2: invite_member
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn invite_member_adds_existing_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_user_no_membership(&pool, "u-dave", "dave").await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "dave", "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["username"], "dave");
    assert_eq!(body["role"], "member");
    assert_eq!(body["user_id"], "u-dave");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn invite_member_404_for_unknown_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "ghost", "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn invite_member_409_if_already_member(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-eve", "eve", "member").await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "eve", "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// =====================================================================
// Task 3: change_member_role
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn change_role_promotes_member_to_admin(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-frank", "frank", "member").await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default/members/u-frank")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["role"], "admin");
    assert_eq!(body["user_id"], "u-frank");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn change_role_rejects_demoting_last_owner(pool: PgPool) {
    // Seed only one owner in the org and try to demote them.
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default/members/admin-1")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    let msg = body["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("last owner"),
        "expected message to mention 'last owner', got: {msg}"
    );
}

// =====================================================================
// Task 5: expanded MemberResponse (email/enabled/balance/threshold/etc)
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_members_returns_enriched_shape(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-enrich", "enrich", "member").await;

    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let arr = body.as_array().expect("response is a JSON array");
    let target = arr
        .iter()
        .find(|m| m["user_id"] == "u-enrich")
        .expect("seeded member present");
    assert!(target.get("username").is_some());
    assert!(target.get("email").is_some());
    assert!(target.get("balance").is_some());
    assert!(target.get("threshold").is_some());
    assert!(target.get("enabled").is_some());
    assert!(target.get("role").is_some());
    assert!(target.get("group_id").is_some());
    assert!(target.get("group_name").is_some());
    assert!(target.get("created_at").is_some());
    // Old field name must be gone.
    assert!(target.get("joined_at").is_none());
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_member_can_toggle_enabled(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-toggle", "toggle", "member").await;

    let app = build_app(common::make_state(pool.clone()));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default/members/u-toggle")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"enabled": false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify the underlying user row was updated.
    let enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM users WHERE id = 'u-toggle'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!enabled);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_member_can_assign_group(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-grp", "grp", "member").await;

    // Seed a group in org_default.
    sqlx::query(
        r#"INSERT INTO groups (id, org_id, name, created_at, updated_at)
           VALUES ('g-team', 'org_default', 'Team', NOW(), NOW())"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = build_app(common::make_state(pool.clone()));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default/members/u-grp")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"group_id": "g-team"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["group_id"], "g-team");
    assert_eq!(body["group_name"], "Team");

    // Verify the underlying member row was updated.
    let stored: Option<String> =
        sqlx::query_scalar("SELECT group_id FROM members WHERE user_id = 'u-grp' AND org_id = 'org_default'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.as_deref(), Some("g-team"));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_member_rejects_unknown_group(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-grp2", "grp2", "member").await;

    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default/members/u-grp2")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"group_id": "does-not-exist"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// =====================================================================
// Task 4: remove_member
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn remove_member_succeeds_for_admin(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-grace", "grace", "member").await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default/members/u-grace")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify the membership row is actually gone via the listing endpoint.
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/members")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let still_present = body
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["user_id"] == "u-grace");
    assert!(
        !still_present,
        "u-grace must no longer be in the member listing"
    );
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn remove_member_rejects_last_owner(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default/members/admin-1")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    let msg = body["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("last owner"),
        "expected message to mention 'last owner', got: {msg}"
    );
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn remove_member_can_self_remove(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-heidi", "heidi", "member").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-heidi");

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default/members/u-heidi")
                .header("authorization", bearer(&tok))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// =====================================================================
// Task 6: account routes moved from /admin/users/* to /admin/members/*
// =====================================================================
//
// `seed_default_member` writes the members row via raw SQL and bypasses the
// storage layer's `upsert_member`, so it does NOT create the paired
// accounts row that the handlers look up via `get_account_by_user_id`. We
// therefore seed the account row explicitly here (balance = 0). The
// canonical fix would be to route the helper through `upsert_member`, but
// that is out of scope for Task 6; the inline seed keeps the blast radius
// to this single test.
//
// We exercise GET /balance rather than POST /recharge to keep Task 6
// focused on the route move. (recharge's downstream `add_balance` path has
// a separate column/placeholder arity bug in postgres.rs that is unrelated
// to this refactor and will be fixed in its own change.)
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_balance_member_under_new_route(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-bal", "bal", "member").await;

    // Seed the accounts row the handler looks up. Threshold matches the
    // storage layer's default (DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS).
    sqlx::query(
        r#"INSERT INTO accounts (id, org_id, user_id, balance, threshold, created_at, updated_at)
           VALUES ('acc-bal', 'org_default', 'u-bal', 0, 0, NOW(), NOW())
           ON CONFLICT (org_id, user_id) DO NOTHING"#,
    )
    .execute(&pool)
    .await
    .expect("seed account row");

    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/default/admin/members/u-bal/balance")
                .header("authorization", bearer(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Confirm the handler found the right account (balance 0 → 0.0 USD).
    let body = body_json(resp).await;
    assert_eq!(body["account"]["user_id"], "u-bal");
    assert_eq!(body["account"]["balance"], 0.0);
}
