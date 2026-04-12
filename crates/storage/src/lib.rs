pub mod types;
pub mod sqlite;
pub mod migrations;

pub use types::*;

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // API Keys
    async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys(&self) -> Result<Vec<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Providers
    async fn create_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_providers(&self) -> Result<Vec<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_provider(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Models
    async fn create_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model(&self, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_models(&self) -> Result<Vec<ModelWithProvider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Key-Model Rate Limits
    async fn set_key_model_rate_limit(&self, limit: &KeyModelRateLimit) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_model_rate_limit(&self, key_id: &str, model_name: &str) -> Result<Option<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_key_model_rate_limits(&self, key_id: &str) -> Result<Vec<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key_model_rate_limit(&self, key_id: &str, model_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Usage
    async fn record_usage(&self, usage: &UsageRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage(&self, filter: &UsageFilter) -> Result<Vec<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage_paginated(&self, filter: &UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;

    // Audit
    async fn insert_log(&self, log: &AuditLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_logs_paginated(&self, filter: &LogFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;

    // Rate Limit Counters
    async fn increment_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    // Users
    async fn create_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_users(&self) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_users_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_user(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn count_admin_users(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn user_count(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    // Settings
    async fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
