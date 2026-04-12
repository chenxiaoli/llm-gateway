use llm_gateway_storage::{sqlite::SqliteStorage, Storage};
use llm_gateway_auth::create_jwt;
use std::sync::Arc;

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

pub async fn setup_test_db() -> Arc<SqliteStorage> {
    let storage = SqliteStorage::new_in_memory().await.unwrap();
    storage.run_migrations().await.unwrap();
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
