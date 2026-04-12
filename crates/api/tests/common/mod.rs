use llm_gateway_storage::{sqlite::SqliteStorage, Storage};
use std::sync::Arc;

pub async fn setup_test_db() -> Arc<SqliteStorage> {
    let storage = SqliteStorage::new(":memory:").await.unwrap();
    storage.run_migrations().await.unwrap();
    Arc::new(storage)
}
