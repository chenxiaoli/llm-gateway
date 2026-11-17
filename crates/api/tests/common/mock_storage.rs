//! In-memory `Storage` for proxy tests.
//!
//! Only the handful of methods the proxy request path actually touches are
//! implemented; every other method panics with a clear message, so a test that
//! starts depending on a new storage call says so instead of silently
//! receiving empty data.

use async_trait::async_trait;
use llm_gateway_storage::*;
use std::sync::Mutex;

/// Storage double backed by plain vectors.
#[derive(Default)]
pub struct MockStorage {
    pub keys: Mutex<Vec<ApiKey>>,
    pub users: Mutex<Vec<User>>,
    pub accounts: Mutex<Vec<Account>>,
    pub models: Mutex<Vec<Model>>,
    pub providers: Mutex<Vec<Provider>>,
    pub channels: Mutex<Vec<Channel>>,
    pub channel_models: Mutex<Vec<ChannelModel>>,
    pub fallbacks: Mutex<Vec<ModelFallbackConfig>>,
    pub pricing_policies: Mutex<Vec<PricingPolicy>>,
    /// Audit rows written through `insert_log`.
    pub logs: Mutex<Vec<AuditLog>>,
    /// Usage rows written through `record_usage`.
    pub usage: Mutex<Vec<UsageRecord>>,
}

