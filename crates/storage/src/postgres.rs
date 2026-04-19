use crate::types::*;
use async_trait::async_trait;
use sqlx::postgres::PgPool;
use sqlx::FromRow;

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(connection_string: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pool = PgPool::connect(connection_string).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

// ---------------------------------------------------------------------------
// PostgreSQL row types (booleans, TIMESTAMP WITH TIME ZONE)
// ---------------------------------------------------------------------------

#[derive(FromRow)]
struct PgKeyRow {
    id: String,
    name: String,
    key_hash: String,
    rate_limit: Option<i64>,
    budget_monthly: Option<f64>,
    enabled: bool,
    created_by: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgKeyRow> for ApiKey {
    fn from(r: PgKeyRow) -> Self {
        ApiKey {
            id: r.id,
            name: r.name,
            key_hash: r.key_hash,
            rate_limit: r.rate_limit,
            budget_monthly: r.budget_monthly,
            enabled: r.enabled,
            created_by: r.created_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgProviderRow {
    id: String,
    name: String,
    slug: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    endpoints: Option<String>,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgProviderRow> for Provider {
    fn from(r: PgProviderRow) -> Self {
        Provider {
            id: r.id,
            name: r.name,
            slug: r.slug,
            endpoints: r.endpoints,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgModelRow {
    id: String,
    name: String,
    model_type: Option<String>,
    pricing_policy_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow)]
struct PgModelEnrichedRow {
    id: String,
    name: String,
    model_type: Option<String>,
    pp_id: Option<String>,
    pp_name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    channel_ids_csv: Option<String>,
    channel_names_csv: Option<String>,
}

impl From<PgModelRow> for Model {
    fn from(r: PgModelRow) -> Self {
        Model {
            id: r.id,
            name: r.name,
            model_type: r.model_type,
            pricing_policy_id: r.pricing_policy_id,
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct PgModelWithProviderRow {
    id: String,
    name: String,
    model_type: Option<String>,
    billing_type: String,
    input_price: f64,
    output_price: f64,
    request_price: f64,
    pricing_policy_id: Option<String>,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    provider_name: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    endpoints: Option<String>,
}

impl From<PgModelWithProviderRow> for ModelWithProvider {
    fn from(r: PgModelWithProviderRow) -> Self {
        // Parse endpoints JSON to determine compatibility
        // If provider name is "Anthropic", it's natively compatible with Anthropic API
        // If provider name is "OpenAI", it's natively compatible with OpenAI API
        let provider_name_lower = r.provider_name.to_lowercase();
        let is_native_anthropic = provider_name_lower == "anthropic";
        let is_native_openai = provider_name_lower == "openai";

        let openai_compatible = r.endpoints.as_ref()
            .and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok())
            .map(|v| v.get("openai").is_some())
            .unwrap_or(is_native_openai);
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
                pricing_policy_id: r.pricing_policy_id,
                enabled: r.enabled,
                created_at: r.created_at,
            },
            provider_name: r.provider_name,
            openai_compatible,
            anthropic_compatible,
        }
    }
}

#[derive(FromRow)]
struct PgUsageRow {
    id: String,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    protocol: String,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cost: f64,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgUsageRow> for UsageRecord {
    fn from(r: PgUsageRow) -> Self {
        UsageRecord {
            id: r.id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            protocol: parse_protocol(&r.protocol),
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cache_read_tokens: r.cache_read_tokens,
            cost: r.cost,
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct PgUsageSummaryRow {
    model_name: String,
    total_input_tokens: i64,
    total_cache_read_tokens: i64,
    total_output_tokens: i64,
    total_cost: f64,
    request_count: i64,
}

impl From<PgUsageSummaryRow> for UsageSummaryRecord {
    fn from(r: PgUsageSummaryRow) -> Self {
        UsageSummaryRecord {
            model_name: r.model_name,
            total_input_tokens: r.total_input_tokens,
            total_cache_read_tokens: r.total_cache_read_tokens,
            total_output_tokens: r.total_output_tokens,
            total_cost: r.total_cost,
            request_count: r.request_count,
        }
    }
}

#[derive(FromRow)]
struct PgAuditRow {
    id: String,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    protocol: String,
    stream: bool,
    request_body: String,
    response_body: String,
    status_code: i32,
    latency_ms: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    original_model: Option<String>,
    upstream_model: Option<String>,
    model_override_reason: Option<String>,
    request_path: Option<String>,
    upstream_url: Option<String>,
    request_headers: Option<String>,
    response_headers: Option<String>,
}

impl From<PgAuditRow> for AuditLog {
    fn from(r: PgAuditRow) -> Self {
        AuditLog {
            id: r.id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            protocol: parse_protocol(&r.protocol),
            stream: r.stream,
            request_body: r.request_body,
            response_body: r.response_body,
            status_code: r.status_code,
            latency_ms: r.latency_ms,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            created_at: r.created_at,
            original_model: r.original_model,
            upstream_model: r.upstream_model,
            model_override_reason: r.model_override_reason,
            request_path: r.request_path,
            upstream_url: r.upstream_url,
            request_headers: r.request_headers,
            response_headers: r.response_headers,
        }
    }
}

#[derive(FromRow)]
struct PgChannelRow {
    id: String,
    provider_id: String,
    name: String,
    api_key: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    priority: i32,
    pricing_policy_id: Option<String>,
    markup_ratio: f64,
    enabled: bool,
    rpm_limit: Option<i64>,
    tpm_limit: Option<i64>,
    balance: Option<f64>,
    weight: Option<i32>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgChannelRow> for Channel {
    fn from(r: PgChannelRow) -> Self {
        Channel {
            id: r.id,
            provider_id: r.provider_id,
            name: r.name,
            api_key: r.api_key,
            priority: r.priority,
            pricing_policy_id: r.pricing_policy_id,
            markup_ratio: r.markup_ratio,
            enabled: r.enabled,
            rpm_limit: r.rpm_limit,
            tpm_limit: r.tpm_limit,
            balance: r.balance,
            weight: r.weight,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgUserRow {
    id: String,
    username: String,
    password: String,
    role: String,
    enabled: bool,
    refresh_token: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgUserRow> for User {
    fn from(r: PgUserRow) -> Self {
        User {
            id: r.id,
            username: r.username,
            password: r.password,
            role: r.role,
            enabled: r.enabled,
            refresh_token: r.refresh_token,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgChannelModelRow {
    id: String,
    channel_id: String,
    model_id: String,
    upstream_model_name: Option<String>,
    priority_override: Option<i32>,
    pricing_policy_id: Option<String>,
    markup_ratio: f64,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgChannelModelRow> for ChannelModel {
    fn from(r: PgChannelModelRow) -> Self {
        ChannelModel {
            id: r.id,
            channel_id: r.channel_id,
            model_id: r.model_id,
            upstream_model_name: r.upstream_model_name,
            priority_override: r.priority_override,
            pricing_policy_id: r.pricing_policy_id,
            markup_ratio: r.markup_ratio,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgPricingPolicyRow {
    id: String,
    name: String,
    billing_type: String,
    config: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgPricingPolicyRow> for PricingPolicy {
    fn from(r: PgPricingPolicyRow) -> Self {
        PricingPolicy {
            id: r.id,
            name: r.name,
            billing_type: r.billing_type,
            config: serde_json::from_str(&r.config).unwrap_or(serde_json::Value::Null),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_protocol(s: &str) -> Protocol {
    match s {
        "openai" => Protocol::Openai,
        "anthropic" => Protocol::Anthropic,
        _ => Protocol::Openai,
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

#[async_trait]
impl crate::Storage for PostgresStorage {
    // ---- Migrations ----

    async fn run_migrations(&self) -> Result<(), DbErr> {
        let migrator = sqlx::migrate!("./migrations/postgres");
        migrator.run(&self.pool).await.map_err(|e: sqlx::migrate::MigrateError| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(())
    }

    // ---- API Keys ----

    async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, DbErr> {
        sqlx::query(
            "INSERT INTO api_keys (id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&key.id)
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled)
        .bind(&key.created_by)
        .bind(key.created_at)
        .bind(key.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(key.clone())
    }

    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<PgKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<PgKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys WHERE key_hash = $1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn list_keys(&self) -> Result<Vec<ApiKey>, DbErr> {
        let rows: Vec<PgKeyRow> = sqlx::query_as(
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
        let rows: Vec<PgKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys ORDER BY created_at DESC LIMIT $1 OFFSET $2",
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
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE created_by = $1")
            .bind(created_by)
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<PgKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at
             FROM api_keys WHERE created_by = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
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
            "UPDATE api_keys SET name = $1, key_hash = $2, rate_limit = $3, budget_monthly = $4,
             enabled = $5, created_by = $6, updated_at = $7 WHERE id = $8",
        )
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled)
        .bind(&key.created_by)
        .bind(key.updated_at)
        .bind(&key.id)
        .execute(&self.pool)
        .await?;

        Ok(key.clone())
    }

    async fn delete_key(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Providers ----

    async fn create_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "INSERT INTO providers (id, name, slug, base_url, endpoints, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(provider.enabled)
        .bind(provider.created_at)
        .bind(provider.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }

    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, DbErr> {
        let row: Option<PgProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, enabled, created_at, updated_at
             FROM providers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Provider::from))
    }

    async fn list_providers(&self) -> Result<Vec<Provider>, DbErr> {
        let rows: Vec<PgProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, enabled, created_at, updated_at
             FROM providers",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Provider::from).collect())
    }

    async fn update_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "UPDATE providers SET name = $1, slug = $2, base_url = $3, endpoints = $4,
             enabled = $5, updated_at = $6 WHERE id = $7",
        )
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(provider.enabled)
        .bind(provider.updated_at)
        .bind(&provider.id)
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }

    async fn delete_provider(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM providers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Channels ----

    async fn create_channel(&self, channel: &Channel) -> Result<Channel, DbErr> {
        sqlx::query(
            "INSERT INTO channels (id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(&channel.id)
        .bind(&channel.provider_id)
        .bind(&channel.name)
        .bind(&channel.api_key)
        .bind(None::<String>)
        .bind(channel.priority)
        .bind(&channel.pricing_policy_id)
        .bind(channel.markup_ratio)
        .bind(channel.enabled)
        .bind(channel.rpm_limit)
        .bind(channel.tpm_limit)
        .bind(channel.balance)
        .bind(channel.weight.unwrap_or(100))
        .bind(channel.created_at)
        .bind(channel.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(channel.clone())
    }

    async fn get_channel(&self, id: &str) -> Result<Option<Channel>, DbErr> {
        let row: Option<PgChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Channel::from))
    }

    async fn list_channels(&self) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels ORDER BY priority ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn list_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels WHERE provider_id = $1 ORDER BY priority ASC",
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn list_enabled_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
             FROM channels WHERE provider_id = $1 AND enabled = true ORDER BY priority ASC",
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn update_channel(&self, channel: &Channel) -> Result<Channel, DbErr> {
        sqlx::query(
            "UPDATE channels SET name = $1, api_key = $2, base_url = $3, priority = $4, pricing_policy_id = $5, markup_ratio = $6,
             enabled = $7, rpm_limit = $8, tpm_limit = $9, balance = $10, weight = $11, updated_at = $12 WHERE id = $13",
        )
        .bind(&channel.name)
        .bind(&channel.api_key)
        .bind(None::<String>)
        .bind(channel.priority)
        .bind(&channel.pricing_policy_id)
        .bind(channel.markup_ratio)
        .bind(channel.enabled)
        .bind(channel.rpm_limit)
        .bind(channel.tpm_limit)
        .bind(channel.balance)
        .bind(channel.weight)
        .bind(channel.updated_at)
        .bind(&channel.id)
        .execute(&self.pool)
        .await?;

        Ok(channel.clone())
    }

    async fn delete_channel(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM channels WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Models ----

    async fn create_model(&self, model: &Model) -> Result<Model, DbErr> {
        sqlx::query(
            "INSERT INTO models (id, name, model_type, pricing_policy_id, enabled, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&model.id)
        .bind(&model.name)
        .bind(&model.model_type)
        .bind(&model.pricing_policy_id)
        .bind(model.created_at)
        .execute(&self.pool)
        .await?;

        Ok(model.clone())
    }

    async fn get_model(&self, name: &str) -> Result<Option<Model>, DbErr> {
        let row: Option<PgModelRow> = sqlx::query_as(
            "SELECT id, name, model_type, pricing_policy_id, created_at
             FROM models WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Model::from))
    }

    async fn get_model_by_id(&self, id: &str) -> Result<Option<Model>, DbErr> {
        let row: Option<PgModelRow> = sqlx::query_as(
            "SELECT id, name, model_type, pricing_policy_id, created_at
             FROM models WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Model::from))
    }

    async fn get_model_by_provider(&self, _provider_id: &str, _name: &str) -> Result<Option<Model>, DbErr> {
        // No longer supported - models are now N:N with providers
        Ok(None)
    }

    async fn list_models(&self) -> Result<Vec<ModelWithProvider>, DbErr> {
        let rows = sqlx::query_as::<_, PgModelEnrichedRow>(
            r#"
            SELECT
                m.id,
                m.name,
                m.model_type,
                m.pricing_policy_id AS pp_id,
                pp.name AS pp_name,
                m.created_at,
                STRING_AGG(DISTINCT cm.id, ',') AS channel_ids_csv,
                STRING_AGG(DISTINCT c.name, ',') AS channel_names_csv
            FROM models m
            LEFT JOIN pricing_policies pp ON m.pricing_policy_id = pp.id
            LEFT JOIN channel_models cm ON cm.model_id = m.id
            LEFT JOIN channels c ON c.id = cm.channel_id
            GROUP BY m.id
            ORDER BY m.name
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let result: Vec<ModelWithProvider> = rows.into_iter().map(|r| {
            let channel_ids: Vec<String> = r.channel_ids_csv
                .as_ref()
                .map(|s| s.split(',').filter(|x| !x.is_empty()).map(|x| x.to_string()).collect())
                .unwrap_or_default();
            let channel_names: Vec<String> = r.channel_names_csv
                .as_ref()
                .map(|s| s.split(',').filter(|x| !x.is_empty()).map(|x| x.to_string()).collect())
                .unwrap_or_default();

            ModelWithProvider {
                model: Model {
                    id: r.id,
                    name: r.name,
                    model_type: r.model_type,
                    pricing_policy_id: r.pp_id,
                    created_at: r.created_at,
                },
                pricing_policy_name: r.pp_name,
                channel_ids,
                channel_names,
            }
        }).collect();

        Ok(result)
    }

    async fn list_models_by_provider(&self, _provider_id: &str) -> Result<Vec<Model>, DbErr> {
        // No longer supported - models are now N:N with providers
        Ok(vec![])
    }

    async fn update_model(&self, model: &Model) -> Result<Model, DbErr> {
        sqlx::query(
            "UPDATE models SET pricing_policy_id = $1 WHERE name = $2",
        )
        .bind(&model.pricing_policy_id)
        .bind(&model.name)
        .execute(&self.pool)
        .await?;

        Ok(model.clone())
    }

    async fn delete_model(&self, name: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM models WHERE name = $1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Key-Model Rate Limits ----

    async fn set_key_model_rate_limit(&self, limit: &KeyModelRateLimit) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO key_model_rate_limits (key_id, model_id, rpm, tpm)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT(key_id, model_id) DO UPDATE SET rpm = $5, tpm = $6",
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
             WHERE key_id = $1 AND model_id = $2",
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
            "SELECT key_id, model_id, rpm, tpm FROM key_model_rate_limits WHERE key_id = $1",
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
            "DELETE FROM key_model_rate_limits WHERE key_id = $1 AND model_id = $2",
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
            "INSERT INTO usage_records (id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cost, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&usage.id)
        .bind(&usage.key_id)
        .bind(&usage.model_name)
        .bind(&usage.provider_id)
        .bind(&usage.channel_id)
        .bind(protocol_str(&usage.protocol))
        .bind(usage.input_tokens)
        .bind(usage.output_tokens)
        .bind(usage.cache_read_tokens)
        .bind(usage.cost)
        .bind(usage.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn query_usage(&self, filter: &UsageFilter) -> Result<Vec<UsageRecord>, DbErr> {
        // Build query dynamically based on filter - for now, just fetch all
        let rows: Vec<PgUsageRow> = sqlx::query_as(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cost, created_at
             FROM usage_records ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(UsageRecord::from).collect())
    }

    async fn query_usage_paginated(&self, filter: &UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conditions = Vec::new();
        let mut bind_vals: Vec<String> = Vec::new();

        if let Some(ref key_id) = filter.key_id {
            conditions.push(format!("key_id = ${}", bind_vals.len() + 1));
            bind_vals.push(key_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            conditions.push(format!("model_name = ${}", bind_vals.len() + 1));
            bind_vals.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            conditions.push(format!("created_at >= ${}", bind_vals.len() + 1));
            bind_vals.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            conditions.push(format!("created_at <= ${}", bind_vals.len() + 1));
            bind_vals.push(until.to_rfc3339());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM usage_records{}", where_clause);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        for val in &bind_vals {
            count_query = count_query.bind(val);
        }
        let total = count_query.fetch_one(&self.pool).await?.0;

        let offset = (page - 1) * page_size;
        let data_sql = format!(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cost, created_at \
             FROM usage_records{} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            bind_vals.len() + 1,
            bind_vals.len() + 2
        );
        let mut data_query = sqlx::query_as::<_, PgUsageRow>(&data_sql);
        for val in bind_vals {
            data_query = data_query.bind(val);
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

    async fn query_usage_summary(&self, filter: &UsageFilter) -> Result<Vec<UsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conditions = Vec::new();
        let mut bind_vals: Vec<String> = Vec::new();

        if let Some(ref key_id) = filter.key_id {
            conditions.push(format!("key_id = ${}", bind_vals.len() + 1));
            bind_vals.push(key_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            conditions.push(format!("model_name = ${}", bind_vals.len() + 1));
            bind_vals.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            conditions.push(format!("created_at >= ${}", bind_vals.len() + 1));
            bind_vals.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            conditions.push(format!("created_at <= ${}", bind_vals.len() + 1));
            bind_vals.push(until.to_rfc3339());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT \
               model_name, \
               COALESCE(SUM(input_tokens), 0) AS total_input_tokens, \
               COALESCE(SUM(cache_read_tokens), 0) AS total_cache_read_tokens, \
               COALESCE(SUM(output_tokens), 0) AS total_output_tokens, \
               COALESCE(SUM(cost), 0.0) AS total_cost, \
               COUNT(*) AS request_count \
             FROM usage_records{} \
             GROUP BY model_name \
             ORDER BY total_cost DESC",
            where_clause
        );

        let mut query = sqlx::query_as::<_, PgUsageSummaryRow>(&sql);
        for val in bind_vals {
            query = query.bind(val);
        }

        let rows: Vec<PgUsageSummaryRow> = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(UsageSummaryRecord::from).collect())
    }

    // ---- Audit ----

    async fn insert_log(&self, log: &AuditLog) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason,
             request_path, upstream_url, request_headers, response_headers)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)",
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
        .bind(log.created_at)
        .bind(&log.original_model)
        .bind(&log.upstream_model)
        .bind(&log.model_override_reason)
        .bind(&log.request_path)
        .bind(&log.upstream_url)
        .bind(&log.request_headers)
        .bind(&log.response_headers)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<AuditLog>, DbErr> {
        let mut sql = String::from(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason,
             request_path, upstream_url, request_headers, response_headers
             FROM audit_logs WHERE 1=1",
        );

        if filter.key_id.is_some() {
            sql.push_str(" AND key_id = $1");
        }
        if filter.model_name.is_some() {
            sql.push_str(" AND model_name = $2");
        }
        if filter.since.is_some() {
            sql.push_str(" AND created_at >= $3");
        }
        if filter.until.is_some() {
            sql.push_str(" AND created_at <= $4");
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let rows: Vec<PgAuditRow> = sqlx::query_as::<_, PgAuditRow>(&sql).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(AuditLog::from).collect())
    }

    async fn query_logs_paginated(&self, filter: &LogFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<PgAuditRow> = sqlx::query_as(
            "SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason,
             request_path, upstream_url, request_headers, response_headers
             FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(AuditLog::from).collect(),
            total: total.0,
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
            "INSERT INTO rate_limit_counters (key_id, model_name, window, count) VALUES ($1, $2, $3, 1)
             ON CONFLICT(key_id, model_name, window) DO UPDATE SET count = count + 1",
        )
        .bind(key_id)
        .bind(model_name)
        .bind(window)
        .execute(&self.pool)
        .await?;

        let count: (i64,) = sqlx::query_as(
            "SELECT count FROM rate_limit_counters WHERE key_id = $1 AND model_name = $2 AND window = $3",
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
            "SELECT count FROM rate_limit_counters WHERE key_id = $1 AND model_name = $2 AND window = $3",
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
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.password)
        .bind(&user.role)
        .bind(user.enabled)
        .bind(&user.refresh_token)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(user.clone())
    }

    async fn get_user(&self, id: &str) -> Result<Option<User>, DbErr> {
        let row: Option<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, refresh_token, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(User::from))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, DbErr> {
        let row: Option<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, refresh_token, created_at, updated_at FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(User::from))
    }

    async fn list_users(&self) -> Result<Vec<User>, DbErr> {
        let rows: Vec<PgUserRow> = sqlx::query_as(
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
        let rows: Vec<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, refresh_token, created_at, updated_at \
             FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
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
            "UPDATE users SET username = $1, password = $2, role = $3, enabled = $4, refresh_token = $5, updated_at = $6 WHERE id = $7",
        )
        .bind(&user.username)
        .bind(&user.password)
        .bind(&user.role)
        .bind(user.enabled)
        .bind(&user.refresh_token)
        .bind(user.updated_at)
        .bind(&user.id)
        .execute(&self.pool)
        .await?;
        Ok(user.clone())
    }

    async fn delete_user(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Channel Models ----

    async fn create_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr> {
        sqlx::query(
            "INSERT INTO channel_models (id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&cm.id)
        .bind(&cm.channel_id)
        .bind(&cm.model_id)
        .bind(&cm.upstream_model_name)
        .bind(cm.priority_override)
        .bind(&cm.pricing_policy_id)
        .bind(cm.markup_ratio)
        .bind(cm.enabled)
        .bind(cm.created_at)
        .bind(cm.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(cm.clone())
    }

    async fn get_channel_model(&self, id: &str) -> Result<Option<ChannelModel>, DbErr> {
        let row: Option<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ChannelModel::from))
    }

    async fn list_channel_models(&self) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn list_channel_models_by_channel(&self, channel_id: &str) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models WHERE channel_id = $1",
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn get_channel_models_for_model(&self, model_id: &str) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models WHERE model_id = $1",
        )
        .bind(model_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn get_channels_for_model(&self, model_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT c.id, c.provider_id, c.name, c.api_key, c.base_url, c.priority, c.pricing_policy_id, c.markup_ratio, c.enabled, c.rpm_limit, c.tpm_limit, c.balance, c.weight, c.created_at, c.updated_at
             FROM channels c
             JOIN channel_models cm ON c.id = cm.channel_id
             WHERE cm.model_id = $1 AND c.enabled = true",
        )
        .bind(model_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn update_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr> {
        sqlx::query(
            "UPDATE channel_models SET channel_id = $1, model_id = $2, upstream_model_name = $3,
             priority_override = $4, pricing_policy_id = $5, markup_ratio = $6, enabled = $7, updated_at = $8 WHERE id = $9",
        )
        .bind(&cm.channel_id)
        .bind(&cm.model_id)
        .bind(&cm.upstream_model_name)
        .bind(cm.priority_override)
        .bind(&cm.pricing_policy_id)
        .bind(cm.markup_ratio)
        .bind(cm.enabled)
        .bind(cm.updated_at)
        .bind(&cm.id)
        .execute(&self.pool)
        .await?;

        Ok(cm.clone())
    }

    async fn delete_channel_model(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM channel_models WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_admin_users(&self) -> Result<i64, DbErr> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND enabled = true",
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
        let now = chrono::Utc::now();
        let result = sqlx::query(
            "UPDATE users SET refresh_token = $1, updated_at = $2 WHERE id = $3 AND refresh_token = $4",
        )
        .bind(new_token)
        .bind(now)
        .bind(user_id)
        .bind(old_token)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ---- Settings ----

    async fn get_setting(&self, key: &str) -> Result<Option<String>, DbErr> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM settings WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $3",
        )
        .bind(key)
        .bind(value)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- Seed Data ----

    async fn seed_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::seed;

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

    // ---- Pricing Policies ----

    async fn create_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, DbErr> {
        sqlx::query(
            "INSERT INTO pricing_policies (id, name, billing_type, config, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&policy.id)
        .bind(&policy.name)
        .bind(&policy.billing_type)
        .bind(policy.config.to_string())
        .bind(policy.created_at)
        .bind(policy.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(policy.clone())
    }

    async fn get_pricing_policy(&self, id: &str) -> Result<Option<PricingPolicy>, DbErr> {
        let row: Option<PgPricingPolicyRow> = sqlx::query_as(
            "SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(PricingPolicy::from))
    }

    async fn list_pricing_policies(&self) -> Result<Vec<PricingPolicy>, DbErr> {
        let rows: Vec<PgPricingPolicyRow> = sqlx::query_as(
            "SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(PricingPolicy::from).collect())
    }

    async fn list_pricing_policies_with_counts(&self) -> Result<Vec<PricingPolicyWithCounts>, DbErr> {
        let rows: Vec<PgPricingPolicyRow> = sqlx::query_as(
            "SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let policy = PricingPolicy::from(row);
            let model_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM models WHERE pricing_policy_id = $1",
            )
            .bind(&policy.id)
            .fetch_one(&self.pool)
            .await?;
            let channel_model_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM channel_models WHERE pricing_policy_id = $1",
            )
            .bind(&policy.id)
            .fetch_one(&self.pool)
            .await?;
            results.push(PricingPolicyWithCounts {
                policy,
                model_count: model_count.0,
                channel_model_count: channel_model_count.0,
            });
        }
        Ok(results)
    }

    async fn update_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, DbErr> {
        sqlx::query(
            "UPDATE pricing_policies SET name = $1, billing_type = $2, config = $3, updated_at = $4 WHERE id = $5",
        )
        .bind(&policy.name)
        .bind(&policy.billing_type)
        .bind(policy.config.to_string())
        .bind(policy.updated_at)
        .bind(&policy.id)
        .execute(&self.pool)
        .await?;

        Ok(policy.clone())
    }

    async fn delete_pricing_policy(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM pricing_policies WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}