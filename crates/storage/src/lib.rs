pub mod types;
pub mod seed;
pub mod money;
pub mod postgres;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/postgres");

pub use money::*;

pub use types::{
    *,
    Account, Transaction, TransactionType,
    AccountResponse, TransactionResponse,
    CreateTransaction, UpdateAccountThreshold,
    DeductBalance, DeductBalanceResult,
    AddBalance, AddBalanceResult,
};
pub use seed::{SeedData, SeedProvider, SeedModel, get_available_providers, get_available_models, get_seed_provider_models};

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // API Keys
    async fn create_key(&self, org_id: &str, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key(&self, org_id: &str, id: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys(&self, org_id: &str) -> Result<Vec<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys_paginated_for_user(&self, org_id: &str, created_by: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_key(&self, org_id: &str, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key(&self, org_id: &str, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Providers
    async fn create_provider(&self, viewer_org_id: &str, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_provider(&self, viewer_org_id: &str, id: &str) -> Result<Option<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_providers(&self, viewer_org_id: &str) -> Result<Vec<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_provider(&self, viewer_org_id: &str, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_provider(&self, viewer_org_id: &str, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Pricing Policies
    async fn create_pricing_policy(&self, viewer_org_id: &str, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_pricing_policy(&self, viewer_org_id: &str, id: &str) -> Result<Option<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_pricing_policies(&self, viewer_org_id: &str) -> Result<Vec<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_pricing_policies_with_counts(&self, viewer_org_id: &str) -> Result<Vec<PricingPolicyWithCounts>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_pricing_policy(&self, viewer_org_id: &str, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_pricing_policy(&self, viewer_org_id: &str, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Channels
    async fn create_channel(&self, org_id: &str, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
    async fn create_channel_with_models(&self, org_id: &str, channel: &Channel, models: Vec<ChannelModel>) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channel(&self, org_id: &str, id: &str) -> Result<Option<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channels(&self, org_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channels_by_provider(&self, org_id: &str, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_enabled_channels_by_provider(&self, org_id: &str, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_channel(&self, org_id: &str, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_channel(&self, org_id: &str, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn disable_channel_until(&self, org_id: &str, id: &str, until: chrono::DateTime<chrono::Utc>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Models
    async fn create_model(&self, viewer_org_id: &str, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model(&self, viewer_org_id: &str, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model_by_id(&self, viewer_org_id: &str, id: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model_by_provider(&self, viewer_org_id: &str, provider_id: &str, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_models(&self, viewer_org_id: &str) -> Result<Vec<ModelWithProvider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_models_by_provider(&self, viewer_org_id: &str, provider_id: &str) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_model(&self, viewer_org_id: &str, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_model(&self, viewer_org_id: &str, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Key-Model Rate Limits
    async fn set_key_model_rate_limit(&self, org_id: &str, limit: &KeyModelRateLimit) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_model_rate_limit(&self, org_id: &str, key_id: &str, model_id: &str) -> Result<Option<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_key_model_rate_limits(&self, org_id: &str, key_id: &str) -> Result<Vec<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key_model_rate_limit(&self, org_id: &str, key_id: &str, model_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Channel Models
    async fn create_channel_model(&self, org_id: &str, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channel_model(&self, org_id: &str, id: &str) -> Result<Option<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channel_models(&self, org_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_channel_models_by_channel(&self, org_id: &str, channel_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channel_models_for_model(&self, org_id: &str, model_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_channels_for_model(&self, org_id: &str, model_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_channel_model(&self, org_id: &str, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_channel_model(&self, org_id: &str, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Provider Models
    async fn upsert_provider_models(&self, viewer_org_id: &str, provider_id: &str, models: Vec<ProviderModel>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn list_provider_models(&self, viewer_org_id: &str, provider_id: &str) -> Result<Vec<ProviderModelInfo>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_provider_models(&self, viewer_org_id: &str, provider_id: &str, models: Vec<ProviderModel>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Usage
    async fn record_usage(&self, org_id: &str, usage: &UsageRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage(&self, org_id: &str, filter: &UsageFilter) -> Result<Vec<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage_paginated(&self, org_id: &str, filter: &UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage_summary(&self, org_id: &str, filter: &UsageFilter) -> Result<Vec<UsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_channel_usage_summary(&self, org_id: &str, filter: &UsageFilter) -> Result<Vec<ChannelUsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_usage_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;

    async fn query_daily_usage(&self, org_id: &str, filter: &UsageFilter) -> Result<Vec<crate::types::DailyUsageRecord>, Box<dyn std::error::Error + Send + Sync>>;

    // Audit
    async fn insert_log(&self, org_id: &str, log: &AuditLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_logs(&self, org_id: &str, filter: &LogFilter) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;
    async fn query_logs_paginated(&self, org_id: &str, filter: &LogFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLogSummary>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_log(&self, org_id: &str, id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_audit_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;

    // Rate Limit Counters
    async fn increment_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    // Users
    async fn create_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_users(&self, org_id: &str) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_users_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<UserWithBalance>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_user(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn user_count(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn rotate_refresh_token(&self, user_id: &str, old_token: &str, new_token: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    // Settings
    #[deprecated(note = "use get_platform_setting directly")]
    async fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        self.get_platform_setting(key).await
    }
    #[deprecated(note = "use set_platform_setting directly")]
    async fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_platform_setting(key, value).await
    }

    // Accounts
    async fn create_account(&self, org_id: &str, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_account(&self, org_id: &str, id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_account_by_user_id(&self, org_id: &str, user_id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_account(&self, org_id: &str, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>>;

    // Transactions
    async fn create_transaction(&self, org_id: &str, transaction: &Transaction) -> Result<Transaction, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_transaction(&self, org_id: &str, id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_transaction_by_reference(&self, org_id: &str, account_id: &str, reference_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_transaction_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_transactions(&self, org_id: &str, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, Box<dyn std::error::Error + Send + Sync>>;

    // Atomic balance operations
    async fn deduct_balance(&self, org_id: &str, req: &DeductBalance) -> Result<DeductBalanceResult, Box<dyn std::error::Error + Send + Sync>>;
    async fn add_balance(&self, org_id: &str, req: &AddBalance) -> Result<AddBalanceResult, Box<dyn std::error::Error + Send + Sync>>;

    // Groups
    async fn list_groups(&self, org_id: &str) -> Result<Vec<Group>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_groups_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Group>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_group(&self, org_id: &str, id: &str) -> Result<Option<Group>, Box<dyn std::error::Error + Send + Sync>>;
    async fn create_group(&self, org_id: &str, input: &CreateGroup) -> Result<Group, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_group(&self, org_id: &str, id: &str, input: &UpdateGroup) -> Result<Group, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_group(&self, org_id: &str, id: &str) -> Result<DeleteGroupResult, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user_group_id(&self, user_id: &str, org_id: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;

    // Model Fallbacks
    // model_fallbacks is platform-level config in Phase 1 (not org-scoped).
    // The migration does not add org_id to model_fallbacks; spec Decision #3
    // excludes it from the tenant-table list. If Phase 2+ wants per-org fallbacks,
    // a follow-up migration + scope parameter here will be needed.
    async fn create_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model_fallback(&self, id: &str) -> Result<Option<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_model_fallbacks(&self) -> Result<Vec<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_model_fallback(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Seed data
    async fn seed_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // ---- Orgs ----
    async fn create_org(&self, org: CreateOrg) -> Result<Org, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_org(&self, id: &str) -> Result<Option<Org>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Org>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_orgs_for_user(&self, user_id: &str) -> Result<Vec<MembershipSummary>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_org(&self, id: &str, updates: UpdateOrg) -> Result<Org, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_org(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // ---- Members ----
    async fn get_member(&self, user_id: &str, org_id: &str) -> Result<Option<Member>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_members(&self, org_id: &str) -> Result<Vec<Member>, Box<dyn std::error::Error + Send + Sync>>;
    async fn upsert_member(&self, member: Member) -> Result<Member, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_member_role(&self, user_id: &str, org_id: &str, role: MemberRole) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_member(&self, user_id: &str, org_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn count_owners(&self, org_id: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    // ---- Settings split ----
    async fn get_platform_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_platform_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_org_setting(&self, org_id: &str, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_org_setting(&self, org_id: &str, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn list_org_settings(&self, org_id: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>>;
}
