use llm_gateway_storage::{sqlite::SqliteStorage, Storage};
use llm_gateway_auth::create_jwt;
use std::sync::Arc;

pub const TEST_JWT_SECRET: &str = "test-jwt-secret";

pub struct TestUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub token: String,
}

pub async fn setup_test_db() -> Arc<SqliteStorage> {
    let storage = SqliteStorage::new(":memory:").await.unwrap();
    storage.run_migrations().await.unwrap();
    Arc::new(storage)
}

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

pub fn make_user_token(user_id: &str) -> TestUser {
    let token = create_jwt(user_id, "user", TEST_JWT_SECRET).unwrap();
    TestUser {
        id: user_id.to_string(),
        username: "testuser".to_string(),
        role: "user".to_string(),
        token,
    }
}
