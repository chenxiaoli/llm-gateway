use crate::types::*;
use crate::seed;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;
        Ok(Self { pool })
    }

    pub async fn new_in_memory() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let db_name = format!("file:testdb_{}?mode=memory&cache=shared", uuid::Uuid::new_v4());
        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_name))?
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// ---------------------------------------------------------------------------
// SQLite row types (integers for booleans, TEXT for dates)
// ---------------------------------------------------------------------------

#[derive(FromRow)]
struct SqliteKeyRow {
    id: String,
    name: String,
    key_hash: String,
    rate_limit: Option<i64>,
    budget_monthly: Option<f64>,
    enabled: i64,
    created_by: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<SqliteKeyRow> for ApiKey {
    fn from(r: SqliteKeyRow) -> Self {
        ApiKey {
            id: r.id,
            name: r.name,
            key_hash: r.key_hash,
            rate_limit: r.rate_limit,
            budget_monthly: r.budget_monthly,
            enabled: r.enabled != 0,
            created_by: r.created_by,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteProviderRow {
    id: String,
    name: String,
    slug: String,
    base_url: Option<String>,
    endpoints: Option<String>,
    enabled: i64,
    created_at: String,
    updated_at: String,
}

impl From<SqliteProviderRow> for Provider {
    fn from(r: SqliteProviderRow) -> Self {
        Provider {
            id: r.id,
            name: r.name,
            slug: r.slug,
            base_url: r.base_url,
            endpoints: r.endpoints,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteModelRow {
    id: String,
    name: String,
    model_type: Option<String>,
    billing_type: String,
    input_price: f64,
    output_price: f64,
    request_price: f64,
    enabled: i64,
    created_at: String,
}

impl From<SqliteModelRow> for Model {
    fn from(r: SqliteModelRow) -> Self {
        Model {
            id: r.id,
            name: r.name,
            model_type: r.model_type,
            billing_type: r.billing_type,
            input_price: r.input_price,
            output_price: r.output_price,
            request_price: r.request_price,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteModelWithProviderRow {
    id: String,
    name: String,
    model_type: Option<String>,
    billing_type: String,
    input_price: f64,
    output_price: f64,
    request_price: f64,
    enabled: i64,
    created_at: String,
    provider_name: String,
    base_url: Option<String>,
    endpoints: Option<String>,
}

impl From<SqliteModelWithProviderRow> for ModelWithProvider {
    fn from(r: SqliteModelWithProviderRow) -> Self {
        // Parse endpoints JSON to determine compatibility
        // If provider name is "Anthropic", it's natively compatible with Anthropic API
        // If provider name is "OpenAI", it's natively compatible with OpenAI API
        let provider_name_lower = r.provider_name.to_lowercase();
        let is_native_anthropic = provider_name_lower == "anthropic";
        let is_native_openai = provider_name_lower == "openai";

        let openai_compatible = r.endpoints.as_ref()
            .and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok())
            .map(|v| v.get("openai").is_some())
            .unwrap_or(r.base_url.is_some() || is_native_openai);
        let anthropic_compatible = r.endpoints.as_ref()
            .and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok())
            .map(|v| v.get("anthropic").is_some())
            .unwrap_or(is_native_anthropic);

        ModelWithProvider {
            model: Model {
                id: r.id,
                name: r.name,
                model_type: r.model_type,
                billing_type: r.billing_type,
                input_price: r.input_price,
                output_price: r.output_price,
                request_price: r.request_price,
                enabled: r.enabled != 0,
                created_at: parse_rfc3339(&r.created_at),
            },
            provider_name: r.provider_name,
            openai_compatible,
            anthropic_compatible,
        }
    }
}

#[derive(FromRow)]
struct SqliteUsageRow {
    id: String,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    protocol: String,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cost: f64,
    created_at: String,
}

impl From<SqliteUsageRow> for UsageRecord {
    fn from(r: SqliteUsageRow) -> Self {
        UsageRecord {
            id: r.id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            protocol: parse_protocol(&r.protocol),
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cost: r.cost,
            created_at: parse_rfc3339(&r.created_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteAuditRow {
    id: String,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    protocol: String,
    stream: i32,
    request_body: String,
    response_body: String,
    status_code: i32,
    latency_ms: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    created_at: String,
    original_model: Option<String>,
    upstream_model: Option<String>,
    model_override_reason: Option<String>,
}

impl From<SqliteAuditRow> for AuditLog {
    fn from(r: SqliteAuditRow) -> Self {
        AuditLog {
            id: r.id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            protocol: parse_protocol(&r.protocol),
            stream: r.stream != 0, // Convert i32 to bool
            request_body: r.request_body,
            response_body: r.response_body,
            status_code: r.status_code,
            latency_ms: r.latency_ms,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            created_at: parse_rfc3339(&r.created_at),
            original_model: r.original_model,
            upstream_model: r.upstream_model,
            model_override_reason: r.model_override_reason,
        }
    }
}

#[derive(FromRow)]
struct SqliteChannelRow {
    id: String,
    provider_id: String,
    name: String,
    api_key: String,
    base_url: Option<String>,
    priority: i32,
    enabled: i64,
    pricing_policy_id: Option<String>,  // NEW
    markup_ratio: f64,                   // NEW
    rpm_limit: Option<i64>,
    tpm_limit: Option<i64>,
    balance: Option<f64>,
    weight: Option<i32>,
    created_at: String,
    updated_at: String,
}

impl From<SqliteChannelRow> for Channel {
    fn from(r: SqliteChannelRow) -> Self {
        Channel {
            id: r.id,
            provider_id: r.provider_id,
            name: r.name,
            api_key: r.api_key,
            base_url: r.base_url,
            priority: r.priority,
            pricing_policy_id: r.pricing_policy_id,
            markup_ratio: r.markup_ratio,
            enabled: r.enabled != 0,
            rpm_limit: r.rpm_limit,
            tpm_limit: r.tpm_limit,
            balance: r.balance,
            weight: r.weight,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteUserRow {
    id: String,
    username: String,
    password: String,
    role: String,
    enabled: i64,
    refresh_token: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<SqliteUserRow> for User {
    fn from(r: SqliteUserRow) -> Self {
        User {
            id: r.id,
            username: r.username,
            password: r.password,
            role: r.role,
            enabled: r.enabled != 0,
            refresh_token: r.refresh_token,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteChannelModelRow {
    id: String,
    channel_id: String,
    model_id: String,
    upstream_model_name: String,
    priority_override: Option<i32>,
    cost_policy_id: Option<String>,   // NEW: for upstream cost
    markup_ratio: f64,                // NEW, default 1.0
    billing_type: Option<String>,      // NEW: billing_type
    input_price: Option<f64>,          // NEW: input_price
    output_price: Option<f64>,         // NEW: output_price
    request_price: Option<f64>,        // NEW: request_price
    enabled: i64,
    created_at: String,
    updated_at: String,
}

impl From<SqliteChannelModelRow> for ChannelModel {
    fn from(r: SqliteChannelModelRow) -> Self {
        ChannelModel {
            id: r.id,
            channel_id: r.channel_id,
            model_id: r.model_id,
            upstream_model_name: r.upstream_model_name,
            priority_override: r.priority_override,
            cost_policy_id: r.cost_policy_id,
            markup_ratio: r.markup_ratio,
            billing_type: r.billing_type,
            input_price: r.input_price,
            output_price: r.output_price,
            request_price: r.request_price,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

#[derive(FromRow)]
struct SqlitePricingPolicyRow {
    id: String,
    name: String,
    billing_type: String,
    config: String,
    created_at: String,
    updated_at: String,
}

impl From<SqlitePricingPolicyRow> for PricingPolicy {
    fn from(r: SqlitePricingPolicyRow) -> Self {
        PricingPolicy {
            id: r.id,
            name: r.name,
            billing_type: r.billing_type,
            config: serde_json::from_str(&r.config).unwrap_or(serde_json::Value::Null),
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_rfc3339(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .unwrap()
        .with_timezone(&chrono::Utc)
}

fn parse_billing_type(s: &str) -> BillingType {
    match s {
        "token" => BillingType::Token,
        "request" => BillingType::Request,
        _ => BillingType::Token,
    }
}

fn parse_protocol(s: &str) -> Protocol {
    match s {
        "openai" => Protocol::Openai,
        "anthropic" => Protocol::Anthropic,
        _ => Protocol::Openai,
    }
}

fn billing_type_str(bt: &BillingType) -> &'static str {
    match bt {
        BillingType::Token => "token",
        BillingType::Request => "request",
    }
}

fn protocol_str(p: &Protocol) -> &'static str {
    match p {
        Protocol::Openai => "openai",
        Protocol::Anthropic => "anthropic",
    }
}

// ---------------------------------------------------------------------------
// Storage trait implementation
// ---------------------------------------------------------------------------

type DbErr = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
impl crate::Storage for SqliteStorage {
    // ---- Migrations ----

    async fn run_migrations(&self) -> Result<(), DbErr> {
        // sqlx::migrate!() embeds migration files at compile time — no disk access at runtime.
        let migrator = sqlx::migrate!("./migrations");
        migrator.run(&self.pool).await.map_err(|e: sqlx::migrate::MigrateError| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

        Ok(())
    }

    // ---- API Keys ----

    async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, DbErr> {
        sqlx::query(
            "INSERT INTO api_keys (id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&key.id)
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled as i64)
        .bind(&key.created_by)
        .bind(key.created_at.to_rfc3339())
        .bind(key.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(key.clone())
    }

    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys WHERE key_hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn list_keys(&self) -> Result<Vec<ApiKey>, DbErr> {
        let rows: Vec<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ApiKey::from).collect())
    }

    async fn list_keys_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys")
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(ApiKey::from).collect(),
            total: total.0,
            page,
            page_size,
        })
    }

    async fn list_keys_paginated_for_user(&self, created_by: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE created_by = ?")
            .bind(created_by)
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys WHERE created_by = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(created_by)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(ApiKey::from).collect(),
            total: total.0,
            page,
            page_size,
        })
    }

    async fn update_key(&self, key: &ApiKey) -> Result<ApiKey, DbErr> {
        sqlx::query(
            "UPDATE api_keys SET name = ?, key_hash = ?, rate_limit = ?, budget_monthly = ?,
             enabled = ?, created_by = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled as i64)
        .bind(&key.created_by)
        .bind(key.updated_at.to_rfc3339())
        .bind(&key.id)
        .execute(&self.pool)
        .await?;

        Ok(key.clone())
    }

    async fn delete_key(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Providers ----

    async fn create_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "INSERT INTO providers (id, name, slug, base_url, endpoints, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(&provider.base_url)
        .bind(&provider.endpoints)
        .bind(provider.enabled as i64)
        .bind(provider.created_at.to_rfc3339())
        .bind(provider.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }

    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, DbErr> {
        let row: Option<SqliteProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, enabled, created_at, updated_at
             FROM providers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Provider::from))
    }

    async fn list_providers(&self) -> Result<Vec<Provider>, DbErr> {
        let rows: Vec<SqliteProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, enabled, created_at, updated_at
             FROM providers",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Provider::from).collect())
    }

    async fn update_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "UPDATE providers SET name = ?, slug = ?, base_url = ?, endpoints = ?,
             enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(&provider.base_url)
        .bind(&provider.endpoints)
        .bind(provider.enabled as i64)
        .bind(provider.updated_at.to_rfc3339())
        .bind(&provider.id)
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }

    async fn delete_provider(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Channels ----

    async fn create_channel(&self, channel: &Channel) -> Result<Channel, DbErr> {
        sqlx::query(
            "INSERT INTO channels (id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&channel.id)
        .bind(&channel.provider_id)
        .bind(&channel.name)
        .bind(&channel.api_key)
        .bind(&channel.base_url)
        .bind(channel.priority)
        .bind(&channel.pricing_policy_id)
        .bind(channel.markup_ratio)
        .bind(channel.enabled as i64)
        .bind(channel.rpm_limit)
        .bind(channel.tpm_limit)
        .bind(channel.balance)
        .bind(channel.weight.unwrap_or(100))
        .bind(channel.created_at.to_rfc3339())
        .bind(channel.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(channel.clone())
    }

    async fn get_channel(&self, id: &str) -> Result<Option<Channel>, DbErr> {
        let row: Option<SqliteChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Channel::from))
    }

    async fn list_channels(&self) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<SqliteChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels ORDER BY priority ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn list_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<SqliteChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels WHERE provider_id = ? ORDER BY priority ASC",
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn list_enabled_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<SqliteChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels WHERE provider_id = ? AND enabled = 1 ORDER BY priority ASC",
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn update_channel(&self, channel: &Channel) -> Result<Channel, DbErr> {
        sqlx::query(
            "UPDATE channels SET name = ?, api_key = ?, base_url = ?, priority = ?, pricing_policy_id = ?, markup_ratio = ?,
             enabled = ?, rpm_limit = ?, tpm_limit = ?, balance = ?, weight = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&channel.name)
        .bind(&channel.api_key)
        .bind(&channel.base_url)
        .bind(channel.priority)
        .bind(&channel.pricing_policy_id)
        .bind(channel.markup_ratio)
        .bind(channel.enabled as i64)
        .bind(channel.rpm_limit)
        .bind(channel.tpm_limit)
        .bind(channel.balance)
        .bind(channel.weight)
        .bind(channel.updated_at.to_rfc3339())
        .bind(&channel.id)
        .execute(&self.pool)
        .await?;

        Ok(channel.clone())
    }

    async fn delete_channel(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM channels WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Models ----

    async fn create_model(&self, model: &Model) -> Result<Model, DbErr> {
        sqlx::query(
            "INSERT INTO models (id, name, model_type, billing_type, input_price, output_price, request_price, enabled, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&model.id)
        .bind(&model.name)
        .bind(&model.model_type)
        .bind(&model.billing_type)
        .bind(model.input_price)
        .bind(model.output_price)
        .bind(model.request_price)
        .bind(model.enabled as i64)
        .bind(model.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(model.clone())
    }

    async fn get_model(&self, name: &str) -> Result<Option<Model>, DbErr> {
        let row: Option<SqliteModelRow> = sqlx::query_as(
            "SELECT id, name, model_type, billing_type, input_price, output_price, request_price, enabled, created_at
             FROM models WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Model::from))
    }

    async fn get_model_by_provider(&self, _provider_id: &str, _name: &str) -> Result<Option<Model>, DbErr> {
        // No longer supported - models are now N:N with providers via model_providers table
        Ok(None)
    }

    async fn list_models(&self) -> Result<Vec<ModelWithProvider>, DbErr> {
        // For N:N architecture, list all enabled models with provider info via model_providers
        // This requires a JOIN with model_providers and providers tables
        // For now, return empty list - actual implementation needs model_providers table
        Ok(vec![])
    }

    async fn list_models_by_provider(&self, _provider_id: &str) -> Result<Vec<Model>, DbErr> {
        // No longer supported - models are now N:N with providers
        Ok(vec![])
    }

    async fn update_model(&self, model: &Model) -> Result<Model, DbErr> {
        sqlx::query(
            "UPDATE models SET billing_type = ?, input_price = ?, output_price = ?,
             request_price = ?, enabled = ? WHERE name = ?",
        )
        .bind(&model.billing_type)
        .bind(model.input_price)
        .bind(model.output_price)
        .bind(model.request_price)
        .bind(model.enabled as i64)
        .bind(&model.name)
        .execute(&self.pool)
        .await?;

        Ok(model.clone())
    }

    async fn delete_model(&self, name: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM models WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Key-Model Rate Limits ----

    async fn set_key_model_rate_limit(&self, limit: &KeyModelRateLimit) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO key_model_rate_limits (key_id, model_id, rpm, tpm)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(key_id, model_id) DO UPDATE SET rpm = ?, tpm = ?",
        )
        .bind(&limit.key_id)
        .bind(&limit.model_id)
        .bind(limit.rpm)
        .bind(limit.tpm)
        .bind(limit.rpm)
        .bind(limit.tpm)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_key_model_rate_limit(
        &self,
        key_id: &str,
        model_id: &str,
    ) -> Result<Option<KeyModelRateLimit>, DbErr> {
        let row: Option<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT key_id, model_id, rpm, tpm FROM key_model_rate_limits
             WHERE key_id = ? AND model_id = ?",
        )
        .bind(key_id)
        .bind(model_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| KeyModelRateLimit {
            key_id: r.0,
            model_id: r.1,
            rpm: r.2,
            tpm: r.3,
        }))
    }

    async fn list_key_model_rate_limits(&self, key_id: &str) -> Result<Vec<KeyModelRateLimit>, DbErr> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT key_id, model_id, rpm, tpm FROM key_model_rate_limits WHERE key_id = ?",
        )
        .bind(key_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| KeyModelRateLimit {
                key_id: r.0,
                model_id: r.1,
                rpm: r.2,
                tpm: r.3,
            })
            .collect())
    }

    async fn delete_key_model_rate_limit(&self, key_id: &str, model_id: &str) -> Result<(), DbErr> {
        sqlx::query(
            "DELETE FROM key_model_rate_limits WHERE key_id = ? AND model_id = ?",
        )
        .bind(key_id)
        .bind(model_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- Usage ----

    async fn record_usage(&self, usage: &UsageRecord) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO usage_records (id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cost, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&usage.id)
        .bind(&usage.key_id)
        .bind(&usage.model_name)
        .bind(&usage.provider_id)
        .bind(&usage.channel_id)
        .bind(protocol_str(&usage.protocol))
        .bind(usage.input_tokens)
        .bind(usage.output_tokens)
        .bind(usage.cost)
        .bind(usage.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn query_usage(&self, filter: &UsageFilter) -> Result<Vec<UsageRecord>, DbErr> {
        let mut sql = String::from(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cost, created_at
             FROM usage_records WHERE 1=1",
        );
        let mut bind_vars: Vec<String> = Vec::new();

        if let Some(ref key_id) = filter.key_id {
            sql.push_str(" AND key_id = ?");
            bind_vars.push(key_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            sql.push_str(" AND model_name = ?");
            bind_vars.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            sql.push_str(" AND created_at >= ?");
            bind_vars.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            sql.push_str(" AND created_at <= ?");
            bind_vars.push(until.to_rfc3339());
        }

        sql.push_str(" ORDER BY created_at DESC");

        let mut query = sqlx::query_as::<_, SqliteUsageRow>(&sql);
        for var in bind_vars {
            query = query.bind(var);
        }

        let rows: Vec<SqliteUsageRow> = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(UsageRecord::from).collect())
    }

    async fn query_usage_paginated(&self, filter: &UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let mut where_sql = String::from("WHERE 1=1");
        let mut bind_vars: Vec<String> = Vec::new();

        if let Some(ref key_id) = filter.key_id {
            where_sql.push_str(" AND key_id = ?");
            bind_vars.push(key_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            where_sql.push_str(" AND model_name = ?");
            bind_vars.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            where_sql.push_str(" AND created_at >= ?");
            bind_vars.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            where_sql.push_str(" AND created_at <= ?");
            bind_vars.push(until.to_rfc3339());
        }

        let count_sql = format!("SELECT COUNT(*) FROM usage_records {}", where_sql);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        for var in &bind_vars {
            count_query = count_query.bind(var.clone());
        }
        let total = count_query.fetch_one(&self.pool).await?.0;

        let offset = (page - 1) * page_size;
        let data_sql = format!(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cost, created_at \
             FROM usage_records {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_sql
        );
        let mut data_query = sqlx::query_as::<_, SqliteUsageRow>(&data_sql);
        for var in bind_vars {
            data_query = data_query.bind(var);
        }
        data_query = data_query.bind(page_size).bind(offset);
        let rows = data_query.fetch_all(&self.pool).await?;

        Ok(PaginatedResponse {
            items: rows.into_iter().map(UsageRecord::from).collect(),
            total,
            page,
            page_size,
        })
    }

    // ---- Audit ----

    async fn insert_log(&self, log: &AuditLog) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.key_id)
        .bind(&log.model_name)
        .bind(&log.provider_id)
        .bind(&log.channel_id)
        .bind(protocol_str(&log.protocol))
        .bind(log.stream)
        .bind(&log.request_body)
        .bind(&log.response_body)
        .bind(log.status_code)
        .bind(log.latency_ms)
        .bind(log.input_tokens)
        .bind(log.output_tokens)
        .bind(log.created_at.to_rfc3339())
        .bind(&log.original_model)
        .bind(&log.upstream_model)
        .bind(&log.model_override_reason)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<AuditLog>, DbErr> {
        let mut sql = String::from(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason
             FROM audit_logs WHERE 1=1",
        );
        let mut bind_vars: Vec<String> = Vec::new();

        if let Some(ref key_id) = filter.key_id {
            sql.push_str(" AND key_id = ?");
            bind_vars.push(key_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            sql.push_str(" AND model_name = ?");
            bind_vars.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            sql.push_str(" AND created_at >= ?");
            bind_vars.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            sql.push_str(" AND created_at <= ?");
            bind_vars.push(until.to_rfc3339());
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let mut query = sqlx::query_as::<_, SqliteAuditRow>(&sql);
        for var in bind_vars {
            query = query.bind(var);
        }

        let rows: Vec<SqliteAuditRow> = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(AuditLog::from).collect())
    }

    async fn query_logs_paginated(&self, filter: &LogFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        let mut where_sql = String::from("WHERE 1=1");
        let mut bind_vars: Vec<String> = Vec::new();

        if let Some(ref key_id) = filter.key_id {
            where_sql.push_str(" AND key_id = ?");
            bind_vars.push(key_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            where_sql.push_str(" AND model_name = ?");
            bind_vars.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            where_sql.push_str(" AND created_at >= ?");
            bind_vars.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            where_sql.push_str(" AND created_at <= ?");
            bind_vars.push(until.to_rfc3339());
        }

        let count_sql = format!("SELECT COUNT(*) FROM audit_logs {}", where_sql);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        for var in &bind_vars {
            count_query = count_query.bind(var.clone());
        }
        let total = count_query.fetch_one(&self.pool).await?.0;

        let offset = (page - 1) * page_size;
        let data_sql = format!(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body, \
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason \
             FROM audit_logs {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_sql
        );
        let mut data_query = sqlx::query_as::<_, SqliteAuditRow>(&data_sql);
        for var in bind_vars {
            data_query = data_query.bind(var);
        }
        data_query = data_query.bind(page_size).bind(offset);
        let rows = data_query.fetch_all(&self.pool).await?;

        Ok(PaginatedResponse {
            items: rows.into_iter().map(AuditLog::from).collect(),
            total,
            page,
            page_size,
        })
    }

    async fn increment_rate_limit_counter(
        &self,
        key_id: &str,
        model_name: &str,
        window: &str,
    ) -> Result<i64, DbErr> {
        sqlx::query(
            "INSERT INTO rate_limit_counters (key_id, model_name, window, count) VALUES (?, ?, ?, 1)
             ON CONFLICT(key_id, model_name, window) DO UPDATE SET count = count + 1",
        )
        .bind(key_id)
        .bind(model_name)
        .bind(window)
        .execute(&self.pool)
        .await?;

        let count: (i64,) = sqlx::query_as(
            "SELECT count FROM rate_limit_counters WHERE key_id = ? AND model_name = ? AND window = ?",
        )
        .bind(key_id)
        .bind(model_name)
        .bind(window)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    async fn get_rate_limit_counter(
        &self,
        key_id: &str,
        model_name: &str,
        window: &str,
    ) -> Result<i64, DbErr> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT count FROM rate_limit_counters WHERE key_id = ? AND model_name = ? AND window = ?",
        )
        .bind(key_id)
        .bind(model_name)
        .bind(window)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0).unwrap_or(0))
    }

    // ---- Users ----

    async fn create_user(&self, user: &User) -> Result<User, DbErr> {
        sqlx::query(
            "INSERT INTO users (id, username, password, role, enabled, refresh_token, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.password)
        .bind(&user.role)
        .bind(user.enabled as i64)
        .bind(&user.refresh_token)
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(user.clone())
    }

    async fn get_user(&self, id: &str) -> Result<Option<User>, DbErr> {
        let row: Option<SqliteUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, refresh_token, created_at, updated_at FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(User::from))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, DbErr> {
        let row: Option<SqliteUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, refresh_token, created_at, updated_at FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(User::from))
    }

    async fn list_users(&self) -> Result<Vec<User>, DbErr> {
        let rows: Vec<SqliteUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, refresh_token, created_at, updated_at FROM users",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(User::from).collect())
    }

    async fn list_users_paginated(&self, page: i64, page_size: i64) -> Result<PaginatedResponse<User>, Box<dyn std::error::Error + Send + Sync>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<SqliteUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, refresh_token, created_at, updated_at \
             FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(User::from).collect(),
            total: total.0,
            page,
            page_size,
        })
    }

    async fn update_user(&self, user: &User) -> Result<User, DbErr> {
        sqlx::query(
            "UPDATE users SET username = ?, password = ?, role = ?, enabled = ?, refresh_token = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&user.username)
        .bind(&user.password)
        .bind(&user.role)
        .bind(user.enabled as i64)
        .bind(&user.refresh_token)
        .bind(user.updated_at.to_rfc3339())
        .bind(&user.id)
        .execute(&self.pool)
        .await?;
        Ok(user.clone())
    }

    async fn delete_user(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Channel Models ----

    async fn create_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr> {
        sqlx::query(
            "INSERT INTO channel_models (id, channel_id, model_id, upstream_model_name, priority_override, cost_policy_id, markup_ratio, billing_type, input_price, output_price, request_price, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&cm.id)
        .bind(&cm.channel_id)
        .bind(&cm.model_id)
        .bind(&cm.upstream_model_name)
        .bind(cm.priority_override)
        .bind(&cm.cost_policy_id)
        .bind(cm.markup_ratio)
        .bind(&cm.billing_type)
        .bind(cm.input_price)
        .bind(cm.output_price)
        .bind(cm.request_price)
        .bind(cm.enabled as i64)
        .bind(cm.created_at.to_rfc3339())
        .bind(cm.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(cm.clone())
    }

    async fn get_channel_model(&self, id: &str) -> Result<Option<ChannelModel>, DbErr> {
        let row: Option<SqliteChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, cost_policy_id, markup_ratio, billing_type, input_price, output_price, request_price, enabled, created_at, updated_at
             FROM channel_models WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ChannelModel::from))
    }

    async fn list_channel_models(&self) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<SqliteChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, cost_policy_id, markup_ratio, billing_type, input_price, output_price, request_price, enabled, created_at, updated_at
             FROM channel_models",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn list_channel_models_by_channel(&self, channel_id: &str) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<SqliteChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, cost_policy_id, markup_ratio, billing_type, input_price, output_price, request_price, enabled, created_at, updated_at
             FROM channel_models WHERE channel_id = ?",
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn get_channel_models_for_model(&self, model_id: &str) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<SqliteChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, cost_policy_id, markup_ratio, billing_type, input_price, output_price, request_price, enabled, created_at, updated_at
             FROM channel_models WHERE model_id = ?",
        )
        .bind(model_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn get_channels_for_model(&self, model_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<SqliteChannelRow> = sqlx::query_as(
            "SELECT c.id, c.provider_id, c.name, c.api_key, c.base_url, c.priority, c.pricing_policy_id, c.markup_ratio, c.enabled, c.rpm_limit, c.tpm_limit, c.balance, c.weight, c.created_at, c.updated_at
             FROM channels c
             JOIN channel_models cm ON c.id = cm.channel_id
             WHERE cm.model_id = ? AND c.enabled = 1",
        )
        .bind(model_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn update_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr> {
        sqlx::query(
            "UPDATE channel_models SET channel_id = ?, model_id = ?, upstream_model_name = ?,
             priority_override = ?, cost_policy_id = ?, markup_ratio = ?, billing_type = ?, input_price = ?, output_price = ?, request_price = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&cm.channel_id)
        .bind(&cm.model_id)
        .bind(&cm.upstream_model_name)
        .bind(cm.priority_override)
        .bind(&cm.cost_policy_id)
        .bind(cm.markup_ratio)
        .bind(&cm.billing_type)
        .bind(cm.input_price)
        .bind(cm.output_price)
        .bind(cm.request_price)
        .bind(cm.enabled as i64)
        .bind(cm.updated_at.to_rfc3339())
        .bind(&cm.id)
        .execute(&self.pool)
        .await?;

        Ok(cm.clone())
    }

    async fn delete_channel_model(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM channel_models WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_admin_users(&self) -> Result<i64, DbErr> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND enabled = 1",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn user_count(&self) -> Result<i64, DbErr> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn rotate_refresh_token(&self, user_id: &str, old_token: &str, new_token: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE users SET refresh_token = ?, updated_at = ? WHERE id = ? AND refresh_token = ?",
        )
        .bind(new_token)
        .bind(&now)
        .bind(user_id)
        .bind(old_token)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ---- Settings ----

    async fn get_setting(&self, key: &str) -> Result<Option<String>, DbErr> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM settings WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = ?",
        )
        .bind(key)
        .bind(value)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- Pricing Policies ----

    async fn create_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, DbErr> {
        sqlx::query(
            "INSERT INTO pricing_policies (id, name, billing_type, config, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&policy.id)
        .bind(&policy.name)
        .bind(&policy.billing_type)
        .bind(policy.config.to_string())
        .bind(policy.created_at.to_rfc3339())
        .bind(policy.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(policy.clone())
    }

    async fn get_pricing_policy(&self, id: &str) -> Result<Option<PricingPolicy>, DbErr> {
        let row: Option<SqlitePricingPolicyRow> = sqlx::query_as(
            "SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(PricingPolicy::from))
    }

    async fn list_pricing_policies(&self) -> Result<Vec<PricingPolicy>, DbErr> {
        let rows: Vec<SqlitePricingPolicyRow> = sqlx::query_as(
            "SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(PricingPolicy::from).collect())
    }

    async fn update_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, DbErr> {
        sqlx::query(
            "UPDATE pricing_policies SET name = ?, billing_type = ?, config = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&policy.name)
        .bind(&policy.billing_type)
        .bind(policy.config.to_string())
        .bind(policy.updated_at.to_rfc3339())
        .bind(&policy.id)
        .execute(&self.pool)
        .await?;

        Ok(policy.clone())
    }

    async fn delete_pricing_policy(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM pricing_policies WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Seed Data ----

    async fn seed_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check if providers already exist (idempotent)
        let existing = self.list_providers().await?;
        if existing.is_empty() {
            // Get seed providers and insert them
            let seed_providers = seed::get_seed_providers();
            let mut inserted_providers = Vec::new();
            for provider in seed_providers {
                let inserted = self.create_provider(&provider).await?;
                inserted_providers.push(inserted);
            }

            // Build provider ID map and get seed models
            let provider_ids = seed::build_provider_id_map(&inserted_providers);
            let seed_models = seed::get_seed_models(&provider_ids);

            // Insert seed models
            for model in seed_models {
                self.create_model(&model).await?;
            }
        }

        // Seed default settings for audit logs (idempotent - uses ON CONFLICT)
        self.set_setting("audit_log_request", "true").await?;
        self.set_setting("audit_log_response", "true").await?;

        Ok(())
    }
}
