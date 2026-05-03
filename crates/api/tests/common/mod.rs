use llm_gateway_storage::{postgres::PostgresStorage, Storage};
use llm_gateway_auth::create_jwt;
use llm_gateway_api::{ChannelRegistry, ResolvedChannel};
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
}

pub const TEST_JWT_SECRET: &str = "test-jwt-secret";

#[allow(dead_code)]
pub struct TestUser {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub username: String,
    #[allow(dead_code)]
    pub role: String,
    pub token: String,
}

/// Set up a test database connection.
/// Requires DATABASE_URL env var to point at a PostgreSQL instance.
/// Truncates all tables to ensure test isolation.
pub async fn setup_test_db() -> Arc<PostgresStorage> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL env var must be set for tests");
    let storage = PostgresStorage::new(&database_url)
        .await
        .expect("Failed to connect to test PostgreSQL");
    storage.run_migrations().await.expect("Failed to run migrations");

    // Truncate all tables for test isolation
    let tables = [
        "transactions", "accounts", "usage_records", "audit_logs",
        "rate_limit_counters", "channel_models", "channels",
        "provider_models", "provider_models_pricing", "models",
        "providers", "api_keys", "users", "settings",
        "pricing_policies", "model_fallbacks",
    ];
    for table in &tables {
        sqlx::query(&format!("TRUNCATE TABLE {} CASCADE", table))
            .execute(storage.pool())
            .await
            .expect("Failed to truncate table");
    }

    Arc::new(storage)
}

#[allow(dead_code)]
pub fn make_admin_token() -> TestUser {
    let id = "admin-1".to_string();
    let token = create_jwt(&id, "admin", TEST_JWT_SECRET).unwrap();
    TestUser {
        id,
        username: "admin".to_string(),
        role: "admin".to_string(),
        token,
    }
}

#[allow(dead_code)]
pub fn make_user_token(user_id: &str) -> TestUser {
    let token = create_jwt(user_id, "user", TEST_JWT_SECRET).unwrap();
    TestUser {
        id: user_id.to_string(),
        username: "testuser".to_string(),
        role: "user".to_string(),
        token,
    }
}
