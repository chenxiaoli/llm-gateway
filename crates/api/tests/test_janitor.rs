mod common;

use chrono::{Duration, Utc};
use llm_gateway_api::janitor;
use sqlx::PgPool;

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn deletes_only_old_system_rows(pool: PgPool) {
    // Seed: 1 fresh (keep) + 2 stale (delete). All created_by='system'.
    // Two users and two orgs so the test exercises cross-org impersonation
    // rows in both directions.
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('admin-1', 'admin1', 'x', 'platform_admin', 'org_default', true, NOW(), NOW())"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('admin-2', 'admin2', 'x', 'platform_admin', 'org_default', true, NOW(), NOW())"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
           VALUES ('org_other', 'other', 'Other', NULL, NOW(), NOW())"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Fresh (5 minutes old) — keep
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at, last_seen)
           VALUES ('admin-1', 'org_default', 'admin', 'system', NOW(), $1)"#,
    )
    .bind(Utc::now() - Duration::minutes(5))
    .execute(&pool)
    .await
    .unwrap();
    // Stale (2 hours old) — delete
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at, last_seen)
           VALUES ('admin-2', 'org_default', 'admin', 'system', NOW(), $1)"#,
    )
    .bind(Utc::now() - Duration::hours(2))
    .execute(&pool)
    .await
    .unwrap();
    // Stale (3 hours old) — delete, cross-org
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at, last_seen)
           VALUES ('admin-1', 'org_other', 'admin', 'system', NOW(), $1)"#,
    )
    .bind(Utc::now() - Duration::hours(3))
    .execute(&pool)
    .await
    .unwrap();

    let state = common::make_state(pool.clone());
    let deleted = janitor::cleanup_stale_impersonations(&state, Duration::hours(1))
        .await
        .unwrap();
    assert_eq!(deleted, 2, "expected 2 stale rows deleted");

    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM members WHERE created_by = 'system'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, 1, "expected the fresh row to remain");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn preserves_real_memberships(pool: PgPool) {
    // A real membership with created_by != 'system' must NEVER be deleted,
    // even if its last_seen is stale.
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('real-user', 'realuser', 'x', NULL, 'org_default', true, NOW(), NOW())"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at, last_seen)
           VALUES ('real-user', 'org_default', 'member', 'admin-1', NOW(), $1)"#,
    )
    .bind(Utc::now() - Duration::hours(24))
    .execute(&pool)
    .await
    .unwrap();

    let state = common::make_state(pool.clone());
    let deleted = janitor::cleanup_stale_impersonations(&state, Duration::hours(1))
        .await
        .unwrap();
    assert_eq!(deleted, 0, "real memberships must not be touched");

    let real_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM members WHERE user_id = 'real-user'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(real_count, 1);
}
