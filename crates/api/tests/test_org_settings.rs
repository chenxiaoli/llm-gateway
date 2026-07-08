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
const OWNER_PASS: &str = "owner-pass";

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

/// Insert a user row + member row in org_default with the given role.
/// Password is stored as plaintext 'x' (no login needed for these tests).
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

/// Insert a member whose user row carries a real bcrypt hash, so password
/// re-check on DELETE can succeed/fail meaningfully.
async fn seed_default_member_with_password(
    pool: &PgPool,
    user_id: &str,
    username: &str,
    role: &str,
    password_hash: &str,
) -> String {
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, $3, NULL, $4, true, NOW(), NOW())
           ON CONFLICT (id) DO UPDATE SET password = EXCLUDED.password, username = EXCLUDED.username"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(password_hash)
    .bind(common::TEST_ORG)
    .execute(pool)
    .await
    .expect("seed user with password");

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

/// Insert org_other with slug "other" (for duplicate-slug tests).
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

/// Make a plain-member JWT (no platform role) for the given user id.
fn member_token(user_id: &str) -> String {
    llm_gateway_auth::create_jwt(user_id, Some(common::TEST_ORG), None, common::TEST_JWT_SECRET).unwrap()
}

// =====================================================================
// Task 5: PATCH /{org_slug} — update name / slug
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_renames_name(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "New Name"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "New Name");
    // Slug should be unchanged.
    assert_eq!(body["slug"], "default");
    // Role is echoed back from the caller's context.
    assert_eq!(body["role"], "owner");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_changes_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"slug": "new-slug"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["slug"], "new-slug");
    // Name should be unchanged.
    assert_eq!(body["name"], "Default Org");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_rejects_invalid_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"slug": "UPPER CASE"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_rejects_reserved_slug(pool: PgPool) {
    // Renaming to a slug that collides with a literal-410 legacy route
    // (keys, model-fallbacks, usage) would make the org unreachable — every
    // /{slug}/... request would be absorbed by the 410 handlers.
    common::seed_admin_user(&pool).await;
    let admin = common::make_admin_token();

    for reserved in ["keys", "model-fallbacks", "usage"] {
        let app = build_app(common::make_state(pool.clone()));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/default")
                    .header("authorization", bearer(&admin.token))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "slug": reserved }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "slug '{reserved}' should be rejected as reserved"
        );
    }
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_rejects_duplicate_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_other_org(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"slug": "other"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_rejects_empty_name(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build_app(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"name": "  "}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_forbidden_for_member(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-plain", "plain", "member").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-plain");

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&tok))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "Hacked"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn update_org_admin_role_can_patch(pool: PgPool) {
    // Admin (not owner) should be allowed by can_manage_org_settings.
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-admin", "adminuser", "admin").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-admin");

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/default")
                .header("authorization", bearer(&tok))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "Admin Rename"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Admin Rename");
}

// =====================================================================
// Task 6: DELETE /{org_slug} — owner + password
// =====================================================================

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn delete_org_requires_owner(pool: PgPool) {
    // Admin (not owner) attempting delete → 403.
    common::seed_admin_user(&pool).await;
    seed_default_member(&pool, "u-admin", "adminuser", "admin").await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("u-admin");

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default")
                .header("authorization", bearer(&tok))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"password": "anything"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn delete_org_requires_password(pool: PgPool) {
    // Owner JWT with wrong password → 401.
    let hash = llm_gateway_auth::hash_password(OWNER_PASS).unwrap();
    // Replace the seeded admin-1 with a real password hash while keeping the
    // owner membership + platform_admin role.
    common::seed_admin_user(&pool).await;
    sqlx::query(
        r#"UPDATE users SET password = $1 WHERE id = 'admin-1'"#,
    )
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();
    // Silence unused-variable lint in case hash is unused on some builds.
    let _ = &hash;

    let app = build_app(common::make_state(pool.clone()));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"password": "wrong-password"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Verify the org is still there.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orgs WHERE id = $1")
        .bind(common::TEST_ORG)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "org should still exist after failed delete");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn delete_org_with_correct_password_cascades(pool: PgPool) {
    // Owner with correct password → 204, and members row is gone via cascade.
    common::seed_admin_user(&pool).await;
    // Add a second member so we can verify the cascade hit someone other than
    // the deleting owner.
    seed_default_member(&pool, "u-bob", "bob", "member").await;
    // Set a real password hash on the owner.
    let hash = llm_gateway_auth::hash_password(OWNER_PASS).unwrap();
    sqlx::query(
        r#"UPDATE users SET password = $1 WHERE id = 'admin-1'"#,
    )
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();

    let app = build_app(common::make_state(pool.clone()));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default")
                .header("authorization", bearer(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"password": OWNER_PASS}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Org row gone.
    let org_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orgs WHERE id = $1")
        .bind(common::TEST_ORG)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(org_count, 0, "org row should be gone after delete");

    // Members cascade — no member rows should reference the deleted org.
    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE org_id = $1")
            .bind(common::TEST_ORG)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        member_count, 0,
        "member rows should be cascade-deleted with the org"
    );
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn delete_org_owner_via_direct_seed(pool: PgPool) {
    // Independent test: seed an owner with a real password directly (not via
    // seed_admin_user, which sets platform_admin) and confirm the owner role
    // alone satisfies can_delete_org.
    let hash = llm_gateway_auth::hash_password(OWNER_PASS).unwrap();
    seed_default_member_with_password(
        &pool,
        "owner-1",
        "owner",
        "owner",
        &hash,
    )
    .await;
    let app = build_app(common::make_state(pool));
    let tok = member_token("owner-1");

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/default")
                .header("authorization", bearer(&tok))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"password": OWNER_PASS}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
