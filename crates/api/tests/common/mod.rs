use llm_gateway_storage::postgres::PostgresStorage;
use llm_gateway_storage::Storage;
use llm_gateway_auth::create_jwt;
use llm_gateway_api::{AppState, ChannelRegistry, ResolvedChannel, SystemInfo};
use llm_gateway_email::noop::NoopMailer;
use llm_gateway_email::templates::TemplateRegistry;
use llm_gateway_ratelimit::RateLimiter;
use sqlx::postgres::PgPool;
use std::sync::Arc;

pub struct MockChannelRegistry;

#[async_trait::async_trait]
impl ChannelRegistry for MockChannelRegistry {
    async fn resolve(&self, _channel_id: &str) -> Option<ResolvedChannel> {
        None
    }
    async fn resolve_by_model(&self, _model: &str) -> Vec<ResolvedChannel> {
        Vec::new()
    }
    async fn reload(&self) {}
    fn disable_channel_model(&self, _channel_id: &str, _model_name: &str, _until: std::time::Instant) {}
    fn is_circuit_broken(&self, _channel_id: &str, _model_name: &str) -> bool { false }
}

pub const TEST_JWT_SECRET: &str = "test-jwt-secret";
pub const TEST_ORG: &str = "org_default";

#[allow(dead_code)]
pub struct TestUser {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub username: String,
    pub token: String,
}

pub fn make_state(pool: PgPool) -> Arc<AppState> {
    let db = PostgresStorage::from_pool(pool);
    Arc::new(AppState {
        storage: Arc::new(db) as Arc<dyn Storage>,
        rate_limiter: Arc::new(RateLimiter::new(60)),
        jwt_secret: TEST_JWT_SECRET.to_string(),
        encryption_key: [0u8; 32],
        nats_publisher: None,
        registry: Arc::new(MockChannelRegistry),
        system_info: SystemInfo {
            server_bind_address: "0.0.0.0:8080".to_string(),
            database_driver: "postgres".to_string(),
            rate_limit_window_secs: 60,
            rate_limit_flush_interval_secs: 30,
            upstream_timeout_secs: 30,
            audit_retention_days: Some(90),
        },
        public_base_url: "http://localhost:5173".to_string(),
        mailer: Arc::new(NoopMailer::new()),
        templates: Arc::new(
            TemplateRegistry::load("noreply@test.local".to_string(), "Test".to_string())
                .expect("load templates"),
        ),
    })
}

/// Insert (or replace) the canonical test admin user with id="admin-1" plus
/// an owner membership in `org_default`. Required because `resolve_org_context`
/// now does a member lookup — JWTs alone are no longer sufficient.
///
/// `role: "admin"` historically meant "platform admin" in the test fixtures;
/// we now store that as `platform_role = Some(PlatformRole::PlatformAdmin)` on
/// the user row, plus an owner member row so the per-org checks pass too.
pub async fn seed_admin_user(pool: &PgPool) {
    // upsert user
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('admin-1', 'admin', 'x', 'platform_admin', $1, true, NOW(), NOW())
           ON CONFLICT (id) DO UPDATE SET platform_role = 'platform_admin', current_org_id = $1, enabled = true"#,
    )
    .bind(TEST_ORG)
    .execute(pool)
    .await
    .expect("seed admin user");

    // upsert owner membership in org_default
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ('admin-1', $1, 'owner', 'admin-1', NOW())
           ON CONFLICT (user_id, org_id) DO UPDATE SET role = 'owner'"#,
    )
    .bind(TEST_ORG)
    .execute(pool)
    .await
    .expect("seed admin member");
}

/// Mark a user's email as verified, bypassing the verification gate so
/// /auth/login succeeds. Tests that don't care about the verification
/// flow call this after registration; tests that DO care leave the user
/// un-verified and assert the login gate.
pub async fn mark_user_verified(pool: &PgPool, username: &str) {
    sqlx::query(
        "UPDATE users SET email_verified_at = NOW() WHERE username = $1",
    )
    .bind(username)
    .execute(pool)
    .await
    .expect("mark user verified");
}

/// Insert (or replace) a regular test member with the given id and an
/// optional group_id. Used by tests that exercise the member-role path.
pub async fn seed_member(pool: &PgPool, user_id: &str, group_id: Option<&str>) {
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $1, 'x', NULL, $2, true, NOW(), NOW())
           ON CONFLICT (id) DO NOTHING"#,
    )
    .bind(user_id)
    .bind(TEST_ORG)
    .execute(pool)
    .await
    .expect("seed member user");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, group_id, created_by, created_at)
           VALUES ($1, $2, 'member', $3, 'admin-1', NOW())
           ON CONFLICT (user_id, org_id) DO UPDATE SET group_id = $3"#,
    )
    .bind(user_id)
    .bind(TEST_ORG)
    .bind(group_id)
    .execute(pool)
    .await
    .expect("seed member row");
}

#[allow(dead_code)]
pub fn make_admin_token() -> TestUser {
    let id = "admin-1".to_string();
    let token = create_jwt(&id, Some(TEST_ORG), Some("platform_admin"), TEST_JWT_SECRET).unwrap();
    TestUser {
        id,
        username: "admin".to_string(),
        token,
    }
}

#[allow(dead_code)]
pub fn make_user_token(user_id: &str) -> TestUser {
    let token = create_jwt(user_id, Some(TEST_ORG), None, TEST_JWT_SECRET).unwrap();
    TestUser {
        id: user_id.to_string(),
        username: "testuser".to_string(),
        token,
    }
}

#[allow(dead_code)]
pub fn make_member_token(user_id: &str) -> TestUser {
    // Alias for make_user_token; kept separate to communicate intent.
    make_user_token(user_id)
}
