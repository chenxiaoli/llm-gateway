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

// ─── Phase 5: org-defaults test helpers ──────────────────────────────────
//
// These helpers seed a *fresh* org (not org_default) with a unique slug so
// each `#[sqlx::test]` case starts from a clean slate — no leakage between
// tests via the shared org_default row. They return a JWT that carries the
// new org as `current_org_id`, which is what `org_resolve_layer` +
// `membership_layer` need to let the request through.

/// Seed a fresh org with a unique slug and an owner user. Returns the
/// owner's JWT and the new org's slug.
pub async fn seed_org_with_admin(pool: &PgPool) -> (String, String) {
    let tag = uuid::Uuid::new_v4().to_string();
    let slug = format!("o-{}", &tag.replace('-', "").to_lowercase()[..12]);
    let org_id = format!("org-{tag}");
    let user_id = format!("u-{tag}");

    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, NULL, true, NOW(), NOW())"#,
    )
    .bind(&user_id)
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed admin user");

    // Insert the org AFTER the user so the orgs.owner_id FK is satisfied.
    sqlx::query(
        r#"INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
           VALUES ($1, $2, $3, $4, NOW(), NOW())"#,
    )
    .bind(&org_id)
    .bind(&slug)
    .bind(format!("Org {tag}"))
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed org");

    // Backfill current_org_id now that the org exists (FK is deferred-ish in
    // practice but keeping the user insert above clean avoids ordering pain).
    sqlx::query("UPDATE users SET current_org_id = $1 WHERE id = $2")
        .bind(&org_id)
        .bind(&user_id)
        .execute(pool)
        .await
        .expect("set current_org_id");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ($1, $2, 'owner', $1, NOW())"#,
    )
    .bind(&user_id)
    .bind(&org_id)
    .execute(pool)
    .await
    .expect("seed owner member");

    let token =
        llm_gateway_auth::create_jwt(&user_id, Some(&org_id), None, TEST_JWT_SECRET).unwrap();
    (token, slug)
}

/// Seed a plain `member` role user in the org identified by `slug`. Returns
/// that member's JWT. The org + its owner must already exist (seed via
/// `seed_org_with_admin` first).
pub async fn seed_member_in_org(pool: &PgPool, slug: &str) -> String {
    let org_id: String = sqlx::query_scalar("SELECT id FROM orgs WHERE slug = $1")
        .bind(slug)
        .fetch_one(pool)
        .await
        .expect("org exists by slug");

    let tag = uuid::Uuid::new_v4().to_string();
    let user_id = format!("m-{tag}");

    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, $3, true, NOW(), NOW())"#,
    )
    .bind(&user_id)
    .bind(&user_id)
    .bind(&org_id)
    .execute(pool)
    .await
    .expect("seed member user");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ($1, $2, 'member', $1, NOW())"#,
    )
    .bind(&user_id)
    .bind(&org_id)
    .execute(pool)
    .await
    .expect("seed member row");

    llm_gateway_auth::create_jwt(&user_id, Some(&org_id), None, TEST_JWT_SECRET).unwrap()
}

// ─── Phase 5: proxy enforcement test helper ──────────────────────────────
//
// The proxy path uses the raw API key (not a JWT) for /v1/chat/completions.
// This helper seeds a fresh org + owner user, optionally writes the
// `default_rate_limit_rpm` org setting, inserts an api_keys row carrying the
// given per-key rate_limit, and returns the plaintext key the test then sends
// as the Bearer token. The proxy hashes it on the way in to look up the row.
//
// Note: `org_default` setting and `api_key.rate_limit` are both Option<i64>
// — None means "unlimited" / "fall back to org default" respectively.

