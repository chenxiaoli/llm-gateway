pub mod types;
pub mod seed;
pub mod money;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "postgres")]
pub mod postgres;

pub use money::*;

pub use types::{
    *,
    Account, Transaction, TransactionType,
    AccountResponse, TransactionResponse,
    CreateTransaction, UpdateAccountThreshold,
    DeductBalance, DeductBalanceResult,
    AddBalance, AddBalanceResult,
};
pub use seed::{SeedData, SeedProvider, SeedModel, get_available_providers, get_available_models};

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // API Keys
    async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys(&self) -> Result<Vec<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys_paginated_for_user(&self, created_by: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Providers
    async fn create_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_providers(&self) -> Result<Vec<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_provider(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Pricing Policies
    async fn create_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_pricing_policy(&self, id: &str) -> Result<Option<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_pricing_policies(&self) -> Result<Vec<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_pricing_policies_with_counts(&self) -> Result<Vec<PricingPolicyWithCounts>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_pricing_policy(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Channels
    async fn create_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
    async fn create_channel_with_models(&self, channel: &Channel, models: Vec<ChannelModel>) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channel(&self, id: &str) -> Result<Option<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channels(&self) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_enabled_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_channel(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Models
    async fn create_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model(&self, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model_by_id(&self, id: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model_by_provider(&self, provider_id: &str, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_models(&self) -> Result<Vec<ModelWithProvider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_models_by_provider(&self, provider_id: &str) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Key-Model Rate Limits
    async fn set_key_model_rate_limit(&self, limit: &KeyModelRateLimit) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_model_rate_limit(&self, key_id: &str, model_id: &str) -> Result<Option<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_key_model_rate_limits(&self, key_id: &str) -> Result<Vec<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key_model_rate_limit(&self, key_id: &str, model_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Channel Models
    async fn create_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channel_model(&self, id: &str) -> Result<Option<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channel_models(&self) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channel_models_by_channel(&self, channel_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channel_models_for_model(&self, model_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channels_for_model(&self, model_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_channel_model(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Usage
    async fn record_usage(&self, usage: &UsageRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage(&self, filter: &UsageFilter) -> Result<Vec<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage_paginated(&self, filter: &UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage_summary(&self, filter: &UsageFilter) -> Result<Vec<UsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage_cost_by_user(&self, since: chrono::DateTime<chrono::Utc>, until: chrono::DateTime<chrono::Utc>) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error + Send + Sync>>;

    // Audit
    async fn insert_log(&self, log: &AuditLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_logs_paginated(&self, filter: &LogFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLogSummary>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_log(&self, id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;

    // Rate Limit Counters
    async fn increment_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    // Users
    async fn create_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_users(&self) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_users_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<UserWithBalance>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_user(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn count_admin_users(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn user_count(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn rotate_refresh_token(&self, user_id: &str, old_token: &str, new_token: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    // Settings
    async fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Accounts
    async fn create_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_account(&self, id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_account_by_user_id(&self, user_id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>>;

    // Transactions
    async fn create_transaction(&self, transaction: &Transaction) -> Result<Transaction, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_transaction(&self, id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_transaction_by_reference(&self, account_id: &str, reference_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_transactions(&self, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, Box<dyn std::error::Error + Send + Sync>>;

    // Atomic balance operations
    async fn deduct_balance(&self, req: &DeductBalance) -> Result<DeductBalanceResult, Box<dyn std::error::Error + Send + Sync>>;
    async fn add_balance(&self, req: &AddBalance) -> Result<AddBalanceResult, Box<dyn std::error::Error + Send + Sync>>;

    // Model Fallbacks
    async fn create_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model_fallback(&self, id: &str) -> Result<Option<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_model_fallbacks(&self) -> Result<Vec<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_model_fallback(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Seed data
    async fn seed_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
