mod common;

use llm_gateway_storage::Storage;
use llm_gateway_storage::types::SetPlatformRoleError;
use sqlx::PgPool;

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_grants_to_none(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-target', 'target', 'x', NULL, $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .unwrap();

    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("u-target", "admin-1", Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin), false)
        .await
        .expect("grant succeeds");

    let user = storage.get_user("u-target").await.unwrap().unwrap();
    assert_eq!(
        user.platform_role,
        Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin)
    );
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_404_for_missing_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let err = storage
        .set_user_platform_role("nonexistent", "admin-1", Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin), false)
        .await
        .unwrap_err();
    assert!(matches!(err, SetPlatformRoleError::UserNotFound));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_blocks_last_admin_self_demote(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let err = storage
        .set_user_platform_role("admin-1", "admin-1", None, false)
        .await
        .unwrap_err();
    assert!(matches!(err, SetPlatformRoleError::LastPlatformAdmin));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_allows_last_admin_with_override(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("admin-1", "admin-1", None, true)
        .await
        .expect("override flag bypasses guard");
    let user = storage.get_user("admin-1").await.unwrap().unwrap();
    assert_eq!(user.platform_role, None);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_allows_demote_when_two_admins_exist(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-second', 'second', 'x', 'platform_admin', $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .unwrap();
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("u-second", "admin-1", None, false)
        .await
        .expect("two admins -> demote succeeds");
    let user = storage.get_user("u-second").await.unwrap().unwrap();
    assert_eq!(user.platform_role, None);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_idempotent_grant(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("admin-1", "admin-1", Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin), false)
        .await
        .expect("re-grant is no-op");
    let user = storage.get_user("admin-1").await.unwrap().unwrap();
    assert_eq!(
        user.platform_role,
        Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin)
    );
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_platform_admins_returns_all_admins(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-second', 'second', 'x', 'platform_admin', $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool).await.unwrap();

    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let admins = storage.list_platform_admins().await.unwrap();
    assert_eq!(admins.len(), 2);
    let usernames: Vec<String> = admins.iter().map(|u| u.username.clone().unwrap_or_default()).collect();
    assert!(usernames.contains(&"admin".to_string()));
    assert!(usernames.contains(&"second".to_string()));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn search_user_candidates_excludes_admins_and_matches_query(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    for (id, name, email) in [
        ("u-a1", "alice_one", "alice1@x.com"),
        ("u-a2", "bob_two",   "alice2@x.com"),
        ("u-b",  "charlie",   "charlie@x.com"),
    ] {
        sqlx::query(
            r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, email, created_at, updated_at)
               VALUES ($1, $2, 'x', NULL, $3, true, $4, NOW(), NOW())"#,
        )
        .bind(id).bind(name).bind(common::TEST_ORG).bind(email)
        .execute(&pool).await.unwrap();
    }

    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let hits = storage.search_user_candidates("alice").await.unwrap();
    assert_eq!(hits.len(), 2);
    let names: Vec<String> = hits.iter().map(|u| u.username.clone().unwrap_or_default()).collect();
    assert!(names.contains(&"alice_one".to_string()));
    assert!(names.contains(&"bob_two".to_string()));
    assert!(!names.contains(&"charlie".to_string()));
    assert!(!names.contains(&"admin".to_string())); // platform_admin excluded
}