/// Seed a fresh org + owner, optionally set an org-wide
/// `default_rate_limit_rpm`, and create an api_key with the given per-key
/// `rate_limit`. Returns the plaintext key string (the test sends it as a
/// Bearer token; the proxy hashes it for lookup).
///
/// `state` is taken by reference even though we write directly via the pool:
/// the contract is "the same AppState the app is built with", so the proxy
/// sees the same `RateLimiter` the test logic conceptually shares. We don't
/// need to call anything on it here.
pub async fn seed_org_with_default_and_key(
    pool: &PgPool,
    _state: &Arc<AppState>,
    org_default_rpm: Option<i64>,
    key_rate_limit: Option<i64>,
) -> String {
    let tag = uuid::Uuid::new_v4().to_string();
    let slug = format!("o-{}", &tag.replace('-', "").to_lowercase()[..12]);
    let org_id = format!("org-{tag}");
    let user_id = format!("u-{tag}");

    // User first (org FK references it via owner_id).
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, NULL, true, NOW(), NOW())"#,
    )
    .bind(&user_id)
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed user");

    sqlx::query(
        r#"INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
           VALUES ($1, $2, $3, $4, NOW(), NOW())"#,
    )
    .bind(&org_id)
    .bind(&slug)
    .bind(format!("Org {tag}"))
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed org");

    sqlx::query("UPDATE users SET current_org_id = $1 WHERE id = $2")
        .bind(&org_id)
        .bind(&user_id)
        .execute(pool)
        .await
        .expect("set current_org_id");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ($1, $2, 'owner', $1, NOW())"#,
    )
    .bind(&user_id)
    .bind(&org_id)
    .execute(pool)
    .await
    .expect("seed owner member");

    // Org-wide default RPM, if requested. We write the raw kv row directly
    // rather than going through the typed `set_org_defaults` facade — same
    // payload shape, and avoids a storage-trait import in this test helper.
    if let Some(rpm) = org_default_rpm {
        sqlx::query(
            r#"INSERT INTO org_settings (org_id, key, value)
               VALUES ($1, 'default_rate_limit_rpm', $2)"#,
        )
        .bind(&org_id)
        .bind(rpm.to_string())
        .execute(pool)
        .await
        .expect("set org default_rate_limit_rpm");
    }

    // Mint a plaintext key, hash it the same way the proxy will, and insert
    // the api_keys row. `created_by` is set so the proxy's balance-check
    // branch runs — but the user has no billing account, so the check is a
    // no-op (account lookup returns None).
    let plaintext = llm_gateway_auth::generate_api_key();
    let key_hash = llm_gateway_auth::hash_api_key(&plaintext);
    let key_id = format!("key-{tag}");
    sqlx::query(
        r#"INSERT INTO api_keys
             (id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly,
              enabled, created_by, model_fallback_id, created_at, updated_at)
           VALUES ($1, $2, 'test-key', $3, NULL, $4, NULL, true, $5, NULL, NOW(), NOW())"#,
    )
    .bind(&key_id)
    .bind(&org_id)
    .bind(&key_hash)
    .bind(key_rate_limit)
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed api_key");

    plaintext
}

