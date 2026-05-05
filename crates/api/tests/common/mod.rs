use llm_gateway_storage::postgres::PostgresStorage;
use llm_gateway_storage::Storage;
use llm_gateway_auth::create_jwt;
use llm_gateway_api::{AppState, ChannelRegistry, ResolvedChannel, SystemInfo};
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
    })
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