#[async_trait]
impl Storage for MockStorage {
    async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::run_migrations is not implemented by this test double");
    }

    async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_key is not implemented by this test double");
    }

    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_key is not implemented by this test double");
    }

    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.keys.lock().unwrap().iter().find(|k| k.key_hash == hash).cloned())
    }

    async fn list_keys(&self) -> Result<Vec<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_keys is not implemented by this test double");
    }

    async fn list_keys_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_keys_paginated is not implemented by this test double");
    }

    async fn list_keys_paginated_for_user(&self, created_by: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_keys_paginated_for_user is not implemented by this test double");
    }

    async fn update_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_key is not implemented by this test double");
    }

    async fn delete_key(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_key is not implemented by this test double");
    }

    async fn create_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_provider is not implemented by this test double");
    }

    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.providers.lock().unwrap().iter().find(|p| p.id == id).cloned())
    }

    async fn list_providers(&self) -> Result<Vec<Provider>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_providers is not implemented by this test double");
    }

    async fn update_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_provider is not implemented by this test double");
    }

    async fn delete_provider(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_provider is not implemented by this test double");
    }

    async fn create_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_pricing_policy is not implemented by this test double");
    }

    async fn get_pricing_policy(&self, id: &str) -> Result<Option<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .pricing_policies
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn list_pricing_policies(&self) -> Result<Vec<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_pricing_policies is not implemented by this test double");
    }

    async fn list_pricing_policies_with_counts(&self) -> Result<Vec<PricingPolicyWithCounts>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_pricing_policies_with_counts is not implemented by this test double");
    }

    async fn update_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_pricing_policy is not implemented by this test double");
    }

    async fn delete_pricing_policy(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_pricing_policy is not implemented by this test double");
    }

    async fn create_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_channel is not implemented by this test double");
    }

    async fn create_channel_with_models(&self, channel: &Channel, models: Vec<ChannelModel>) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_channel_with_models is not implemented by this test double");
    }

    async fn get_channel(&self, id: &str) -> Result<Option<Channel>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_channel is not implemented by this test double");
    }

    async fn list_channels(&self) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.channels.lock().unwrap().clone())
    }

    async fn list_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_channels_by_provider is not implemented by this test double");
    }

    async fn list_enabled_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_enabled_channels_by_provider is not implemented by this test double");
    }

    async fn update_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_channel is not implemented by this test double");
    }

    async fn delete_channel(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_channel is not implemented by this test double");
    }

    async fn disable_channel_until(&self, id: &str, until: chrono::DateTime<chrono::Utc>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::disable_channel_until is not implemented by this test double");
    }

    async fn create_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_model is not implemented by this test double");
    }

    async fn get_model(&self, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_model is not implemented by this test double");
    }

    async fn get_model_by_id(&self, id: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_model_by_id is not implemented by this test double");
    }

    async fn get_model_by_provider(&self, provider_id: &str, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_model_by_provider is not implemented by this test double");
    }

    async fn list_models(&self) -> Result<Vec<ModelWithProvider>, Box<dyn std::error::Error + Send + Sync>> {
        let models = self.models.lock().unwrap();
        let channel_models = self.channel_models.lock().unwrap();
        Ok(models
            .iter()
            .map(|m| {
                let cms: Vec<&ChannelModel> =
                    channel_models.iter().filter(|cm| cm.model_id == m.id).collect();
                ModelWithProvider {
                    model: m.clone(),
                    pricing_policy_name: None,
                    channel_ids: cms.iter().map(|cm| cm.channel_id.clone()).collect(),
                    channel_names: vec![],
                }
            })
            .collect())
    }

    async fn list_models_by_provider(&self, provider_id: &str) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_models_by_provider is not implemented by this test double");
    }

    async fn update_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_model is not implemented by this test double");
    }

    async fn delete_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_model is not implemented by this test double");
    }

    async fn set_key_model_rate_limit(&self, limit: &KeyModelRateLimit) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::set_key_model_rate_limit is not implemented by this test double");
    }

    async fn get_key_model_rate_limit(&self, key_id: &str, model_id: &str) -> Result<Option<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_key_model_rate_limit is not implemented by this test double");
    }

    async fn list_key_model_rate_limits(&self, key_id: &str) -> Result<Vec<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_key_model_rate_limits is not implemented by this test double");
    }

    async fn delete_key_model_rate_limit(&self, key_id: &str, model_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_key_model_rate_limit is not implemented by this test double");
    }

    async fn create_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_channel_model is not implemented by this test double");
    }

    async fn get_channel_model(&self, id: &str) -> Result<Option<ChannelModel>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_channel_model is not implemented by this test double");
    }

    async fn list_channel_models(&self) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_channel_models is not implemented by this test double");
    }

    async fn list_channel_models_by_channel(&self, channel_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_channel_models_by_channel is not implemented by this test double");
    }

    async fn get_channel_models_for_model(&self, model_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .channel_models
            .lock()
            .unwrap()
            .iter()
            .filter(|cm| cm.model_id == model_id)
            .cloned()
            .collect())
    }

    async fn get_channels_for_model(&self, model_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_channels_for_model is not implemented by this test double");
    }

    async fn update_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_channel_model is not implemented by this test double");
    }

    async fn delete_channel_model(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_channel_model is not implemented by this test double");
    }

    async fn upsert_provider_models(&self, provider_id: &str, models: Vec<ProviderModel>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::upsert_provider_models is not implemented by this test double");
    }

    async fn list_provider_models(&self, provider_id: &str) -> Result<Vec<ProviderModelInfo>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_provider_models is not implemented by this test double");
    }

    async fn set_provider_models(&self, provider_id: &str, models: Vec<ProviderModel>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::set_provider_models is not implemented by this test double");
    }

    async fn record_usage(&self, usage: &UsageRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.usage.lock().unwrap().push(usage.clone());
        Ok(())
    }

    async fn query_usage(&self, filter: &UsageFilter) -> Result<Vec<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::query_usage is not implemented by this test double");
    }

    async fn query_usage_paginated(&self, filter: &UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::query_usage_paginated is not implemented by this test double");
    }

    async fn query_usage_summary(&self, filter: &UsageFilter) -> Result<Vec<UsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::query_usage_summary is not implemented by this test double");
    }

    async fn query_channel_usage_summary(&self, filter: &UsageFilter) -> Result<Vec<ChannelUsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::query_channel_usage_summary is not implemented by this test double");
    }

    async fn get_usage_by_request_id(&self, request_id: &str) -> Result<Option<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_usage_by_request_id is not implemented by this test double");
    }

    async fn query_daily_usage(&self, filter: &UsageFilter) -> Result<Vec<DailyUsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::query_daily_usage is not implemented by this test double");
    }

    async fn insert_log(&self, log: &AuditLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logs.lock().unwrap().push(log.clone());
        Ok(())
    }

    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::query_logs is not implemented by this test double");
    }

    async fn query_logs_paginated(&self, filter: &LogFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLogSummary>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::query_logs_paginated is not implemented by this test double");
    }

    async fn get_log(&self, id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_log is not implemented by this test double");
    }

    async fn get_audit_by_request_id(&self, request_id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_audit_by_request_id is not implemented by this test double");
    }

    async fn increment_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::increment_rate_limit_counter is not implemented by this test double");
    }

    async fn get_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_rate_limit_counter is not implemented by this test double");
    }

    async fn create_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_user is not implemented by this test double");
    }

    async fn get_user(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.users.lock().unwrap().iter().find(|u| u.id == id).cloned())
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_user_by_username is not implemented by this test double");
    }

    async fn list_users(&self) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_users is not implemented by this test double");
    }

    async fn list_users_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<UserWithBalance>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_users_paginated is not implemented by this test double");
    }

    async fn update_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_user is not implemented by this test double");
    }

    async fn delete_user(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_user is not implemented by this test double");
    }

    async fn count_admin_users(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::count_admin_users is not implemented by this test double");
    }

    async fn user_count(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::user_count is not implemented by this test double");
    }

    async fn rotate_refresh_token(&self, user_id: &str, old_token: &str, new_token: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::rotate_refresh_token is not implemented by this test double");
    }

    async fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_setting is not implemented by this test double");
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::set_setting is not implemented by this test double");
    }

    async fn create_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_account is not implemented by this test double");
    }

    async fn get_account(&self, id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_account is not implemented by this test double");
    }

    async fn get_account_by_user_id(&self, user_id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.accounts.lock().unwrap().iter().find(|a| a.user_id == user_id).cloned())
    }

    async fn update_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_account is not implemented by this test double");
    }

    async fn create_transaction(&self, transaction: &Transaction) -> Result<Transaction, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_transaction is not implemented by this test double");
    }

    async fn get_transaction(&self, id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_transaction is not implemented by this test double");
    }

    async fn get_transaction_by_reference(&self, account_id: &str, reference_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_transaction_by_reference is not implemented by this test double");
    }

    async fn get_transaction_by_request_id(&self, request_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_transaction_by_request_id is not implemented by this test double");
    }

    async fn list_transactions(&self, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_transactions is not implemented by this test double");
    }

    async fn deduct_balance(&self, req: &DeductBalance) -> Result<DeductBalanceResult, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::deduct_balance is not implemented by this test double");
    }

    async fn add_balance(&self, req: &AddBalance) -> Result<AddBalanceResult, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::add_balance is not implemented by this test double");
    }

    async fn list_groups(&self) -> Result<Vec<Group>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_groups is not implemented by this test double");
    }

    async fn list_groups_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<Group>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::list_groups_paginated is not implemented by this test double");
    }

    async fn get_group(&self, id: &str) -> Result<Option<Group>, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::get_group is not implemented by this test double");
    }

    async fn create_group(&self, input: &CreateGroup) -> Result<Group, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_group is not implemented by this test double");
    }

    async fn update_group(&self, id: &str, input: &UpdateGroup) -> Result<Group, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_group is not implemented by this test double");
    }

    async fn delete_group(&self, id: &str) -> Result<DeleteGroupResult, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_group is not implemented by this test double");
    }

    async fn get_user_group_id(&self, user_id: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == user_id)
            .and_then(|u| u.group_id.clone()))
    }

    async fn create_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::create_model_fallback is not implemented by this test double");
    }

    async fn get_model_fallback(&self, id: &str) -> Result<Option<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.fallbacks.lock().unwrap().iter().find(|f| f.id == id).cloned())
    }

    async fn list_model_fallbacks(&self) -> Result<Vec<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.fallbacks.lock().unwrap().clone())
    }

    async fn update_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::update_model_fallback is not implemented by this test double");
    }

    async fn delete_model_fallback(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::delete_model_fallback is not implemented by this test double");
    }

    async fn seed_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("MockStorage::seed_data is not implemented by this test double");
    }
}