// ─── Phase 6: budget enforcement test helpers ───────────────────────────
//
// Sibling of `seed_org_with_default_and_key` (above) but for monthly budgets.
// `default_budget_monthly_usd` is stored in `org_settings` kv as a raw string
// of integer USD subunits (10^8 per USD), matching how the production
// `set_org_defaults` facade writes it.
//
// `key_budget_monthly` is stored directly on the `api_keys.budget_monthly`
// column (BIGINT). Both are `Option<i64>` — None means "fall back to the next
// layer" / "unlimited".
pub async fn seed_org_with_budget_and_key(
    pool: &PgPool,
    _state: &Arc<AppState>,
    org_default_budget_monthly: Option<i64>,
    key_budget_monthly: Option<i64>,
) -> String {
    let tag = uuid::Uuid::new_v4().to_string();
    let slug = format!("o-{}", &tag.replace('-', "").to_lowercase()[..12]);
    let org_id = format!("org-{tag}");
    let user_id = format!("u-{tag}");

    // User first (org FK references it via owner_id).
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, NULL, true, NOW(), NOW())"#,
    )
    .bind(&user_id)
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed user");

    sqlx::query(
        r#"INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
           VALUES ($1, $2, $3, $4, NOW(), NOW())"#,
    )
    .bind(&org_id)
    .bind(&slug)
    .bind(format!("Org {tag}"))
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed org");

    sqlx::query("UPDATE users SET current_org_id = $1 WHERE id = $2")
        .bind(&org_id)
        .bind(&user_id)
        .execute(pool)
        .await
        .expect("set current_org_id");

    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ($1, $2, 'owner', $1, NOW())"#,
    )
    .bind(&user_id)
    .bind(&org_id)
    .execute(pool)
    .await
    .expect("seed owner member");

    // Org-wide default monthly budget, if requested. We write the raw kv row
    // directly rather than going through the typed `set_org_defaults` facade
    // — same payload shape, and avoids a storage-trait import in this helper.
    if let Some(units) = org_default_budget_monthly {
        sqlx::query(
            r#"INSERT INTO org_settings (org_id, key, value)
               VALUES ($1, 'default_budget_monthly_usd', $2)"#,
        )
        .bind(&org_id)
        .bind(units.to_string())
        .execute(pool)
        .await
        .expect("set org default_budget_monthly_usd");
    }

    // Mint a plaintext key, hash it the same way the proxy will, and insert
    // the api_keys row with the per-key budget_monthly. `created_by` is set
    // so the proxy's balance-check branch runs — but the user has no billing
    // account, so the check is a no-op (account lookup returns None).
    let plaintext = llm_gateway_auth::generate_api_key();
    let key_hash = llm_gateway_auth::hash_api_key(&plaintext);
    let key_id = format!("key-{tag}");
    sqlx::query(
        r#"INSERT INTO api_keys
             (id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly,
              enabled, created_by, model_fallback_id, created_at, updated_at)
           VALUES ($1, $2, 'test-key', $3, NULL, NULL, $4, true, $5, NULL, NOW(), NOW())"#,
    )
    .bind(&key_id)
    .bind(&org_id)
    .bind(&key_hash)
    .bind(key_budget_monthly)
    .bind(&user_id)
    .execute(pool)
    .await
    .expect("seed api_key");

    plaintext
}

/// Seed a usage record via the same write path production traffic uses.
///
/// Goes through `storage.record_usage(...)` so the `budget_counters.accrued`
/// row is incremented in the same transaction as `usage_records` — matching
/// what a real request would do. The proxy's `get_month_to_date_spend` then
/// reads back the counter; this gives the test the exact same counter state
/// the proxy sees in production.
///
/// `cost_units` is in USD subunits (10^8 per USD), e.g. `300_000_000` for $3.
/// `bearer` is the plaintext API key the test sent; we hash it to look up
/// the api_keys row and grab its `(id, org_id)`.
///
/// Note: `AppState` does not expose the underlying `PgPool` (it stores an
/// `Arc<dyn Storage>`), so the test helper takes the pool directly. This is
/// safe because every test here constructs `AppState` from the same pool
/// (see `make_state`), so reading the pool independently for fixture
/// seeding is observationally identical.
pub async fn seed_usage_record(
    pool: &PgPool,
    storage: &Arc<dyn llm_gateway_storage::Storage>,
    bearer: &str,
    cost_units: i64,
) {
    let key_hash = llm_gateway_auth::hash_api_key(bearer);
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id, org_id FROM api_keys WHERE key_hash = $1",
    )
    .bind(&key_hash)
    .fetch_optional(pool)
    .await
    .expect("look up api_key by hash");
    let (key_id, org_id) = row.expect("api_key not found for bearer");

    let usage = llm_gateway_storage::UsageRecord {
        id: format!("seed-{}", uuid::Uuid::new_v4()),
        org_id: org_id.clone(),
        request_id: None,
        key_id: key_id.clone(),
        model_name: "seed".into(),
        provider_id: "seed".into(),
        channel_id: None,
        protocol: llm_gateway_storage::Protocol::Openai,
        input_tokens: Some(1),
        output_tokens: Some(1),
        cache_read_tokens: None,
        cache_creation_tokens: None,
        cost: cost_units,
        pricing_policy: None,
        weighted_tokens: 0,
        user_id: None,
        created_at: chrono::Utc::now(),
    };
    storage
        .record_usage(&org_id, &usage)
        .await
        .expect("record_usage");
}
