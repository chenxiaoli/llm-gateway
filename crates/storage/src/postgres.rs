use crate::types::*;
use async_trait::async_trait;
use sqlx::postgres::PgPool;
use sqlx::{Acquire, FromRow, Row};

/// Error returned when an org-private catalog entry would shadow a
/// platform-level entry with the same name/slug. Lives here (not in
/// `org::OrgError`) to avoid a circular `storage ↔ org` dependency.
#[derive(Debug)]
struct CatalogNameReserved(String);
impl std::fmt::Display for CatalogNameReserved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "catalog name reserved at platform level: {}", self.0)
    }
}
impl std::error::Error for CatalogNameReserved {}

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(connection_string: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pool = PgPool::connect(connection_string).await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
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
    org_id: String,
    name: String,
    key_hash: String,
    key_prefix: Option<String>,
    // Postgres stores this column as INTEGER (INT4); decoding it directly into
    // `i64` trips sqlx's strict type check. Read it as the native width and
    // widen at the ApiKey boundary.
    rate_limit: Option<i32>,
    budget_monthly: Option<i64>,
    enabled: bool,
    created_by: Option<String>,
    model_fallback_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgKeyRow> for ApiKey {
    fn from(r: PgKeyRow) -> Self {
        ApiKey {
            id: r.id,
            org_id: r.org_id,
            name: r.name,
            key_hash: r.key_hash,
            key_prefix: r.key_prefix,
            rate_limit: r.rate_limit.map(|i| i as i64),
            budget_monthly: r.budget_monthly,
            enabled: r.enabled,
            created_by: r.created_by,
            model_fallback_id: r.model_fallback_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PgKeyWithMtdRow {
    id: String,
    org_id: String,
    name: String,
    key_hash: String,
    key_prefix: Option<String>,
    rate_limit: Option<i32>,
    budget_monthly: Option<i64>,
    enabled: bool,
    created_by: Option<String>,
    model_fallback_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    mtd_units: i64,
}

impl From<PgKeyWithMtdRow> for crate::types::ApiKeyWithMtd {
    fn from(r: PgKeyWithMtdRow) -> Self {
        crate::types::ApiKeyWithMtd {
            key: ApiKey {
                id: r.id,
                org_id: r.org_id,
                name: r.name,
                key_hash: r.key_hash,
                key_prefix: r.key_prefix,
                rate_limit: r.rate_limit.map(|i| i as i64),
                budget_monthly: r.budget_monthly,
                enabled: r.enabled,
                created_by: r.created_by,
                model_fallback_id: r.model_fallback_id,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
            mtd_units: r.mtd_units,
        }
    }
}

#[derive(FromRow)]
struct PgProviderRow {
    id: String,
    owner_org_id: Option<String>,
    name: String,
    slug: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    endpoints: Option<String>,
    proxy_url: Option<String>,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgProviderRow> for Provider {
    fn from(r: PgProviderRow) -> Self {
        Provider {
            id: r.id,
            owner_org_id: r.owner_org_id,
            name: r.name,
            slug: r.slug,
            endpoints: r.endpoints,
            proxy_url: r.proxy_url,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgModelRow {
    id: String,
    owner_org_id: Option<String>,
    name: String,
    model_type: Option<String>,
    pricing_policy_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow)]
struct PgModelEnrichedRow {
    id: String,
    owner_org_id: Option<String>,
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
            owner_org_id: r.owner_org_id,
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
    owner_org_id: Option<String>,
    name: String,
    model_type: Option<String>,
    pricing_policy_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    provider_name: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    endpoints: Option<String>,
}

impl From<PgModelWithProviderRow> for ModelWithProvider {
    fn from(r: PgModelWithProviderRow) -> Self {
        ModelWithProvider {
            model: Model {
                id: r.id,
                owner_org_id: r.owner_org_id,
                name: r.name,
                model_type: r.model_type,
                pricing_policy_id: r.pricing_policy_id,
                created_at: r.created_at,
            },
            pricing_policy_name: None,
            channel_ids: Vec::new(),
            channel_names: Vec::new(),
        }
    }
}

#[derive(FromRow)]
struct PgUsageRow {
    id: String,
    org_id: String,
    request_id: Option<String>,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    protocol: String,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
    cost: i64,
    pricing_policy: Option<serde_json::Value>,
    weighted_tokens: i64,
    user_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgUsageRow> for UsageRecord {
    fn from(r: PgUsageRow) -> Self {
        UsageRecord {
            id: r.id,
            org_id: r.org_id,
            request_id: r.request_id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            protocol: parse_protocol(&r.protocol),
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cache_read_tokens: r.cache_read_tokens,
            cache_creation_tokens: r.cache_creation_tokens,
            cost: r.cost,
            pricing_policy: r.pricing_policy,
            weighted_tokens: r.weighted_tokens,
            user_id: r.user_id,
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct PgUsageSummaryRow {
    model_name: String,
    total_input_tokens: i64,
    total_cache_read_tokens: i64,
    total_cache_creation_tokens: i64,
    total_output_tokens: i64,
    total_cost: i64,
    request_count: i64,
}

impl From<PgUsageSummaryRow> for UsageSummaryRecord {
    fn from(r: PgUsageSummaryRow) -> Self {
        UsageSummaryRecord {
            model_name: r.model_name,
            total_input_tokens: r.total_input_tokens,
            total_cache_read_tokens: r.total_cache_read_tokens,
            total_cache_creation_tokens: r.total_cache_creation_tokens,
            total_output_tokens: r.total_output_tokens,
            total_cost: r.total_cost,
            request_count: r.request_count,
        }
    }
}

#[derive(FromRow)]
struct PgChannelUsageSummaryRow {
    channel_id: Option<String>,
    channel_name: Option<String>,
    total_requests: i64,
    total_cost: i64,
    total_input_tokens: i64,
    total_output_tokens: i64,
}

impl From<PgChannelUsageSummaryRow> for ChannelUsageSummaryRecord {
    fn from(r: PgChannelUsageSummaryRow) -> Self {
        ChannelUsageSummaryRecord {
            channel_id: r.channel_id,
            channel_name: r.channel_name,
            total_requests: r.total_requests,
            total_cost: r.total_cost,
            total_input_tokens: r.total_input_tokens,
            total_output_tokens: r.total_output_tokens,
        }
    }
}

#[derive(FromRow)]
struct PgAuditSummaryRow {
    id: String,
    request_id: Option<String>,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    channel_name: Option<String>,
    protocol: String,
    stream: bool,
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
    user_id: Option<String>,
    routes: Option<serde_json::Value>,
}

impl From<PgAuditSummaryRow> for AuditLogSummary {
    fn from(r: PgAuditSummaryRow) -> Self {
        AuditLogSummary {
            id: r.id,
            request_id: r.request_id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            channel_name: r.channel_name,
            protocol: parse_protocol(&r.protocol),
            stream: r.stream,
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
            user_id: r.user_id,
            routes: r.routes.and_then(|v| serde_json::from_value(v).ok()),
        }
    }
}

#[derive(FromRow)]
struct PgAuditRow {
    id: String,
    org_id: String,
    request_id: Option<String>,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    channel_name: Option<String>,
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
    user_id: Option<String>,
    actor_is_platform_admin: bool,
    routes: Option<serde_json::Value>,
}

impl From<PgAuditRow> for AuditLog {
    fn from(r: PgAuditRow) -> Self {
        AuditLog {
            id: r.id,
            org_id: r.org_id,
            request_id: r.request_id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            channel_name: r.channel_name,
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
            user_id: r.user_id,
            actor_is_platform_admin: r.actor_is_platform_admin,
            routes: r.routes.and_then(|v| serde_json::from_value(v).ok()),
        }
    }
}

#[derive(FromRow)]
struct PgChannelRow {
    id: String,
    org_id: String,
    provider_id: String,
    name: String,
    api_key: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    priority: i32,
    pricing_policy_id: Option<String>,
    markup_ratio: i64,
    enabled: bool,
    rpm_limit: Option<i64>,
    tpm_limit: Option<i64>,
    balance: Option<i64>,
    weight: Option<i32>,
    available_hours: Option<String>,
    created_by: Option<String>,
    group_id: Option<String>,
    disabled_until: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgChannelRow> for Channel {
    fn from(r: PgChannelRow) -> Self {
        Channel {
            id: r.id,
            org_id: r.org_id,
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
            available_hours: match r.available_hours {
                Some(s) if !s.is_empty() => serde_json::from_str(&s).ok(),
                _ => None,
            },
            created_by: r.created_by,
            group_id: r.group_id,
            disabled_until: r.disabled_until,
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
    platform_role: Option<String>,
    current_org_id: Option<String>,
    enabled: bool,
    refresh_token: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    // Phase 4:
    email: Option<String>,
    email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    requires_email_verification: bool,
    password_changed_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgUserRow> for User {
    fn from(r: PgUserRow) -> Self {
        User {
            id: r.id,
            username: r.username,
            password: r.password,
            platform_role: r.platform_role.and_then(|s| crate::types::PlatformRole::parse(&s)),
            current_org_id: r.current_org_id,
            enabled: r.enabled,
            refresh_token: r.refresh_token,
            created_at: r.created_at,
            updated_at: r.updated_at,
            email: r.email,
            email_verified_at: r.email_verified_at,
            requires_email_verification: r.requires_email_verification,
            password_changed_at: r.password_changed_at,
        }
    }
}

#[derive(FromRow)]
struct PgUserWithBalanceRow {
    id: String,
    username: String,
    role: String,
    enabled: bool,
    group_id: Option<String>,
    group_name: Option<String>,
    balance: Option<i64>,
    threshold: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgUserWithBalanceRow> for UserWithBalance {
    fn from(r: PgUserWithBalanceRow) -> Self {
        UserWithBalance {
            id: r.id,
            username: r.username,
            role: r.role,
            enabled: r.enabled,
            group_id: r.group_id,
            group_name: r.group_name,
            balance: r.balance.unwrap_or(0),
            threshold: r.threshold.unwrap_or(DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgGroupRow {
    id: String,
    org_id: String,
    name: String,
    description: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgGroupRow> for Group {
    fn from(r: PgGroupRow) -> Self {
        Group {
            id: r.id,
            org_id: r.org_id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct PgChannelModelRow {
    id: String,
    org_id: String,
    channel_id: String,
    model_id: String,
    upstream_model_name: Option<String>,
    priority_override: Option<i32>,
    pricing_policy_id: Option<String>,
    markup_ratio: i64,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgChannelModelRow> for ChannelModel {
    fn from(r: PgChannelModelRow) -> Self {
        ChannelModel {
            id: r.id,
            org_id: r.org_id,
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
    owner_org_id: Option<String>,
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
            owner_org_id: r.owner_org_id,
            name: r.name,
            billing_type: r.billing_type,
            config: serde_json::from_str(&r.config).unwrap_or(serde_json::Value::Null),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── Account rows ────────────────────────────────────────────────────────────────

#[derive(FromRow)]
struct PgAccountRow {
    id: String,
    org_id: String,
    user_id: String,
    balance: i64,
    threshold: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgAccountRow> for Account {
    fn from(r: PgAccountRow) -> Self {
        Account {
            id: r.id,
            org_id: r.org_id,
            user_id: r.user_id,
            balance: r.balance,
            threshold: r.threshold,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── Transaction rows ───────────────────────────────────────────────────────────

#[derive(FromRow)]
struct PgTransactionRow {
    id: String,
    org_id: String,
    account_id: String,
    transaction_type: String,
    amount: i64,
    balance_after: i64,
    description: Option<String>,
    reference_id: Option<String>,
    request_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgTransactionRow> for Transaction {
    fn from(r: PgTransactionRow) -> Self {
        let tt = match r.transaction_type.as_str() {
            "credit" => TransactionType::Credit,
            "debit" => TransactionType::Debit,
            "credit_adjustment" => TransactionType::CreditAdjustment,
            "debit_refund" => TransactionType::DebitRefund,
            _ => TransactionType::Debit,
        };
        Transaction {
            id: r.id,
            org_id: r.org_id,
            account_id: r.account_id,
            transaction_type: tt,
            amount: r.amount,
            balance_after: r.balance_after,
            description: r.description,
            reference_id: r.reference_id,
            request_id: r.request_id,
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct PgModelFallbackRow {
    id: String,
    name: String,
    config: String,
    created_by: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgModelFallbackRow> for ModelFallbackConfig {
    fn from(r: PgModelFallbackRow) -> Self {
        ModelFallbackConfig {
            id: r.id,
            name: r.name,
            config: serde_json::from_str(&r.config).unwrap_or_default(),
            created_by: r.created_by,
            created_at: r.created_at,
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

    async fn create_key(&self, org_id: &str, key: &ApiKey) -> Result<ApiKey, DbErr> {
        sqlx::query(
            "INSERT INTO api_keys (id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&key.id)
        .bind(org_id)
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(&key.key_prefix)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled)
        .bind(&key.created_by)
        .bind(&key.model_fallback_id)
        .bind(key.created_at)
        .bind(key.updated_at)
        .execute(&self.pool)
        .await?;

        let mut k = key.clone();
        k.org_id = org_id.to_string();
        Ok(k)
    }

    async fn get_key(&self, org_id: &str, id: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<PgKeyRow> = sqlx::query_as(
            "SELECT id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at
             FROM api_keys WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<PgKeyRow> = sqlx::query_as(
            "SELECT id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at
             FROM api_keys WHERE key_hash = $1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn list_keys(&self, org_id: &str) -> Result<Vec<ApiKey>, DbErr> {
        let rows: Vec<PgKeyRow> = sqlx::query_as(
            "SELECT id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at
             FROM api_keys WHERE org_id = $1",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ApiKey::from).collect())
    }

    async fn list_keys_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<PgKeyRow> = sqlx::query_as(
            "SELECT id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at
             FROM api_keys WHERE org_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(org_id)
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

    async fn list_keys_paginated_for_user(&self, org_id: &str, created_by: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE org_id = $1 AND created_by = $2")
            .bind(org_id)
            .bind(created_by)
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<PgKeyRow> = sqlx::query_as(
            "SELECT id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at
             FROM api_keys WHERE org_id = $1 AND created_by = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
        )
        .bind(org_id)
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

    async fn list_keys_paginated_with_mtd(
        &self,
        org_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<PaginatedResponse<crate::types::ApiKeyWithMtd>, DbErr> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let month_bucket = format!("{}", chrono::Utc::now().format("%Y-%m"));
        let rows: Vec<PgKeyWithMtdRow> = sqlx::query_as(
            "SELECT ak.id, ak.org_id, ak.name, ak.key_hash, ak.key_prefix,
                    ak.rate_limit, ak.budget_monthly, ak.enabled, ak.created_by,
                    ak.model_fallback_id, ak.created_at, ak.updated_at,
                    COALESCE(bc.accrued, 0) AS mtd_units
             FROM api_keys ak
             LEFT JOIN budget_counters bc
               ON bc.key_id = ak.id AND bc.month_bucket = $2
             WHERE ak.org_id = $1
             ORDER BY ak.created_at DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(org_id)
        .bind(&month_bucket)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(crate::types::ApiKeyWithMtd::from).collect(),
            total: total.0,
            page,
            page_size,
        })
    }

    async fn update_key(&self, org_id: &str, key: &ApiKey) -> Result<ApiKey, DbErr> {
        sqlx::query(
            "UPDATE api_keys SET name = $1, key_hash = $2, rate_limit = $3, budget_monthly = $4,
             enabled = $5, created_by = $6, model_fallback_id = $7, updated_at = $8 WHERE org_id = $9 AND id = $10",
        )
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled)
        .bind(&key.created_by)
        .bind(&key.model_fallback_id)
        .bind(key.updated_at)
        .bind(org_id)
        .bind(&key.id)
        .execute(&self.pool)
        .await?;

        let mut k = key.clone();
        k.org_id = org_id.to_string();
        Ok(k)
    }

    async fn delete_key(&self, org_id: &str, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM api_keys WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Providers (catalog: visibility filter + anti-shadowing) ----

    async fn create_provider(&self, viewer_org_id: &str, provider: &Provider) -> Result<Provider, DbErr> {
        // Anti-shadowing: an org-private provider may not reuse a platform-level slug.
        if let Some(_org) = &provider.owner_org_id {
            let collision: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM providers WHERE slug = $1 AND owner_org_id IS NULL",
            )
            .bind(&provider.slug)
            .fetch_optional(&self.pool)
            .await?;
            if collision.is_some() {
                return Err(Box::new(CatalogNameReserved(provider.slug.clone())));
            }
        }

        sqlx::query(
            "INSERT INTO providers (id, owner_org_id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&provider.id)
        .bind(&provider.owner_org_id)
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(&provider.proxy_url)
        .bind(provider.enabled)
        .bind(provider.created_at)
        .bind(provider.updated_at)
        .execute(&self.pool)
        .await?;

        let _ = viewer_org_id;
        Ok(provider.clone())
    }

    async fn get_provider(&self, viewer_org_id: &str, id: &str) -> Result<Option<Provider>, DbErr> {
        let row: Option<PgProviderRow> = sqlx::query_as(
            "SELECT id, owner_org_id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at
             FROM providers
             WHERE id = $1 AND (owner_org_id IS NULL OR owner_org_id = $2)",
        )
        .bind(id)
        .bind(viewer_org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Provider::from))
    }

    async fn list_providers(&self, viewer_org_id: &str) -> Result<Vec<Provider>, DbErr> {
        let rows: Vec<PgProviderRow> = sqlx::query_as(
            "SELECT id, owner_org_id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at
             FROM providers
             WHERE owner_org_id IS NULL OR owner_org_id = $1",
        )
        .bind(viewer_org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Provider::from).collect())
    }

    async fn update_provider(&self, viewer_org_id: &str, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "UPDATE providers SET name = $1, slug = $2, base_url = $3, endpoints = $4,
             proxy_url = $5, enabled = $6, updated_at = $7
             WHERE id = $8 AND (owner_org_id IS NULL OR owner_org_id = $9)",
        )
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(&provider.proxy_url)
        .bind(provider.enabled)
        .bind(provider.updated_at)
        .bind(&provider.id)
        .bind(viewer_org_id)
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }

    async fn delete_provider(&self, viewer_org_id: &str, id: &str) -> Result<(), DbErr> {
        sqlx::query(
            "DELETE FROM providers WHERE id = $1 AND (owner_org_id IS NULL OR owner_org_id = $2)",
        )
        .bind(id)
        .bind(viewer_org_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- Channels (tenant: org_id scoping) ----

    async fn create_channel(&self, org_id: &str, channel: &Channel) -> Result<Channel, DbErr> {
        sqlx::query(
            "INSERT INTO channels (id, org_id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, available_hours, created_by, group_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)",
        )
        .bind(&channel.id)
        .bind(org_id)
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
        .bind(channel.available_hours.as_ref().map(|s| serde_json::to_string(s).unwrap()))
        .bind(&channel.created_by)
        .bind(&channel.group_id)
        .bind(channel.created_at)
        .bind(channel.updated_at)
        .execute(&self.pool)
        .await?;

        let mut c = channel.clone();
        c.org_id = org_id.to_string();
        Ok(c)
    }

    async fn create_channel_with_models(&self, org_id: &str, channel: &Channel, models: Vec<ChannelModel>) -> Result<Channel, DbErr> {
        let mut tx = self.pool.begin().await?;
        let channel_id = channel.id.clone();

        sqlx::query(
            "INSERT INTO channels (id, org_id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, available_hours, created_by, group_id, disabled_until, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
        )
        .bind(&channel.id)
        .bind(org_id)
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
        .bind(channel.available_hours.as_ref().map(|s| serde_json::to_string(s).unwrap()))
        .bind(&channel.created_by)
        .bind(&channel.group_id)
        .bind(channel.disabled_until)
        .bind(channel.created_at)
        .bind(channel.updated_at)
        .execute(&mut *tx)
        .await?;

        for cm in &models {
            sqlx::query(
                "INSERT INTO channel_models (id, org_id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(&cm.id)
            .bind(org_id)
            .bind(&channel_id)
            .bind(&cm.model_id)
            .bind(&cm.upstream_model_name)
            .bind(cm.priority_override)
            .bind(&cm.pricing_policy_id)
            .bind(cm.markup_ratio)
            .bind(cm.enabled)
            .bind(cm.created_at)
            .bind(cm.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        let mut c = channel.clone();
        c.org_id = org_id.to_string();
        Ok(c)
    }

    async fn get_channel(&self, org_id: &str, id: &str) -> Result<Option<Channel>, DbErr> {
        let row: Option<PgChannelRow> = sqlx::query_as(
            "SELECT id, org_id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_by, group_id, disabled_until, created_at, updated_at, available_hours
             FROM channels WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Channel::from))
    }

    async fn list_channels(&self, org_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT id, org_id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_by, group_id, disabled_until, created_at, updated_at, available_hours
             FROM channels WHERE org_id = $1 ORDER BY priority ASC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn list_channels_by_provider(&self, org_id: &str, provider_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT id, org_id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_by, group_id, disabled_until, created_at, updated_at, available_hours
             FROM channels WHERE org_id = $1 AND provider_id = $2 ORDER BY priority ASC",
        )
        .bind(org_id)
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn list_enabled_channels_by_provider(&self, org_id: &str, provider_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT id, org_id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_by, group_id, disabled_until, created_at, updated_at, available_hours
             FROM channels WHERE org_id = $1 AND provider_id = $2 AND enabled = true ORDER BY priority ASC",
        )
        .bind(org_id)
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn update_channel(&self, org_id: &str, channel: &Channel) -> Result<Channel, DbErr> {
        sqlx::query(
            "UPDATE channels SET name = $1, api_key = $2, base_url = $3, priority = $4, pricing_policy_id = $5, markup_ratio = $6,
             enabled = $7, rpm_limit = $8, tpm_limit = $9, balance = $10, weight = $11, available_hours = $12, group_id = $13, disabled_until = $14, updated_at = $15 WHERE org_id = $16 AND id = $17",
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
        .bind(channel.available_hours.as_ref().map(|s| serde_json::to_string(s).unwrap()))
        .bind(&channel.group_id)
        .bind(channel.disabled_until)
        .bind(channel.updated_at)
        .bind(org_id)
        .bind(&channel.id)
        .execute(&self.pool)
        .await?;

        let mut c = channel.clone();
        c.org_id = org_id.to_string();
        Ok(c)
    }

    async fn delete_channel(&self, org_id: &str, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM channels WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn disable_channel_until(&self, org_id: &str, id: &str, until: chrono::DateTime<chrono::Utc>) -> Result<(), DbErr> {
        sqlx::query("UPDATE channels SET disabled_until = $1, updated_at = NOW() WHERE org_id = $2 AND id = $3")
            .bind(until)
            .bind(org_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Models (catalog: visibility filter + anti-shadowing) ----

    async fn create_model(&self, viewer_org_id: &str, model: &Model) -> Result<Model, DbErr> {
        // Anti-shadowing: an org-private model may not reuse a platform-level name.
        if let Some(_org) = &model.owner_org_id {
            let collision: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM models WHERE name = $1 AND owner_org_id IS NULL",
            )
            .bind(&model.name)
            .fetch_optional(&self.pool)
            .await?;
            if collision.is_some() {
                return Err(Box::new(CatalogNameReserved(model.name.clone())));
            }
        }

        sqlx::query(
            "INSERT INTO models (id, owner_org_id, name, model_type, pricing_policy_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&model.id)
        .bind(&model.owner_org_id)
        .bind(&model.name)
        .bind(&model.model_type)
        .bind(&model.pricing_policy_id)
        .bind(model.created_at)
        .execute(&self.pool)
        .await?;

        let _ = viewer_org_id;
        Ok(model.clone())
    }

    async fn get_model(&self, viewer_org_id: &str, name: &str) -> Result<Option<Model>, DbErr> {
        let row: Option<PgModelRow> = sqlx::query_as(
            "SELECT id, owner_org_id, name, model_type, pricing_policy_id, created_at
             FROM models
             WHERE name = $1 AND (owner_org_id IS NULL OR owner_org_id = $2)",
        )
        .bind(name)
        .bind(viewer_org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Model::from))
    }

    async fn get_model_by_id(&self, viewer_org_id: &str, id: &str) -> Result<Option<Model>, DbErr> {
        let row: Option<PgModelRow> = sqlx::query_as(
            "SELECT id, owner_org_id, name, model_type, pricing_policy_id, created_at
             FROM models
             WHERE id = $1 AND (owner_org_id IS NULL OR owner_org_id = $2)",
        )
        .bind(id)
        .bind(viewer_org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Model::from))
    }

    async fn get_model_by_provider(&self, viewer_org_id: &str, _provider_id: &str, _name: &str) -> Result<Option<Model>, DbErr> {
        // No longer supported - models are now N:N with providers
        let _ = viewer_org_id;
        Ok(None)
    }

    async fn list_models(&self, viewer_org_id: &str) -> Result<Vec<ModelWithProvider>, DbErr> {
        let rows = sqlx::query_as::<_, PgModelEnrichedRow>(
            r#"
            SELECT
                m.id,
                m.owner_org_id,
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
            WHERE m.owner_org_id IS NULL OR m.owner_org_id = $1
            GROUP BY m.id, m.owner_org_id, m.name, m.model_type, m.pricing_policy_id, m.created_at, pp.name
            ORDER BY m.name
            "#
        )
        .bind(viewer_org_id)
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
                    owner_org_id: r.owner_org_id,
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

    async fn list_models_by_provider(&self, viewer_org_id: &str, _provider_id: &str) -> Result<Vec<Model>, DbErr> {
        // No longer supported - models are now N:N with providers
        let _ = viewer_org_id;
        Ok(vec![])
    }

    async fn update_model(&self, viewer_org_id: &str, model: &Model) -> Result<Model, DbErr> {
        sqlx::query(
            "UPDATE models SET name = $1, pricing_policy_id = $2
             WHERE id = $3 AND (owner_org_id IS NULL OR owner_org_id = $4)",
        )
        .bind(&model.name)
        .bind(&model.pricing_policy_id)
        .bind(&model.id)
        .bind(viewer_org_id)
        .execute(&self.pool)
        .await?;

        Ok(model.clone())
    }

    async fn delete_model(&self, viewer_org_id: &str, name: &str) -> Result<(), DbErr> {
        sqlx::query(
            "DELETE FROM models
             WHERE name = $1 AND (owner_org_id IS NULL OR owner_org_id = $2)",
        )
        .bind(name)
        .bind(viewer_org_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- Key-Model Rate Limits (tenant: org_id scoping) ----

    async fn set_key_model_rate_limit(&self, org_id: &str, limit: &KeyModelRateLimit) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO key_model_rate_limits (org_id, key_id, model_id, rpm, tpm)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT(key_id, model_id) DO UPDATE SET rpm = $6, tpm = $7",
        )
        .bind(org_id)
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
        org_id: &str,
        key_id: &str,
        model_id: &str,
    ) -> Result<Option<KeyModelRateLimit>, DbErr> {
        let row: Option<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT key_id, model_id, rpm, tpm FROM key_model_rate_limits
             WHERE org_id = $1 AND key_id = $2 AND model_id = $3",
        )
        .bind(org_id)
        .bind(key_id)
        .bind(model_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| KeyModelRateLimit {
            org_id: org_id.to_string(),
            key_id: r.0,
            model_id: r.1,
            rpm: r.2,
            tpm: r.3,
        }))
    }

    async fn list_key_model_rate_limits(&self, org_id: &str, key_id: &str) -> Result<Vec<KeyModelRateLimit>, DbErr> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT key_id, model_id, rpm, tpm FROM key_model_rate_limits
             WHERE org_id = $1 AND key_id = $2",
        )
        .bind(org_id)
        .bind(key_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| KeyModelRateLimit {
                org_id: org_id.to_string(),
                key_id: r.0,
                model_id: r.1,
                rpm: r.2,
                tpm: r.3,
            })
            .collect())
    }

    async fn delete_key_model_rate_limit(&self, org_id: &str, key_id: &str, model_id: &str) -> Result<(), DbErr> {
        sqlx::query(
            "DELETE FROM key_model_rate_limits WHERE org_id = $1 AND key_id = $2 AND model_id = $3",
        )
        .bind(org_id)
        .bind(key_id)
        .bind(model_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- Usage (tenant: org_id scoping) ----

    async fn record_usage(&self, org_id: &str, usage: &UsageRecord) -> Result<(), DbErr> {
        let mut tx = self.pool.begin().await?;

        // Existing 17-column insert (unchanged).
        sqlx::query(
            "INSERT INTO usage_records (id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost, pricing_policy, weighted_tokens, user_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
        )
        .bind(&usage.id)
        .bind(org_id)
        .bind(&usage.request_id)
        .bind(&usage.key_id)
        .bind(&usage.model_name)
        .bind(&usage.provider_id)
        .bind(&usage.channel_id)
        .bind(protocol_str(&usage.protocol))
        .bind(usage.input_tokens)
        .bind(usage.output_tokens)
        .bind(usage.cache_read_tokens)
        .bind(usage.cache_creation_tokens)
        .bind(usage.cost)
        .bind(&usage.pricing_policy)
        .bind(usage.weighted_tokens)
        .bind(usage.user_id.clone())
        .bind(usage.created_at)
        .execute(&mut *tx)
        .await?;

        // Atomic counter upsert. Month bucket derived from created_at (UTC),
        // so backdated records bucket into the month they actually occurred.
        let month_bucket = format!("{}", usage.created_at.format("%Y-%m"));
        sqlx::query(
            "INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (key_id, month_bucket)
             DO UPDATE SET accrued = budget_counters.accrued + EXCLUDED.accrued,
                           updated_at = NOW()",
        )
        .bind(&usage.key_id)
        .bind(&month_bucket)
        .bind(usage.cost)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_month_to_date_spend(&self, key_id: &str) -> Result<i64, DbErr> {
        let month_bucket = format!("{}", chrono::Utc::now().format("%Y-%m"));
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT accrued FROM budget_counters WHERE key_id = $1 AND month_bucket = $2",
        )
        .bind(key_id)
        .bind(&month_bucket)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(v,)| v).unwrap_or(0))
    }

    async fn get_org_month_to_date_spend(&self, org_id: &str) -> Result<i64, DbErr> {
        let month_bucket = format!("{}", chrono::Utc::now().format("%Y-%m"));
        // INNER JOIN: every budget_counters row has a matching api_keys row
        // (FK), so a LEFT JOIN would just add NULLs we don't want. COALESCE
        // covers the "org has zero matching counter rows" case (fetch_optional
        // then unwrap_or(0) below).
        // SUM over NUMERIC returns NUMERIC, which sqlx won't decode to i64
        // directly — cast to BIGINT (the value space is bounded by accrued i64).
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(bc.accrued), 0) AS BIGINT)
             FROM budget_counters bc
             JOIN api_keys ak ON ak.id = bc.key_id
             WHERE ak.org_id = $1 AND bc.month_bucket = $2",
        )
        .bind(org_id)
        .bind(&month_bucket)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(v,)| v).unwrap_or(0))
    }

    async fn query_usage(&self, org_id: &str, _filter: &UsageFilter) -> Result<Vec<UsageRecord>, DbErr> {
        // Build query dynamically based on filter - for now, just fetch all (org-scoped)
        let rows: Vec<PgUsageRow> = sqlx::query_as(
            "SELECT id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost, pricing_policy, weighted_tokens, user_id, created_at
             FROM usage_records WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(UsageRecord::from).collect())
    }

    async fn query_usage_paginated(&self, org_id: &str, filter: &UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        // org_id is always $1; filter conditions take parameters $2.. and are
        // rebuilt in the same order for both the count and data queries below.
        let mut conditions = vec!["org_id = $1".to_string()];
        let mut param_idx = 2;
        let mut bind_user: Option<String> = None;
        let mut bind_key: Option<String> = None;
        let mut bind_model: Option<String> = None;
        let mut bind_since: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut bind_until: Option<chrono::DateTime<chrono::Utc>> = None;

        if let Some(ref user_id) = filter.user_id {
            conditions.push(format!("user_id = ${}", param_idx));
            bind_user = Some(user_id.clone());
            param_idx += 1;
        }
        if let Some(ref key_id) = filter.key_id {
            conditions.push(format!("key_id = ${}", param_idx));
            bind_key = Some(key_id.clone());
            param_idx += 1;
        }
        if let Some(ref model_name) = filter.model_name {
            conditions.push(format!("model_name = ${}", param_idx));
            bind_model = Some(model_name.clone());
            param_idx += 1;
        }
        if let Some(since) = filter.since {
            conditions.push(format!("created_at >= ${}", param_idx));
            bind_since = Some(since);
            param_idx += 1;
        }
        if let Some(until) = filter.until {
            conditions.push(format!("created_at <= ${}", param_idx));
            bind_until = Some(until);
        }

        let where_clause = format!(" WHERE {}", conditions.join(" AND "));

        let count_sql = format!("SELECT COUNT(*) FROM usage_records{}", where_clause);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        count_query = count_query.bind(org_id);
        if let Some(ref v) = bind_user { count_query = count_query.bind(v); }
        if let Some(ref v) = bind_key { count_query = count_query.bind(v); }
        if let Some(ref v) = bind_model { count_query = count_query.bind(v); }
        if let Some(ref v) = bind_since { count_query = count_query.bind(*v); }
        if let Some(ref v) = bind_until { count_query = count_query.bind(*v); }
        let total = count_query.fetch_one(&self.pool).await?.0;

        let offset = (page - 1) * page_size;
        let data_sql = format!(
            "SELECT id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost, pricing_policy, weighted_tokens, user_id, created_at \
             FROM usage_records{} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            param_idx,
            param_idx + 1
        );
        let mut data_query = sqlx::query_as::<_, PgUsageRow>(&data_sql);
        data_query = data_query.bind(org_id);
        if let Some(v) = bind_user { data_query = data_query.bind(v); }
        if let Some(v) = bind_key { data_query = data_query.bind(v); }
        if let Some(v) = bind_model { data_query = data_query.bind(v); }
        if let Some(v) = bind_since { data_query = data_query.bind(v); }
        if let Some(v) = bind_until { data_query = data_query.bind(v); }
        data_query = data_query.bind(page_size).bind(offset);
        let rows = data_query.fetch_all(&self.pool).await?;

        Ok(PaginatedResponse {
            items: rows.into_iter().map(UsageRecord::from).collect(),
            total,
            page,
            page_size,
        })
    }

    async fn query_usage_summary(&self, org_id: &str, filter: &UsageFilter) -> Result<Vec<UsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conditions = vec!["org_id = $1".to_string()];
        let mut param_idx = 2;
        let mut bind_user: Option<String> = None;
        let mut bind_key: Option<String> = None;
        let mut bind_model: Option<String> = None;
        let mut bind_since: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut bind_until: Option<chrono::DateTime<chrono::Utc>> = None;

        if let Some(ref user_id) = filter.user_id {
            conditions.push(format!("user_id = ${}", param_idx));
            bind_user = Some(user_id.clone());
            param_idx += 1;
        }
        if let Some(ref key_id) = filter.key_id {
            conditions.push(format!("key_id = ${}", param_idx));
            bind_key = Some(key_id.clone());
            param_idx += 1;
        }
        if let Some(ref model_name) = filter.model_name {
            conditions.push(format!("model_name = ${}", param_idx));
            bind_model = Some(model_name.clone());
            param_idx += 1;
        }
        if let Some(since) = filter.since {
            conditions.push(format!("created_at >= ${}", param_idx));
            bind_since = Some(since);
            param_idx += 1;
        }
        if let Some(until) = filter.until {
            conditions.push(format!("created_at <= ${}", param_idx));
            bind_until = Some(until);
        }

        let where_clause = format!(" WHERE {}", conditions.join(" AND "));

        let sql = format!(
            "SELECT \
               model_name, \
               COALESCE(SUM(input_tokens), 0)::BIGINT AS total_input_tokens, \
               COALESCE(SUM(cache_read_tokens), 0)::BIGINT AS total_cache_read_tokens, \
               COALESCE(SUM(cache_creation_tokens), 0)::BIGINT AS total_cache_creation_tokens, \
               COALESCE(SUM(output_tokens), 0)::BIGINT AS total_output_tokens, \
               COALESCE(SUM(cost), 0)::BIGINT AS total_cost, \
               COUNT(*) AS request_count \
             FROM usage_records{} \
             GROUP BY model_name \
             ORDER BY total_cost DESC",
            where_clause
        );

        let mut query = sqlx::query_as::<_, PgUsageSummaryRow>(&sql);
        query = query.bind(org_id);
        if let Some(v) = bind_user { query = query.bind(v); }
        if let Some(v) = bind_key { query = query.bind(v); }
        if let Some(v) = bind_model { query = query.bind(v); }
        if let Some(v) = bind_since { query = query.bind(v); }
        if let Some(v) = bind_until { query = query.bind(v); }

        let rows: Vec<PgUsageSummaryRow> = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(UsageSummaryRecord::from).collect())
    }

    async fn query_channel_usage_summary(&self, org_id: &str, filter: &UsageFilter) -> Result<Vec<ChannelUsageSummaryRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conditions = vec!["u.org_id = $1".to_string()];
        let mut param_idx = 2;
        let mut bind_vals: Vec<String> = Vec::new();
        let mut bind_since: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut bind_until: Option<chrono::DateTime<chrono::Utc>> = None;

        if let Some(ref user_id) = filter.user_id {
            conditions.push(format!("u.user_id = ${}", param_idx));
            bind_vals.push(user_id.clone());
            param_idx += 1;
        }
        if let Some(ref key_id) = filter.key_id {
            conditions.push(format!("u.key_id = ${}", param_idx));
            bind_vals.push(key_id.clone());
            param_idx += 1;
        }
        if let Some(ref model_name) = filter.model_name {
            conditions.push(format!("u.model_name = ${}", param_idx));
            bind_vals.push(model_name.clone());
            param_idx += 1;
        }
        if let Some(since) = filter.since {
            conditions.push(format!("u.created_at >= ${}", param_idx));
            bind_since = Some(since);
            param_idx += 1;
        }
        if let Some(until) = filter.until {
            conditions.push(format!("u.created_at <= ${}", param_idx));
            bind_until = Some(until);
        }

        let where_clause = format!(" WHERE {}", conditions.join(" AND "));

        let sql = format!(
            "SELECT \
               u.channel_id, \
               c.name AS channel_name, \
               COUNT(*) AS total_requests, \
               COALESCE(SUM(u.cost), 0)::BIGINT AS total_cost, \
               COALESCE(SUM(u.input_tokens), 0)::BIGINT AS total_input_tokens, \
               COALESCE(SUM(u.output_tokens), 0)::BIGINT AS total_output_tokens \
             FROM usage_records u \
             LEFT JOIN channels c ON u.channel_id = c.id \
             {} \
             GROUP BY u.channel_id, c.name \
             ORDER BY total_requests DESC",
            where_clause
        );

        let mut query = sqlx::query_as::<_, PgChannelUsageSummaryRow>(&sql);
        query = query.bind(org_id);
        for v in bind_vals {
            query = query.bind(v);
        }
        if let Some(v) = bind_since { query = query.bind(v); }
        if let Some(v) = bind_until { query = query.bind(v); }

        let rows: Vec<PgChannelUsageSummaryRow> = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(ChannelUsageSummaryRecord::from).collect())
    }

    async fn get_usage_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgUsageRow> = sqlx::query_as(
            "SELECT id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost, pricing_policy, weighted_tokens, user_id, created_at
             FROM usage_records WHERE org_id = $1 AND request_id = $2",
        )
        .bind(org_id)
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(UsageRecord::from))
    }

    async fn query_daily_usage(&self, org_id: &str, filter: &UsageFilter) -> Result<Vec<crate::types::DailyUsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
        // org_id is always $1; subsequent filter params take $2..
        let mut where_clauses = vec![format!("org_id = $1")];
        let mut param_idx = 2u32;

        if filter.user_id.is_some() { where_clauses.push(format!("user_id = ${}", param_idx)); param_idx += 1; }
        if filter.key_id.is_some() { where_clauses.push(format!("key_id = ${}", param_idx)); param_idx += 1; }
        if filter.model_name.is_some() { where_clauses.push(format!("model_name = ${}", param_idx)); param_idx += 1; }
        if filter.since.is_some() { where_clauses.push(format!("created_at >= ${}", param_idx)); param_idx += 1; }
        if filter.until.is_some() { where_clauses.push(format!("created_at < ${}", param_idx)); }

        let where_sql = format!("WHERE {}", where_clauses.join(" AND "));

        let tz = filter.tz.as_deref().unwrap_or("Etc/UTC");
        let sql = format!(
            "SELECT TO_CHAR((created_at AT TIME ZONE '{}')::date, 'YYYY-MM-DD') as date, \
             COALESCE(SUM(input_tokens), 0)::bigint as total_input_tokens, \
             COALESCE(SUM(output_tokens), 0)::bigint as total_output_tokens, \
             COALESCE(SUM(cache_read_tokens), 0)::bigint as total_cache_read_tokens, \
             COALESCE(SUM(cache_creation_tokens), 0)::bigint as total_cache_creation_tokens, \
             COALESCE(SUM(weighted_tokens), 0)::bigint as total_weighted_tokens, \
             COALESCE(SUM(cost), 0)::bigint as total_cost, \
             COUNT(*)::bigint as request_count \
             FROM usage_records {} \
             GROUP BY (created_at AT TIME ZONE '{}')::date \
             ORDER BY date",
            tz, where_sql, tz
        );

        let mut query = sqlx::query_as::<_, (String, i64, i64, i64, i64, i64, i64, i64)>(&sql);
        query = query.bind(org_id);

        if let Some(ref v) = filter.user_id { query = query.bind(v); }
        if let Some(ref v) = filter.key_id { query = query.bind(v); }
        if let Some(ref v) = filter.model_name { query = query.bind(v); }
        if let Some(v) = filter.since { query = query.bind(v); }
        if let Some(v) = filter.until { query = query.bind(v); }

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(date, inp, out, cr, cc, wt, cost, cnt)| {
            crate::types::DailyUsageRecord {
                date, total_input_tokens: inp, total_output_tokens: out,
                total_cache_read_tokens: cr, total_cache_creation_tokens: cc,
                total_weighted_tokens: wt, total_cost: cost, request_count: cnt,
            }
        }).collect())
    }

    // ---- Audit (tenant: org_id scoping) ----

    async fn insert_log(&self, org_id: &str, log: &AuditLog) -> Result<(), DbErr> {
        let routes_json = match log.routes.as_ref() {
            Some(r) => serde_json::to_value(r).unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        };
        sqlx::query(
            "INSERT INTO audit_logs (id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason,
             request_path, upstream_url, request_headers, response_headers, user_id, actor_is_platform_admin, routes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)",
        )
        .bind(&log.id)
        .bind(org_id)
        .bind(&log.request_id)
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
        .bind(log.user_id.clone())
        .bind(log.actor_is_platform_admin)
        .bind(routes_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn query_logs(&self, org_id: &str, filter: &LogFilter) -> Result<Vec<AuditLog>, DbErr> {
        // org_id is always $1; subsequent filter params take $2.. and are
        // appended in the same order they are bound below. LIMIT/OFFSET (when
        // present) are bound as trailing params after the filter values.
        let mut conditions: Vec<String> = vec!["org_id = $1".to_string()];
        let mut bind_vals: Vec<String> = Vec::new();

        if let Some(ref key_id) = filter.key_id {
            conditions.push(format!("key_id = ${}", bind_vals.len() + 2));
            bind_vals.push(key_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            conditions.push(format!("model_name = ${}", bind_vals.len() + 2));
            bind_vals.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            conditions.push(format!("created_at >= ${}", bind_vals.len() + 2));
            bind_vals.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            conditions.push(format!("created_at <= ${}", bind_vals.len() + 2));
            bind_vals.push(until.to_rfc3339());
        }

        let mut sql = String::from(
            "SELECT id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason,
             request_path, upstream_url, request_headers, response_headers, user_id, actor_is_platform_admin, routes
             FROM audit_logs",
        );
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
        sql.push_str(" ORDER BY created_at DESC");

        let limit_offset_start = bind_vals.len() + 2; // next slot after filters
        let limit_placeholder: Option<String> = filter.limit.map(|_| format!("${}", limit_offset_start));
        let offset_placeholder: Option<String> = filter.offset.map(|_| {
            let idx = limit_offset_start + if filter.limit.is_some() { 1 } else { 0 };
            format!("${}", idx)
        });
        if let Some(ref p) = limit_placeholder {
            sql.push_str(&format!(" LIMIT {}", p));
        }
        if let Some(ref p) = offset_placeholder {
            sql.push_str(&format!(" OFFSET {}", p));
        }

        let mut q = sqlx::query_as::<_, PgAuditRow>(&sql);
        q = q.bind(org_id);
        for val in &bind_vals {
            q = q.bind(val);
        }
        if let Some(limit) = filter.limit { q = q.bind(limit); }
        if let Some(offset) = filter.offset { q = q.bind(offset); }
        let rows: Vec<PgAuditRow> = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(AuditLog::from).collect())
    }

    async fn query_logs_paginated(&self, org_id: &str, filter: &LogFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLogSummary>, Box<dyn std::error::Error + Send + Sync>> {
        // org_id is always $1; subsequent filter params take $2.. and are
        // rebound in the same order for both count and data queries below.
        let mut conditions = vec!["a.org_id = $1".to_string()];
        let mut bind_vals: Vec<String> = Vec::new();

        if let Some(ref request_id) = filter.request_id {
            conditions.push(format!("a.request_id = ${}", bind_vals.len() + 2));
            bind_vals.push(request_id.clone());
        }

        if let Some(ref user_id) = filter.user_id {
            conditions.push(format!("a.user_id = ${}", bind_vals.len() + 2));
            bind_vals.push(user_id.clone());
        }
        if let Some(ref key_id) = filter.key_id {
            conditions.push(format!("a.key_id = ${}", bind_vals.len() + 2));
            bind_vals.push(key_id.clone());
        }
        if let Some(ref channel_id) = filter.channel_id {
            conditions.push(format!("a.channel_id = ${}", bind_vals.len() + 2));
            bind_vals.push(channel_id.clone());
        }
        if let Some(ref model_name) = filter.model_name {
            conditions.push(format!("a.model_name = ${}", bind_vals.len() + 2));
            bind_vals.push(model_name.clone());
        }
        if let Some(since) = filter.since {
            conditions.push(format!("a.created_at >= ${}", bind_vals.len() + 2));
            bind_vals.push(since.to_rfc3339());
        }
        if let Some(until) = filter.until {
            conditions.push(format!("a.created_at <= ${}", bind_vals.len() + 2));
            bind_vals.push(until.to_rfc3339());
        }

        let where_clause = format!(" WHERE {}", conditions.join(" AND "));

        let count_sql = format!("SELECT COUNT(*) FROM audit_logs a{}", where_clause);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        count_query = count_query.bind(org_id);
        for val in &bind_vals {
            count_query = count_query.bind(val);
        }
        let total = count_query.fetch_one(&self.pool).await?.0;

        let offset = (page - 1) * page_size;
        let data_sql = format!(
            "SELECT a.id, a.request_id, a.key_id, a.model_name, a.provider_id, a.channel_id, c.name AS channel_name, a.protocol, a.stream,
             a.status_code, a.latency_ms, a.input_tokens, a.output_tokens, a.created_at, a.original_model, a.upstream_model, a.model_override_reason,
             a.request_path, a.upstream_url, a.request_headers, a.response_headers, a.user_id, a.routes
             FROM audit_logs a LEFT JOIN channels c ON a.channel_id = c.id{} ORDER BY a.created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            bind_vals.len() + 2,
            bind_vals.len() + 3
        );
        let mut data_query = sqlx::query_as::<_, PgAuditSummaryRow>(&data_sql);
        data_query = data_query.bind(org_id);
        for val in bind_vals {
            data_query = data_query.bind(val);
        }
        data_query = data_query.bind(page_size).bind(offset);
        let rows = data_query.fetch_all(&self.pool).await?;

        Ok(PaginatedResponse {
            items: rows.into_iter().map(AuditLogSummary::from).collect(),
            total,
            page,
            page_size,
        })
    }

    async fn get_log(&self, org_id: &str, id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgAuditRow> = sqlx::query_as(
            "SELECT a.id, a.org_id, a.request_id, a.key_id, a.model_name, a.provider_id, a.channel_id, c.name AS channel_name, a.protocol, a.stream, a.request_body, a.response_body,
             a.status_code, a.latency_ms, a.input_tokens, a.output_tokens, a.created_at, a.original_model, a.upstream_model, a.model_override_reason,
             a.request_path, a.upstream_url, a.request_headers, a.response_headers, a.user_id, a.actor_is_platform_admin, a.routes
             FROM audit_logs a LEFT JOIN channels c ON a.channel_id = c.id WHERE a.org_id = $1 AND a.id = $2",
        )
        .bind(org_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(AuditLog::from))
    }

    async fn get_audit_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgAuditRow> = sqlx::query_as(
            "SELECT a.id, a.org_id, a.request_id, a.key_id, a.model_name, a.provider_id, a.channel_id, c.name AS channel_name, a.protocol, a.stream, a.request_body, a.response_body,
             a.status_code, a.latency_ms, a.input_tokens, a.output_tokens, a.created_at, a.original_model, a.upstream_model, a.model_override_reason,
             a.request_path, a.upstream_url, a.request_headers, a.response_headers, a.user_id, a.actor_is_platform_admin, a.routes
             FROM audit_logs a LEFT JOIN channels c ON a.channel_id = c.id WHERE a.org_id = $1 AND a.request_id = $2",
        )
        .bind(org_id)
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(AuditLog::from))
    }

    async fn increment_rate_limit_counter(
        &self,
        key_id: &str,
        model_name: &str,
        window: &str,
    ) -> Result<i64, DbErr> {
        sqlx::query(
            "INSERT INTO rate_limit_counters (key_id, model_name, \"window\", count) VALUES ($1, $2, $3, 1)
             ON CONFLICT(key_id, model_name, \"window\") DO UPDATE SET count = count + 1",
        )
        .bind(key_id)
        .bind(model_name)
        .bind(window)
        .execute(&self.pool)
        .await?;

        let count: (i64,) = sqlx::query_as(
            "SELECT count FROM rate_limit_counters WHERE key_id = $1 AND model_name = $2 AND \"window\" = $3",
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
            "SELECT count FROM rate_limit_counters WHERE key_id = $1 AND model_name = $2 AND \"window\" = $3",
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
            "INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, refresh_token,
                                created_at, updated_at,
                                email, email_verified_at, requires_email_verification, password_changed_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.password)
        .bind(user.platform_role.as_ref().map(|r| r.as_str()))
        .bind(&user.current_org_id)
        .bind(user.enabled)
        .bind(&user.refresh_token)
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(&user.email)
        .bind(user.email_verified_at)
        .bind(user.requires_email_verification)
        .bind(user.password_changed_at)
        .execute(&self.pool)
        .await?;
        Ok(user.clone())
    }

    async fn get_user(&self, id: &str) -> Result<Option<User>, DbErr> {
        let row: Option<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, platform_role, current_org_id, enabled, refresh_token,
                    created_at, updated_at,
                    email, email_verified_at, requires_email_verification, password_changed_at
             FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(User::from))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, DbErr> {
        let row: Option<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, platform_role, current_org_id, enabled, refresh_token,
                    created_at, updated_at,
                    email, email_verified_at, requires_email_verification, password_changed_at
             FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(User::from))
    }

    async fn list_users(&self, org_id: &str) -> Result<Vec<User>, DbErr> {
        let rows: Vec<PgUserRow> = sqlx::query_as(
            "SELECT u.id, u.username, u.password, u.platform_role, u.current_org_id, u.enabled, u.refresh_token,
                    u.created_at, u.updated_at,
                    u.email, u.email_verified_at, u.requires_email_verification, u.password_changed_at
             FROM users u
             JOIN members m ON m.user_id = u.id
             WHERE m.org_id = $1
             ORDER BY u.username",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(User::from).collect())
    }

    async fn list_users_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<UserWithBalance>, Box<dyn std::error::Error + Send + Sync>> {
        // TODO(Task 9): UserWithBalance still carries legacy role/group_id columns.
        // Once the management handlers stop reading them, the struct + this query
        // should drop them. Until then we synthesize a role/group_id from the
        // membership row so the existing UI keeps rendering.
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users u JOIN members m ON m.user_id = u.id WHERE m.org_id = $1",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<PgUserWithBalanceRow> = sqlx::query_as(
            "SELECT u.id, u.username, COALESCE(m.role, 'member') AS role, u.enabled, m.group_id, g.name AS group_name, \
                    COALESCE(a.balance, 0) AS balance, COALESCE(a.threshold, 0) AS threshold, u.created_at, u.updated_at \
             FROM users u \
             JOIN members m ON m.user_id = u.id AND m.org_id = $1 \
             LEFT JOIN accounts a ON a.user_id = u.id AND a.org_id = $1 \
             LEFT JOIN groups g ON g.id = m.group_id \
             ORDER BY u.created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(org_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(UserWithBalance::from).collect(),
            total: total.0,
            page,
            page_size,
        })
    }

    async fn update_user(&self, user: &User) -> Result<User, DbErr> {
        sqlx::query(
            "UPDATE users SET username = $1, password = $2, platform_role = $3, current_org_id = $4, enabled = $5, refresh_token = $6, password_changed_at = $7, updated_at = $8 WHERE id = $9",
        )
        .bind(&user.username)
        .bind(&user.password)
        .bind(user.platform_role.as_ref().map(|r| r.as_str()))
        .bind(&user.current_org_id)
        .bind(user.enabled)
        .bind(&user.refresh_token)
        .bind(&user.password_changed_at)
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

    // ---- Channel Models (tenant: org_id scoping) ----

    async fn create_channel_model(&self, org_id: &str, cm: &ChannelModel) -> Result<ChannelModel, DbErr> {
        sqlx::query(
            "INSERT INTO channel_models (id, org_id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&cm.id)
        .bind(org_id)
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

        let mut c = cm.clone();
        c.org_id = org_id.to_string();
        Ok(c)
    }

    async fn get_channel_model(&self, org_id: &str, id: &str) -> Result<Option<ChannelModel>, DbErr> {
        let row: Option<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, org_id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ChannelModel::from))
    }

    async fn list_channel_models(&self, org_id: &str) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, org_id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models WHERE org_id = $1",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn list_channel_models_by_channel(&self, org_id: &str, channel_id: &str) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, org_id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models WHERE org_id = $1 AND channel_id = $2",
        )
        .bind(org_id)
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn get_channel_models_for_model(&self, org_id: &str, model_id: &str) -> Result<Vec<ChannelModel>, DbErr> {
        let rows: Vec<PgChannelModelRow> = sqlx::query_as(
            "SELECT id, org_id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, markup_ratio, enabled, created_at, updated_at
             FROM channel_models WHERE org_id = $1 AND model_id = $2",
        )
        .bind(org_id)
        .bind(model_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ChannelModel::from).collect())
    }

    async fn get_channels_for_model(&self, org_id: &str, model_id: &str) -> Result<Vec<Channel>, DbErr> {
        let rows: Vec<PgChannelRow> = sqlx::query_as(
            "SELECT c.id, c.org_id, c.provider_id, c.name, c.api_key, c.base_url, c.priority, c.pricing_policy_id, c.markup_ratio, c.enabled, c.rpm_limit, c.tpm_limit, c.balance, c.weight, c.created_by, c.group_id, c.disabled_until, c.created_at, c.updated_at, c.available_hours
             FROM channels c
             JOIN channel_models cm ON c.id = cm.channel_id
             WHERE c.org_id = $1 AND cm.model_id = $2 AND c.enabled = true",
        )
        .bind(org_id)
        .bind(model_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Channel::from).collect())
    }

    async fn update_channel_model(&self, org_id: &str, cm: &ChannelModel) -> Result<ChannelModel, DbErr> {
        sqlx::query(
            "UPDATE channel_models SET channel_id = $1, model_id = $2, upstream_model_name = $3,
             priority_override = $4, pricing_policy_id = $5, markup_ratio = $6, enabled = $7, updated_at = $8
             WHERE org_id = $9 AND id = $10",
        )
        .bind(&cm.channel_id)
        .bind(&cm.model_id)
        .bind(&cm.upstream_model_name)
        .bind(cm.priority_override)
        .bind(&cm.pricing_policy_id)
        .bind(cm.markup_ratio)
        .bind(cm.enabled)
        .bind(cm.updated_at)
        .bind(org_id)
        .bind(&cm.id)
        .execute(&self.pool)
        .await?;

        let mut c = cm.clone();
        c.org_id = org_id.to_string();
        Ok(c)
    }

    async fn delete_channel_model(&self, org_id: &str, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM channel_models WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Provider Models (catalog: visibility filter on provider + model) ----

    async fn upsert_provider_models(&self, viewer_org_id: &str, provider_id: &str, models: Vec<ProviderModel>) -> Result<(), DbErr> {
        for pm in models {
            sqlx::query(
                "INSERT INTO provider_models (provider_id, model_id, owner_org_id, upstream_name, pricing_policy_id, created_at)
                 VALUES ($1, $2, $3, $4, $5, NOW())
                 ON CONFLICT (provider_id, model_id) DO UPDATE SET upstream_name = EXCLUDED.upstream_name, pricing_policy_id = EXCLUDED.pricing_policy_id",
            )
            .bind(provider_id)
            .bind(&pm.model_id)
            .bind(&pm.owner_org_id)
            .bind(&pm.upstream_name)
            .bind(&pm.pricing_policy_id)
            .execute(&self.pool)
            .await?;
        }
        let _ = viewer_org_id;
        Ok(())
    }

    async fn list_provider_models(&self, viewer_org_id: &str, provider_id: &str) -> Result<Vec<ProviderModelInfo>, DbErr> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(
            "SELECT pm.model_id, m.name, pm.upstream_name, pm.pricing_policy_id
             FROM provider_models pm
             JOIN models m ON m.id = pm.model_id
             WHERE pm.provider_id = $1
               AND (pm.owner_org_id IS NULL OR pm.owner_org_id = $2)
               AND (m.owner_org_id IS NULL OR m.owner_org_id = $2)
             ORDER BY m.name",
        )
        .bind(provider_id)
        .bind(viewer_org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(model_id, model_name, upstream_name, pricing_policy_id)| ProviderModelInfo {
            model_id,
            model_name,
            upstream_name,
            pricing_policy_id,
        }).collect())
    }

    async fn set_provider_models(&self, viewer_org_id: &str, provider_id: &str, models: Vec<ProviderModel>) -> Result<(), DbErr> {
        // Scope the DELETE to this org's rows only. The handler always supplies
        // `owner_org_id = Some(ctx.org_id)` on input, so platform-level rows
        // (owner_org_id IS NULL) are never created here and must never be
        // deleted here either — otherwise an org admin could wipe platform-wide
        // provider↔model mappings by calling PUT /admin/providers/{id}/models.
        sqlx::query(
            "DELETE FROM provider_models WHERE provider_id = $1 AND owner_org_id = $2",
        )
        .bind(provider_id)
        .bind(viewer_org_id)
        .execute(&self.pool)
        .await?;
        for pm in models {
            let owner = pm.owner_org_id.as_deref();
            if !matches!(owner, None) && owner != Some(viewer_org_id) {
                return Err(Box::new(CatalogNameReserved(
                    "set_provider_models: cannot assign rows to another org".into(),
                )));
            }
            sqlx::query(
                "INSERT INTO provider_models (provider_id, model_id, owner_org_id, upstream_name, pricing_policy_id, created_at)
                 VALUES ($1, $2, $3, $4, $5, now())",
            )
            .bind(provider_id)
            .bind(&pm.model_id)
            .bind(&pm.owner_org_id)
            .bind(&pm.upstream_name)
            .bind(&pm.pricing_policy_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
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

    // ─── Atomic Balance Operations ───────────────────────────────────────────────

    async fn deduct_balance(&self, org_id: &str, req: &DeductBalance) -> Result<DeductBalanceResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        // Lock the account row and get current balance (org-scoped)
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT balance FROM accounts WHERE org_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(org_id)
        .bind(&req.account_id)
        .fetch_optional(&mut *tx)
        .await?;

        let current_balance = match row {
            Some((b,)) => b,
            None => return Ok(DeductBalanceResult::AccountNotFound),
        };

        if current_balance < req.amount {
            return Ok(DeductBalanceResult::InsufficientBalance {
                current_balance,
                requested: req.amount,
            });
        }

        let new_balance = current_balance - req.amount;
        let now = chrono::Utc::now();

        // Update account balance
        sqlx::query("UPDATE accounts SET balance = $1, updated_at = $2 WHERE id = $3")
            .bind(new_balance)
            .bind(now)
            .bind(&req.account_id)
            .execute(&mut *tx)
            .await?;

        // Create transaction record
        let tx_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO transactions (id, org_id, account_id, type, amount, balance_after, description, reference_id, request_id, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(&tx_id)
        .bind(org_id)
        .bind(&req.account_id)
        .bind(req.transaction_type.as_str())
        .bind(req.amount)
        .bind(new_balance)
        .bind(&req.description)
        .bind(&req.reference_id)
        .bind(&req.request_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(DeductBalanceResult::Success(Transaction {
            id: tx_id,
            org_id: org_id.to_string(),
            account_id: req.account_id.clone(),
            transaction_type: req.transaction_type,
            amount: req.amount,
            balance_after: new_balance,
            description: req.description.clone(),
            reference_id: req.reference_id.clone(),
            request_id: req.request_id.clone(),
            created_at: now,
        }))
    }

    async fn add_balance(&self, org_id: &str, req: &AddBalance) -> Result<AddBalanceResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        // Lock the account row and get current balance (org-scoped)
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT balance FROM accounts WHERE org_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(org_id)
        .bind(&req.account_id)
        .fetch_optional(&mut *tx)
        .await?;

        let current_balance = match row {
            Some((b,)) => b,
            None => return Ok(AddBalanceResult::AccountNotFound),
        };

        let new_balance = current_balance + req.amount;
        let now = chrono::Utc::now();

        // Update account balance
        sqlx::query("UPDATE accounts SET balance = $1, updated_at = $2 WHERE id = $3")
            .bind(new_balance)
            .bind(now)
            .bind(&req.account_id)
            .execute(&mut *tx)
            .await?;

        // Create transaction record
        let tx_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO transactions (id, org_id, account_id, type, amount, balance_after, description, reference_id, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(&tx_id)
        .bind(org_id)
        .bind(&req.account_id)
        .bind(req.transaction_type.as_str())
        .bind(req.amount)
        .bind(new_balance)
        .bind(&req.description)
        .bind(&req.reference_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(AddBalanceResult::Success(Transaction {
            id: tx_id,
            org_id: org_id.to_string(),
            account_id: req.account_id.clone(),
            transaction_type: req.transaction_type,
            amount: req.amount,
            balance_after: new_balance,
            description: req.description.clone(),
            reference_id: req.reference_id.clone(),
            request_id: None,
            created_at: now,
        }))
    }

    // ---- Model Fallbacks ----

    async fn create_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, DbErr> {
        let config_json = serde_json::to_string(&config.config).unwrap_or_default();
        sqlx::query(
            "INSERT INTO model_fallbacks (id, name, config, created_by, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&config.id)
        .bind(&config.name)
        .bind(&config_json)
        .bind(&config.created_by)
        .bind(config.created_at)
        .execute(&self.pool)
        .await?;

        Ok(config.clone())
    }

    async fn get_model_fallback(&self, id: &str) -> Result<Option<ModelFallbackConfig>, DbErr> {
        let row: Option<PgModelFallbackRow> = sqlx::query_as(
            "SELECT id, name, config, created_by, created_at FROM model_fallbacks WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ModelFallbackConfig::from))
    }

    async fn list_model_fallbacks(&self) -> Result<Vec<ModelFallbackConfig>, DbErr> {
        let rows: Vec<PgModelFallbackRow> = sqlx::query_as(
            "SELECT id, name, config, created_by, created_at FROM model_fallbacks",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ModelFallbackConfig::from).collect())
    }

    async fn update_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, DbErr> {
        let config_json = serde_json::to_string(&config.config).unwrap_or_default();
        sqlx::query(
            "UPDATE model_fallbacks SET name = $1, config = $2 WHERE id = $3",
        )
        .bind(&config.name)
        .bind(&config_json)
        .bind(&config.id)
        .execute(&self.pool)
        .await?;

        Ok(config.clone())
    }

    async fn delete_model_fallback(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM model_fallbacks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Groups ----

    async fn list_groups(&self, org_id: &str) -> Result<Vec<Group>, DbErr> {
        let rows: Vec<PgGroupRow> = sqlx::query_as(
            "SELECT id, org_id, name, description, created_at, updated_at FROM groups WHERE org_id = $1 ORDER BY name",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Group::from).collect())
    }

    async fn list_groups_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Group>, Box<dyn std::error::Error + Send + Sync>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let rows: Vec<PgGroupRow> = sqlx::query_as(
            "SELECT id, org_id, name, description, created_at, updated_at FROM groups WHERE org_id = $1 ORDER BY name LIMIT $2 OFFSET $3",
        )
        .bind(org_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(Group::from).collect(),
            total: total.0,
            page,
            page_size,
        })
    }

    async fn get_group(&self, org_id: &str, id: &str) -> Result<Option<Group>, DbErr> {
        let row: Option<PgGroupRow> = sqlx::query_as(
            "SELECT id, org_id, name, description, created_at, updated_at FROM groups WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Group::from))
    }

    async fn create_group(&self, org_id: &str, input: &CreateGroup) -> Result<Group, DbErr> {
        let id = uuid::Uuid::new_v4().to_string();
        let row: PgGroupRow = sqlx::query_as(
            "INSERT INTO groups (id, org_id, name, description)
             VALUES ($1, $2, $3, $4)
             RETURNING id, org_id, name, description, created_at, updated_at",
        )
        .bind(&id)
        .bind(org_id)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await?;
        Ok(Group::from(row))
    }

    // Tri-state description: None=keep, Some(None)=clear, Some(Some(v))=set
    // Uses a flag param to distinguish "don't touch" vs "set to NULL" (cleared).
    async fn update_group(&self, org_id: &str, id: &str, input: &UpdateGroup) -> Result<Group, DbErr> {
        let should_update_description = input.description.is_some();
        let new_description: Option<String> = match &input.description {
            Some(Some(v)) => Some(v.clone()),
            _ => None,
        };
        let row: Option<PgGroupRow> = sqlx::query_as(
            "UPDATE groups
             SET name = COALESCE($3, name),
                 description = CASE WHEN $4::boolean THEN $5 ELSE description END,
                 updated_at = NOW()
             WHERE org_id = $1 AND id = $2
             RETURNING id, org_id, name, description, created_at, updated_at",
        )
        .bind(org_id)
        .bind(id)
        .bind(&input.name)
        .bind(should_update_description)
        .bind(&new_description)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Group::from).ok_or_else(|| -> DbErr {
            Box::new(sqlx::Error::RowNotFound)
        })
    }

    async fn delete_group(&self, org_id: &str, id: &str) -> Result<DeleteGroupResult, DbErr> {
        // group_id moved from users → members in the migration; clear it on
        // memberships in this org, plus channels in this org.
        let cleared_users = sqlx::query(
            "UPDATE members SET group_id = NULL WHERE group_id = $1 AND org_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?
        .rows_affected() as i64;

        let cleared_channels = sqlx::query(
            "UPDATE channels SET group_id = NULL WHERE group_id = $1 AND org_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?
        .rows_affected() as i64;

        sqlx::query("DELETE FROM groups WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(DeleteGroupResult { cleared_users, cleared_channels })
    }

    async fn get_user_group_id(&self, user_id: &str, org_id: &str) -> Result<Option<String>, DbErr> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT group_id FROM members WHERE user_id = $1 AND org_id = $2",
        )
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|(g,)| g))
    }

    // ---- Seed Data ----

    async fn seed_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::seed;

        // Seed data is platform-level: viewer_org_id = "" because the rows we
        // create all have owner_org_id = NULL. The visibility filter
        // (owner_org_id IS NULL OR owner_org_id = $1) matches them regardless.
        let viewer_org_id = "";

        // Seed providers if none exist (idempotent)
        let existing_providers = self.list_providers(viewer_org_id).await?;
        if existing_providers.is_empty() {
            let seed_providers = seed::get_seed_providers();
            for provider in seed_providers {
                self.create_provider(viewer_org_id, &provider).await?;
            }
        }

        // Seed pricing policies independently — check pricing_policies table
        let existing_policies = self.list_pricing_policies(viewer_org_id).await?;
        if existing_policies.is_empty() {
            let seed_policies = seed::get_seed_pricing_policies();
            let mut policy_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            for (policy, model_name) in &seed_policies {
                let policy_id = policy.id.clone();
                self.create_pricing_policy(viewer_org_id, policy).await?;
                policy_map.insert(model_name.to_lowercase(), policy_id);
            }

            // Seed models — check models table independently
            let existing_models = self.list_models(viewer_org_id).await?;
            if existing_models.is_empty() {
                let seed_models = seed::get_seed_models(&[]);
                for mut model in seed_models {
                    if let Some(policy_id) = policy_map.get(&model.name.to_lowercase()) {
                        model.pricing_policy_id = Some(policy_id.clone());
                    }
                    self.create_model(viewer_org_id, &model).await?;
                }
            }
        }

        // Seed default platform settings for audit logs (idempotent - uses ON CONFLICT)
        self.set_platform_setting("audit_log_request", "true").await?;
        self.set_platform_setting("audit_log_response", "true").await?;

        // Seed provider_models if empty
        let providers = self.list_providers(viewer_org_id).await?;
        let models = self.list_models(viewer_org_id).await?;
        let provider_id_map = seed::build_provider_id_map(&providers);
        let model_id_map: Vec<(String, String)> = models.iter().map(|m| (m.model.name.clone(), m.model.id.clone())).collect();
        let seed_pm = seed::get_seed_provider_models(&provider_id_map, &model_id_map);
        if !seed_pm.is_empty() {
            for (provider_id, group) in &seed_pm.iter().fold(std::collections::HashMap::<String, Vec<_>>::new(), |mut acc, pm| {
                acc.entry(pm.provider_id.clone()).or_default().push(pm.clone());
                acc
            }) {
                let _ = self.upsert_provider_models(viewer_org_id, provider_id, group.clone()).await;
            }
        }

        Ok(())
    }

    // ---- Pricing Policies (catalog: visibility filter + anti-shadowing) ----

    async fn create_pricing_policy(&self, viewer_org_id: &str, policy: &PricingPolicy) -> Result<PricingPolicy, DbErr> {
        // Anti-shadowing: an org-private policy may not reuse a platform-level name.
        if let Some(_org) = &policy.owner_org_id {
            let collision: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM pricing_policies WHERE name = $1 AND owner_org_id IS NULL",
            )
            .bind(&policy.name)
            .fetch_optional(&self.pool)
            .await?;
            if collision.is_some() {
                return Err(Box::new(CatalogNameReserved(policy.name.clone())));
            }
        }

        sqlx::query(
            "INSERT INTO pricing_policies (id, owner_org_id, name, billing_type, config, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&policy.id)
        .bind(&policy.owner_org_id)
        .bind(&policy.name)
        .bind(&policy.billing_type)
        .bind(policy.config.to_string())
        .bind(policy.created_at)
        .bind(policy.updated_at)
        .execute(&self.pool)
        .await?;

        let _ = viewer_org_id;
        Ok(policy.clone())
    }

    async fn get_pricing_policy(&self, viewer_org_id: &str, id: &str) -> Result<Option<PricingPolicy>, DbErr> {
        let row: Option<PgPricingPolicyRow> = sqlx::query_as(
            "SELECT id, owner_org_id, name, billing_type, config, created_at, updated_at
             FROM pricing_policies
             WHERE id = $1 AND (owner_org_id IS NULL OR owner_org_id = $2)",
        )
        .bind(id)
        .bind(viewer_org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(PricingPolicy::from))
    }

    async fn list_pricing_policies(&self, viewer_org_id: &str) -> Result<Vec<PricingPolicy>, DbErr> {
        let rows: Vec<PgPricingPolicyRow> = sqlx::query_as(
            "SELECT id, owner_org_id, name, billing_type, config, created_at, updated_at
             FROM pricing_policies
             WHERE owner_org_id IS NULL OR owner_org_id = $1",
        )
        .bind(viewer_org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(PricingPolicy::from).collect())
    }

    async fn list_pricing_policies_with_counts(&self, viewer_org_id: &str) -> Result<Vec<PricingPolicyWithCounts>, DbErr> {
        let rows: Vec<PgPricingPolicyRow> = sqlx::query_as(
            "SELECT id, owner_org_id, name, billing_type, config, created_at, updated_at
             FROM pricing_policies
             WHERE owner_org_id IS NULL OR owner_org_id = $1",
        )
        .bind(viewer_org_id)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let policy = PricingPolicy::from(row);
            // Count models using this policy that are also visible to this viewer
            // (platform + own org-private).
            let model_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM models
                 WHERE pricing_policy_id = $1
                   AND (owner_org_id IS NULL OR owner_org_id = $2)",
            )
            .bind(&policy.id)
            .bind(viewer_org_id)
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

    async fn update_pricing_policy(&self, viewer_org_id: &str, policy: &PricingPolicy) -> Result<PricingPolicy, DbErr> {
        sqlx::query(
            "UPDATE pricing_policies SET name = $1, billing_type = $2, config = $3, updated_at = $4
             WHERE id = $5 AND (owner_org_id IS NULL OR owner_org_id = $6)",
        )
        .bind(&policy.name)
        .bind(&policy.billing_type)
        .bind(policy.config.to_string())
        .bind(policy.updated_at)
        .bind(&policy.id)
        .bind(viewer_org_id)
        .execute(&self.pool)
        .await?;

        Ok(policy.clone())
    }

    async fn delete_pricing_policy(&self, viewer_org_id: &str, id: &str) -> Result<(), DbErr> {
        sqlx::query(
            "DELETE FROM pricing_policies WHERE id = $1 AND (owner_org_id IS NULL OR owner_org_id = $2)",
        )
        .bind(id)
        .bind(viewer_org_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ─── Accounts (tenant: org_id scoping) ──────────────────────────────────────

    async fn create_account(&self, org_id: &str, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "INSERT INTO accounts (id, org_id, user_id, balance, threshold, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(&account.id)
        .bind(org_id)
        .bind(&account.user_id)
        .bind(account.balance)
        .bind(account.threshold)
        .bind(account.created_at)
        .bind(account.updated_at)
        .execute(self.pool())
        .await?;
        let mut a = account.clone();
        a.org_id = org_id.to_string();
        Ok(a)
    }

    async fn get_account(&self, org_id: &str, id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgAccountRow> = sqlx::query_as(
            "SELECT id, org_id, user_id, balance, threshold, created_at, updated_at FROM accounts WHERE org_id = $1 AND id = $2"
        )
        .bind(org_id)
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Account::from))
    }

    async fn get_account_by_user_id(&self, org_id: &str, user_id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgAccountRow> = sqlx::query_as(
            "SELECT id, org_id, user_id, balance, threshold, created_at, updated_at FROM accounts WHERE org_id = $1 AND user_id = $2"
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Account::from))
    }

    async fn update_account(&self, org_id: &str, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "UPDATE accounts SET balance = $1, threshold = $2, updated_at = $3 WHERE org_id = $4 AND id = $5"
        )
        .bind(account.balance)
        .bind(account.threshold)
        .bind(account.updated_at)
        .bind(org_id)
        .bind(&account.id)
        .execute(self.pool())
        .await?;
        let mut a = account.clone();
        a.org_id = org_id.to_string();
        Ok(a)
    }

    // ─── Transactions (tenant: org_id scoping) ──────────────────────────────────

    async fn create_transaction(&self, org_id: &str, transaction: &Transaction) -> Result<Transaction, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "INSERT INTO transactions (id, org_id, account_id, type, amount, balance_after, description, reference_id, request_id, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(&transaction.id)
        .bind(org_id)
        .bind(&transaction.account_id)
        .bind(transaction.transaction_type.as_str())
        .bind(transaction.amount)
        .bind(transaction.balance_after)
        .bind(&transaction.description)
        .bind(&transaction.reference_id)
        .bind(&transaction.request_id)
        .bind(transaction.created_at)
        .execute(self.pool())
        .await?;
        let mut t = transaction.clone();
        t.org_id = org_id.to_string();
        Ok(t)
    }

    async fn get_transaction(&self, org_id: &str, id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgTransactionRow> = sqlx::query_as(
            "SELECT id, org_id, account_id, type AS transaction_type, amount, balance_after, description, reference_id, request_id, created_at FROM transactions WHERE org_id = $1 AND id = $2"
        )
        .bind(org_id)
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Transaction::from))
    }

    async fn get_transaction_by_reference(&self, org_id: &str, account_id: &str, reference_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgTransactionRow> = sqlx::query_as(
            "SELECT id, org_id, account_id, type AS transaction_type, amount, balance_after, description, reference_id, request_id, created_at FROM transactions WHERE org_id = $1 AND account_id = $2 AND reference_id = $3"
        )
        .bind(org_id)
        .bind(account_id)
        .bind(reference_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Transaction::from))
    }

    async fn get_transaction_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgTransactionRow> = sqlx::query_as(
            "SELECT id, org_id, account_id, type AS transaction_type, amount, balance_after, description, reference_id, request_id, created_at FROM transactions WHERE org_id = $1 AND request_id = $2"
        )
        .bind(org_id)
        .bind(request_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Transaction::from))
    }

    async fn list_transactions(&self, org_id: &str, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let offset = (page - 1) * page_size;

        let rows: Vec<PgTransactionRow> = sqlx::query_as(
            "SELECT id, org_id, account_id, type AS transaction_type, amount, balance_after, description, reference_id, request_id, created_at FROM transactions WHERE org_id = $1 AND account_id = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        )
        .bind(org_id)
        .bind(account_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;

        let count_row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE org_id = $1 AND account_id = $2"
        )
        .bind(org_id)
        .bind(account_id)
        .fetch_one(self.pool())
        .await?;

        let transactions: Vec<Transaction> = rows.into_iter().map(Transaction::from).collect();
        Ok(PaginatedResponse {
            items: transactions,
            total: count_row.0,
            page,
            page_size,
        })
    }

    // ---- Orgs ----

    async fn create_org(&self, org: CreateOrg) -> Result<Org, DbErr> {
        let row: PgOrgRow = sqlx::query_as::<_, PgOrgRow>(
            "INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW())
             RETURNING id, slug, name, owner_id, created_at, updated_at",
        )
        .bind(&org.id)
        .bind(&org.slug)
        .bind(&org.name)
        .bind(&org.owner_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn get_org(&self, id: &str) -> Result<Option<Org>, DbErr> {
        let row: Option<PgOrgRow> = sqlx::query_as::<_, PgOrgRow>(
            "SELECT id, slug, name, owner_id, created_at, updated_at FROM orgs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Org>, DbErr> {
        let row: Option<PgOrgRow> = sqlx::query_as::<_, PgOrgRow>(
            "SELECT id, slug, name, owner_id, created_at, updated_at FROM orgs WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn list_orgs_for_user(&self, user_id: &str) -> Result<Vec<MembershipSummary>, DbErr> {
        // Manual row parsing: MembershipSummary has a nested Org struct that
        // query_as can't auto-flatten. See Issue A in the task plan.
        let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
            "SELECT o.id, o.slug, o.name, o.owner_id, o.created_at, o.updated_at,
                    m.role, m.group_id
             FROM members m JOIN orgs o ON o.id = m.org_id
             WHERE m.user_id = $1 ORDER BY o.name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let result = rows
            .into_iter()
            .map(|r| {
                let role_str: String = r
                    .try_get("role")
                    .ok()
                    .unwrap_or_else(|| "member".to_string());
                MembershipSummary {
                    org: Org {
                        id: r.try_get("id").unwrap_or_default(),
                        slug: r.try_get("slug").unwrap_or_default(),
                        name: r.try_get("name").unwrap_or_default(),
                        owner_id: r.try_get("owner_id").ok().flatten(),
                        created_at: r
                            .try_get("created_at")
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        updated_at: r
                            .try_get("updated_at")
                            .unwrap_or_else(|_| chrono::Utc::now()),
                    },
                    role: MemberRole::parse(&role_str).unwrap_or(MemberRole::Member),
                    group_id: r.try_get("group_id").ok().flatten(),
                }
            })
            .collect();
        Ok(result)
    }

    async fn update_org(&self, id: &str, updates: UpdateOrg) -> Result<Org, DbErr> {
        // COALESCE per field: None = keep, Some(v) = set. Slug is unique; if
        // the caller passes a duplicate slug the DB raises a constraint error
        // that surfaces as the usual boxed DbErr.
        let row: PgOrgRow = sqlx::query_as::<_, PgOrgRow>(
            "UPDATE orgs
             SET name = COALESCE($2, name),
                 slug = COALESCE($3, slug),
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, slug, name, owner_id, created_at, updated_at",
        )
        .bind(id)
        .bind(&updates.name)
        .bind(&updates.slug)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                // Re-shape RowNotFound into a more descriptive error.
                Box::new(OrgNotFound(id.to_string())) as DbErr
            }
            other => Box::new(other) as DbErr,
        })?;
        Ok(row.into())
    }

    async fn delete_org(&self, id: &str) -> Result<(), DbErr> {
        let result = sqlx::query("DELETE FROM orgs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Box::new(OrgNotFound(id.to_string())));
        }
        Ok(())
    }

    // ---- Members ----

    async fn get_member(&self, user_id: &str, org_id: &str) -> Result<Option<Member>, DbErr> {
        let row: Option<PgMemberRow> = sqlx::query_as(
            "SELECT user_id, org_id, role, group_id, created_by, created_at
             FROM members WHERE user_id = $1 AND org_id = $2",
        )
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Member::from))
    }

    async fn list_members(&self, org_id: &str) -> Result<Vec<MemberWithDetails>, DbErr> {
        let rows: Vec<MemberWithDetails> = sqlx::query_as::<_, MemberWithDetails>(
            r#"
            SELECT
                m.user_id,
                m.org_id,
                u.username,
                u.email,
                m.role,
                m.group_id,
                g.name AS group_name,
                u.enabled,
                COALESCE(a.balance, 0) AS balance,
                COALESCE(a.threshold, $2) AS threshold,
                m.created_at
            FROM members m
            -- INNER JOIN is safe: members.user_id has ON DELETE CASCADE
            -- (migration 20260708000000_saas_orgs.sql), so no member row can
            -- outlive its user. LEFT JOIN would hide FK violations rather than
            -- fail loudly on them.
            JOIN users u ON u.id = m.user_id
            LEFT JOIN groups g ON g.id = m.group_id
            LEFT JOIN accounts a ON a.user_id = m.user_id AND a.org_id = m.org_id
            WHERE m.org_id = $1
            ORDER BY m.created_at ASC
            "#,
        )
        .bind(org_id)
        .bind(DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn upsert_member(&self, member: Member) -> Result<Member, DbErr> {
        // Wrap the member upsert and the paired account INSERT in a single
        // transaction so the per-membership invariant (every members row has
        // a matching accounts row) holds even on partial failure. The account
        // INSERT uses ON CONFLICT DO NOTHING so re-upserting an existing
        // membership (e.g. role change) does not clobber the balance.
        let mut tx = self.pool.begin().await?;

        let row: PgMemberRow = sqlx::query_as(
            "INSERT INTO members (user_id, org_id, role, group_id, created_by)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, org_id) DO UPDATE
               SET role = EXCLUDED.role, group_id = EXCLUDED.group_id, created_by = EXCLUDED.created_by
             RETURNING user_id, org_id, role, group_id, created_by, created_at",
        )
        .bind(&member.user_id)
        .bind(&member.org_id)
        .bind(member.role.as_str())
        .bind(&member.group_id)
        .bind(&member.created_by)
        .fetch_one(&mut *tx)
        .await?;

        let now = chrono::Utc::now();
        let account_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO accounts (id, org_id, user_id, balance, threshold, created_at, updated_at)
             VALUES ($1, $2, $3, 0, $4, $5, $5)
             ON CONFLICT (org_id, user_id) DO NOTHING",
        )
        .bind(&account_id)
        .bind(&member.org_id)
        .bind(&member.user_id)
        .bind(DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Member::from(row))
    }

    async fn update_member_role(
        &self,
        user_id: &str,
        org_id: &str,
        role: MemberRole,
    ) -> Result<(), DbErr> {
        sqlx::query(
            "UPDATE members SET role = $3 WHERE user_id = $1 AND org_id = $2",
        )
        .bind(user_id)
        .bind(org_id)
        .bind(role.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_member(&self, user_id: &str, org_id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM members WHERE user_id = $1 AND org_id = $2")
            .bind(user_id)
            .bind(org_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_owners(&self, org_id: &str) -> Result<i64, DbErr> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM members WHERE org_id = $1 AND role = 'owner'",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn touch_member_last_seen(&self, user_id: &str, org_id: &str) -> Result<(), DbErr> {
        sqlx::query("UPDATE members SET last_seen = NOW() WHERE user_id = $1 AND org_id = $2")
            .bind(user_id)
            .bind(org_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_stale_impersonations(&self, cutoff: chrono::DateTime<chrono::Utc>) -> Result<u64, DbErr> {
        let result = sqlx::query(
            "DELETE FROM members WHERE created_by = 'system' AND last_seen < $1",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ---- Invitations (Phase 3) ----

    async fn create_invitation(
        &self,
        org_id: &str,
        role: &MemberRole,
        created_by: &str,
        recipient_email: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Invitation, DbErr> {
        // Owner is not assignable by invitation (DB CHECK excludes it), but
        // guard here too so the error is friendly rather than a constraint violation.
        let role_str = match role {
            MemberRole::Owner => {
                return Err(format!("cannot mint invitation for role 'owner' (org {org_id})").into());
            }
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
        };
        let token = generate_invitation_token();
        let row: PgInvitationRow = sqlx::query_as(
            "INSERT INTO invitations (token, org_id, role, created_by, recipient_email, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id::text, token, org_id, role, created_by, recipient_email, created_at, expires_at,
                       accepted_at, accepted_by, revoked_at",
        )
        .bind(&token)
        .bind(org_id)
        .bind(role_str)
        .bind(created_by)
        .bind(recipient_email)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(Invitation::from(row))
    }

    async fn get_invitation_by_token(&self, token: &str) -> Result<Option<Invitation>, DbErr> {
        let row: Option<PgInvitationRow> = sqlx::query_as(
            "SELECT id::text, token, org_id, role, created_by, recipient_email, created_at, expires_at,
                    accepted_at, accepted_by, revoked_at
             FROM invitations WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Invitation::from))
    }

    async fn list_invitations_for_org(&self, org_id: &str) -> Result<Vec<Invitation>, DbErr> {
        let rows: Vec<PgInvitationRow> = sqlx::query_as(
            "SELECT id::text, token, org_id, role, created_by, recipient_email, created_at, expires_at,
                    accepted_at, accepted_by, revoked_at
             FROM invitations WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Invitation::from).collect())
    }

    async fn revoke_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> Result<(), DbErr> {
        // Compare `id::text` so any non-UUID string the caller passes simply
        // matches nothing — no-op, no error. (For UPDATE row locks to use the
        // PK index we'd need a real UUID; this is a low-frequency admin path.)
        sqlx::query(
            "UPDATE invitations SET revoked_at = COALESCE(revoked_at, NOW())
             WHERE id::text = $1 AND org_id = $2",
        )
        .bind(invitation_id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn accept_invitation(
        &self,
        token: &str,
        accepting_user_id: &str,
    ) -> Result<Option<Member>, DbErr> {
        // Single transaction: SELECT ... FOR UPDATE serializes concurrent
        // accepts for the same token. Exactly one call hits the UPDATE;
        // later callers see accepted_at IS NOT NULL and bail with None.
        let mut tx = self.pool.begin().await?;

        let row: Option<PgInvitationRow> = sqlx::query_as(
            "SELECT id::text, token, org_id, role, created_by, recipient_email, created_at, expires_at,
                    accepted_at, accepted_by, revoked_at
             FROM invitations WHERE token = $1 FOR UPDATE",
        )
        .bind(token)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(inv) = row else {
            tx.rollback().await?;
            return Ok(None);
        };

        let now = chrono::Utc::now();
        let already_consumed = inv.accepted_at.is_some() || inv.revoked_at.is_some();
        let expired = inv.expires_at < now;
        if already_consumed || expired {
            tx.rollback().await?;
            return Ok(None);
        }

        let role = match MemberRole::parse(&inv.role) {
            Some(r) => r,
            None => {
                tx.rollback().await?;
                return Err(format!("invalid role in invitations row {}: {}", inv.id, inv.role).into());
            }
        };

        // Upsert the membership. If the user is already a member of this org
        // (e.g. they previously accepted a different invitation), update the
        // role to the new invitation's role. PK is (user_id, org_id).
        sqlx::query(
            "INSERT INTO members (user_id, org_id, role, created_by)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (user_id, org_id) DO UPDATE SET role = EXCLUDED.role",
        )
        .bind(accepting_user_id)
        .bind(&inv.org_id)
        .bind(role.as_str())
        .bind(accepting_user_id)
        .execute(&mut *tx)
        .await?;

        // Per-membership invariant: every members row has a matching accounts
        // row. ON CONFLICT DO NOTHING so a re-accept (same user, same org,
        // different invitation) does not clobber the existing balance. Same
        // transaction as the member INSERT — partial state is impossible.
        let account_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO accounts (id, org_id, user_id, balance, threshold, created_at, updated_at)
             VALUES ($1, $2, $3, 0, $4, $5, $5)
             ON CONFLICT (org_id, user_id) DO NOTHING",
        )
        .bind(&account_id)
        .bind(&inv.org_id)
        .bind(accepting_user_id)
        .bind(DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE invitations SET accepted_at = $2, accepted_by = $3 WHERE id::text = $1",
        )
        .bind(&inv.id)
        .bind(now)
        .bind(accepting_user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(Member {
            user_id: accepting_user_id.to_string(),
            org_id: inv.org_id,
            role,
            group_id: None,
            created_by: Some(accepting_user_id.to_string()),
            created_at: now,
        }))
    }

    // ---- Phase 4: users by email ----

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, DbErr> {
        let row: Option<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, platform_role, current_org_id, enabled, refresh_token,
                    created_at, updated_at,
                    email, email_verified_at, requires_email_verification, password_changed_at
             FROM users WHERE LOWER(email) = LOWER($1)",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(User::from))
    }

    async fn set_user_platform_role(
        &self,
        target_user_id: &str,
        _actor_user_id: &str,
        role: Option<PlatformRole>,
        allow_last_admin_override: bool,
    ) -> Result<(), SetPlatformRoleError> {
        let mut tx = self.pool.begin().await?;

        // Lock the target row to prevent concurrent grant/demote racing the count.
        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        )
        .bind(target_user_id)
        .fetch_optional(&mut *tx)
        .await?;
        if exists.is_none() {
            return Err(SetPlatformRoleError::UserNotFound);
        }

        // If demoting, check the count of remaining platform_admins.
        if role.is_none() && !allow_last_admin_override {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM users WHERE platform_role = 'platform_admin'",
            )
            .fetch_one(&mut *tx)
            .await?;
            if count <= 1 {
                return Err(SetPlatformRoleError::LastPlatformAdmin);
            }
        }

        // Apply. None -> NULL (column is TEXT NULL).
        let sql_role: Option<&str> = role.as_ref().map(|_| "platform_admin");
        sqlx::query(
            "UPDATE users SET platform_role = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(sql_role)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn list_platform_admins(&self) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>> {
        let rows: Vec<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, platform_role, current_org_id, enabled, refresh_token,
                    created_at, updated_at,
                    email, email_verified_at, requires_email_verification, password_changed_at
             FROM users WHERE platform_role = 'platform_admin'
             ORDER BY username ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(User::from).collect())
    }

    async fn search_user_candidates(
        &self,
        query: &str,
    ) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let rows: Vec<PgUserRow> = sqlx::query_as(
            "SELECT id, username, password, platform_role, current_org_id, enabled, refresh_token,
                    created_at, updated_at,
                    email, email_verified_at, requires_email_verification, password_changed_at
             FROM users
             WHERE platform_role IS NULL
               AND (LOWER(username) LIKE $1 OR LOWER(COALESCE(email, '')) LIKE $1)
             ORDER BY username ASC
             LIMIT 20",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(User::from).collect())
    }

    async fn set_user_email(
        &self,
        user_id: &str,
        email: &str,
        verified_at: Option<chrono::DateTime<chrono::Utc>>,
        requires_email_verification: bool,
    ) -> Result<User, DbErr> {
        sqlx::query(
            "UPDATE users SET email = $1, email_verified_at = $2, requires_email_verification = $3,
                              updated_at = NOW()
             WHERE id = $4",
        )
        .bind(email)
        .bind(verified_at)
        .bind(requires_email_verification)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        // Re-read for the response — there is no RETURNING on User (PgUserRow
        // is 13 fields, simpler to refetch than maintain a separate shape).
        let row = self
            .get_user(user_id)
            .await?
            .ok_or_else(|| format!("user {user_id} disappeared after set_user_email"))?;
        Ok(row)
    }

    // ---- Phase 4: email_verifications ----

    async fn create_email_verification(
        &self,
        user_id: &str,
        email: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<EmailVerification, DbErr> {
        let token = generate_invitation_token(); // 32-byte base64url; reuse helper
        let row: PgEmailVerificationRow = sqlx::query_as(
            "INSERT INTO email_verifications (token, user_id, email, expires_at)
             VALUES ($1, $2, $3, $4)
             RETURNING id::text, token, user_id, email, created_at, expires_at, consumed_at",
        )
        .bind(&token)
        .bind(user_id)
        .bind(email)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(EmailVerification::from(row))
    }

    async fn get_email_verification_by_token(
        &self,
        token: &str,
    ) -> Result<Option<EmailVerification>, DbErr> {
        let row: Option<PgEmailVerificationRow> = sqlx::query_as(
            "SELECT id::text, token, user_id, email, created_at, expires_at, consumed_at
             FROM email_verifications WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(EmailVerification::from))
    }

    async fn consume_email_verification(&self, token: &str) -> Result<bool, DbErr> {
        let mut tx = self.pool.begin().await?;
        let row: Option<PgEmailVerificationRow> = sqlx::query_as(
            "SELECT id::text, token, user_id, email, created_at, expires_at, consumed_at
             FROM email_verifications WHERE token = $1 FOR UPDATE",
        )
        .bind(token)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.rollback().await?;
            return Ok(false);
        };
        if row.consumed_at.is_some() || row.expires_at < chrono::Utc::now() {
            tx.rollback().await?;
            return Ok(false);
        }

        // Mark consumed in the same txn as the user update so we never get a
        // half-applied state.
        sqlx::query("UPDATE email_verifications SET consumed_at = NOW() WHERE id::text = $1")
            .bind(&row.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE users SET email_verified_at = NOW(), requires_email_verification = FALSE WHERE id = $1")
            .bind(&row.user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    // ---- Phase 4: password_resets ----

    async fn create_password_reset(
        &self,
        user_id: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<PasswordReset, DbErr> {
        let token = generate_invitation_token();
        let row: PgPasswordResetRow = sqlx::query_as(
            "INSERT INTO password_resets (token, user_id, expires_at)
             VALUES ($1, $2, $3)
             RETURNING id::text, token, user_id, created_at, expires_at, consumed_at",
        )
        .bind(&token)
        .bind(user_id)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(PasswordReset::from(row))
    }

    async fn get_password_reset_by_token(
        &self,
        token: &str,
    ) -> Result<Option<PasswordReset>, DbErr> {
        let row: Option<PgPasswordResetRow> = sqlx::query_as(
            "SELECT id::text, token, user_id, created_at, expires_at, consumed_at
             FROM password_resets WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(PasswordReset::from))
    }

    async fn consume_password_reset_and_set_password(
        &self,
        token: &str,
        new_password_hash: &str,
    ) -> Result<PasswordResetOutcome, DbErr> {
        let mut tx = self.pool.begin().await?;
        let row: Option<PgPasswordResetRow> = sqlx::query_as(
            "SELECT id::text, token, user_id, created_at, expires_at, consumed_at
             FROM password_resets WHERE token = $1 FOR UPDATE",
        )
        .bind(token)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.rollback().await?;
            return Ok(PasswordResetOutcome::NotFound);
        };
        if row.consumed_at.is_some() {
            tx.rollback().await?;
            return Ok(PasswordResetOutcome::Consumed);
        }
        if row.expires_at < chrono::Utc::now() {
            tx.rollback().await?;
            return Ok(PasswordResetOutcome::Expired);
        }
        sqlx::query("UPDATE password_resets SET consumed_at = NOW() WHERE id::text = $1")
            .bind(&row.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE users SET password = $1, password_changed_at = NOW(), updated_at = NOW() WHERE id = $2",
        )
        .bind(new_password_hash)
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(PasswordResetOutcome::Success)
    }

    // ---- Settings (platform + org) ----

    async fn get_platform_setting(&self, key: &str) -> Result<Option<String>, DbErr> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM platform_settings WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    async fn set_platform_setting(&self, key: &str, value: &str) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO platform_settings (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_org_setting(&self, org_id: &str, key: &str) -> Result<Option<String>, DbErr> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM org_settings WHERE org_id = $1 AND key = $2",
        )
        .bind(org_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    async fn set_org_setting(&self, org_id: &str, key: &str, value: &str) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO org_settings (org_id, key, value) VALUES ($1, $2, $3)
             ON CONFLICT (org_id, key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(org_id)
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_org_settings(&self, org_id: &str) -> Result<Vec<(String, String)>, DbErr> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM org_settings WHERE org_id = $1 ORDER BY key",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_org_defaults(
        &self,
        org_id: &str,
    ) -> Result<crate::types::OrgDefaults, DbErr> {
        let rpm = self
            .get_org_setting(org_id, "default_rate_limit_rpm")
            .await?
            .and_then(|s| s.parse::<i64>().ok());
        let budget = self
            .get_org_setting(org_id, "default_budget_monthly_usd")
            .await?
            .and_then(|s| s.parse::<i64>().ok());
        Ok(crate::types::OrgDefaults {
            default_rate_limit_rpm: rpm,
            default_budget_monthly_usd: budget,
        })
    }

    async fn set_org_defaults(
        &self,
        org_id: &str,
        defaults: &crate::types::OrgDefaults,
    ) -> Result<(), DbErr> {
        // Wrap both writes in a single transaction so a pool error or crash
        // between them can't leave partial state (the trait doc promises
        // atomicity — see crates/storage/src/lib.rs).
        let mut tx = self.pool.begin().await?;
        match defaults.default_rate_limit_rpm {
            Some(n) => {
                sqlx::query(
                    "INSERT INTO org_settings (org_id, key, value) VALUES ($1, 'default_rate_limit_rpm', $2)
                     ON CONFLICT (org_id, key) DO UPDATE SET value = EXCLUDED.value",
                )
                .bind(org_id)
                .bind(n.to_string())
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query(
                    "DELETE FROM org_settings WHERE org_id = $1 AND key = 'default_rate_limit_rpm'",
                )
                .bind(org_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        match defaults.default_budget_monthly_usd {
            Some(n) => {
                sqlx::query(
                    "INSERT INTO org_settings (org_id, key, value) VALUES ($1, 'default_budget_monthly_usd', $2)
                     ON CONFLICT (org_id, key) DO UPDATE SET value = EXCLUDED.value",
                )
                .bind(org_id)
                .bind(n.to_string())
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query(
                    "DELETE FROM org_settings WHERE org_id = $1 AND key = 'default_budget_monthly_usd'",
                )
                .bind(org_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper types for the new methods above
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct PgMemberRow {
    user_id: String,
    org_id: String,
    role: String,
    group_id: Option<String>,
    created_by: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct PgOrgRow {
    id: String,
    slug: String,
    name: String,
    owner_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgOrgRow> for Org {
    fn from(r: PgOrgRow) -> Self {
        Org {
            id: r.id,
            slug: r.slug,
            name: r.name,
            owner_id: r.owner_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

impl From<PgMemberRow> for Member {
    fn from(r: PgMemberRow) -> Self {
        Member {
            user_id: r.user_id,
            org_id: r.org_id,
            role: MemberRole::parse(&r.role).unwrap_or(MemberRole::Member),
            group_id: r.group_id,
            created_by: r.created_by,
            created_at: r.created_at,
        }
    }
}

/// Lightweight error returned by `update_org` / `delete_org` when the org
/// is missing. Kept local to avoid pulling in `org::OrgError` (which would
/// create a circular dependency — `org` already depends on `storage`).
#[derive(Debug)]
struct OrgNotFound(String);
impl std::fmt::Display for OrgNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "org not found: {}", self.0)
    }
}
impl std::error::Error for OrgNotFound {}

// ---------------------------------------------------------------------------
// Invitation helpers
// ---------------------------------------------------------------------------

/// Generate an opaque 32-byte invitation token, base64url-encoded without
/// padding. 256 bits of entropy from the OS CSPRNG; the `invitations.token`
/// column has a UNIQUE constraint so collisions would surface as a DB error
/// (and at 2^256 the probability is not a practical concern).
fn generate_invitation_token() -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(sqlx::FromRow)]
struct PgInvitationRow {
    // Selected as `id::text` so we don't need sqlx's `uuid` feature enabled.
    // The invitations.id column is UUID, but the rest of this crate talks to
    // every other PK as TEXT — keep the boundary in String.
    id: String,
    token: String,
    org_id: String,
    role: String,
    created_by: String,
    recipient_email: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    accepted_by: Option<String>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PgInvitationRow> for Invitation {
    fn from(r: PgInvitationRow) -> Self {
        Invitation {
            id: r.id,
            token: r.token,
            org_id: r.org_id,
            role: MemberRole::parse(&r.role).unwrap_or(MemberRole::Member),
            created_by: r.created_by,
            recipient_email: r.recipient_email,
            created_at: r.created_at,
            expires_at: r.expires_at,
            accepted_at: r.accepted_at,
            accepted_by: r.accepted_by,
            revoked_at: r.revoked_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PgEmailVerificationRow {
    id: String,
    token: String,
    user_id: String,
    email: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    consumed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PgEmailVerificationRow> for EmailVerification {
    fn from(r: PgEmailVerificationRow) -> Self {
        EmailVerification {
            id: r.id,
            token: r.token,
            user_id: r.user_id,
            email: r.email,
            created_at: r.created_at,
            expires_at: r.expires_at,
            consumed_at: r.consumed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PgPasswordResetRow {
    id: String,
    token: String,
    user_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    consumed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PgPasswordResetRow> for PasswordReset {
    fn from(r: PgPasswordResetRow) -> Self {
        PasswordReset {
            id: r.id,
            token: r.token,
            user_id: r.user_id,
            created_at: r.created_at,
            expires_at: r.expires_at,
            consumed_at: r.consumed_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Storage;

    #[tokio::test]
    async fn test_insert_log_round_trip_with_routes() {
        use crate::types::{AuditLog, Protocol, RouteAttempt};
        let url = match std::env::var("DATABASE_URL") {
            Ok(u) => u,
            Err(_) => {
                eprintln!("DATABASE_URL not set; skipping test");
                return;
            }
        };
        let storage = PostgresStorage::new(&url).await.expect("connect");
        storage.run_migrations().await.expect("migrate");

        let now = chrono::Utc::now();

        // Ensure the default org exists (the migration creates it but the
        // shared test DB may be in any state). Use ON CONFLICT so this is
        // idempotent.
        let _ = sqlx::query(
            "INSERT INTO orgs (id, slug, name, created_at, updated_at)
             VALUES ('org_test', 'test-org', 'Test Org', NOW(), NOW())
             ON CONFLICT (id) DO NOTHING",
        )
        .execute(&storage.pool)
        .await;

        // Use a synthetic API key. Insert it if missing.
        let key_id = "test-routes-key";
        let _ = sqlx::query("INSERT INTO api_keys (id, org_id, name, key_hash, enabled, created_at, updated_at) VALUES ($1, $2, $3, $4, true, $5, $6) ON CONFLICT (id) DO NOTHING")
            .bind(key_id)
            .bind("org_test")
            .bind("test-routes")
            .bind("0000000000000000000000000000000000000000000000000000000000000000")
            .bind(now)
            .bind(now)
            .execute(&storage.pool)
            .await
            .expect("seed key");

        let routes = vec![
            RouteAttempt {
                model: "glm-5.2".to_string(),
                channel_id: "ch-a".to_string(),
                channel_name: Some("Channel A".to_string()),
                provider_id: "p-a".to_string(),
                status_code: 0,
                error_message: Some("Connection refused".to_string()),
                latency_ms: 5,
                started_at: now,
            },
            RouteAttempt {
                model: "glm-5.2".to_string(),
                channel_id: "ch-b".to_string(),
                channel_name: Some("Channel B".to_string()),
                provider_id: "p-b".to_string(),
                status_code: 500,
                error_message: Some("Internal Server Error".to_string()),
                latency_ms: 150,
                started_at: now,
            },
            RouteAttempt {
                model: "minimax-3".to_string(),
                channel_id: "ch-c".to_string(),
                channel_name: Some("Channel C".to_string()),
                provider_id: "p-c".to_string(),
                status_code: 200,
                error_message: None,
                latency_ms: 320,
                started_at: now,
            },
        ];

        let log = AuditLog {
            id: format!("test-routes-{}", uuid::Uuid::new_v4()),
            org_id: "org_test".to_string(),
            request_id: Some(format!("test-req-{}", uuid::Uuid::new_v4())),
            key_id: key_id.to_string(),
            user_id: None,
            model_name: "minimax-3".to_string(),
            provider_id: "test-prov".to_string(),
            channel_id: Some("ch-c".to_string()),
            channel_name: Some("Channel C".to_string()),
            protocol: Protocol::Openai,
            stream: false,
            request_body: r#"{"model":"glm-5.2"}"#.to_string(),
            response_body: r#"{"ok":true}"#.to_string(),
            status_code: 200,
            latency_ms: 500,
            input_tokens: Some(10),
            output_tokens: Some(20),
            created_at: now,
            original_model: Some("glm-5.2".to_string()),
            upstream_model: Some("minimax-3".to_string()),
            model_override_reason: Some("channel_mapping".to_string()),
            request_path: Some("/v1/chat/completions".to_string()),
            upstream_url: Some("https://example.com/v1/chat/completions".to_string()),
            request_headers: None,
            response_headers: None,
            actor_is_platform_admin: false,
            routes: Some(routes.clone()),
        };

        storage.insert_log("org_test", &log).await.expect("insert");

        let fetched = storage
            .get_audit_by_request_id("org_test", log.request_id.as_deref().unwrap())
            .await
            .expect("fetch")
            .expect("found");

        let fetched_routes = fetched.routes.expect("routes present");
        assert_eq!(fetched_routes.len(), 3);
        assert_eq!(fetched_routes[0].channel_id, "ch-a");
        assert_eq!(fetched_routes[0].status_code, 0);
        assert_eq!(fetched_routes[0].error_message.as_deref(), Some("Connection refused"));
        assert_eq!(fetched_routes[1].channel_id, "ch-b");
        assert_eq!(fetched_routes[1].status_code, 500);
        assert_eq!(fetched_routes[2].channel_id, "ch-c");
        assert_eq!(fetched_routes[2].status_code, 200);
        assert!(fetched_routes[2].error_message.is_none());

        // Regression: get_log (by id) must also round-trip routes.
        // Previously its SELECT missed the routes column, producing
        // sqlx ColumnNotFound("routes") at this call site.
        let by_id = storage
            .get_log("org_test", &log.id)
            .await
            .expect("get_log fetch")
            .expect("get_log found");
        let by_id_routes = by_id.routes.expect("routes present via get_log");
        assert_eq!(by_id_routes.len(), 3);
        assert_eq!(by_id_routes[0].channel_id, "ch-a");

        // Cleanup: remove the rows we inserted so the test is idempotent
        // and doesn't leave garbage in the shared test DB.
        sqlx::query("DELETE FROM audit_logs WHERE id = $1")
            .bind(&log.id)
            .execute(&storage.pool)
            .await
            .expect("cleanup audit_logs");
        sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(key_id)
            .execute(&storage.pool)
            .await
            .expect("cleanup api_keys");
    }
}

#[cfg(test)]
mod org_tests {
    use super::*;
    use crate::Storage;

    /// Round-trips membership on the default org: inserts a synthetic user
    /// into the default org, calls `upsert_member`, and verifies the row
    /// appears in `list_members` with the role that was written.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn bootstrap_default_org_round_trip_membership(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);

        let org = storage
            .get_org("org_default")
            .await
            .expect("get_org")
            .expect("default org exists after migration");

        assert_eq!(org.slug, "default");
        assert_eq!(org.name, "Default Org");

        // The migration only creates memberships for users that already existed
        // when it ran. In a fresh sqlx::test DB there are no users, so we
        // insert a synthetic admin-style user + verify the membership backfill
        // path manually instead.
        sqlx::query(
            "INSERT INTO users (id, username, password, created_at, updated_at)
             VALUES ('u-bootstrap-test', 'bootstrap_test', 'x', NOW(), NOW())
             ON CONFLICT (id) DO NOTHING",
        )
        .execute(&storage.pool)
        .await
        .expect("insert user");

        storage
            .upsert_member(Member {
                user_id: "u-bootstrap-test".to_string(),
                org_id: "org_default".to_string(),
                role: MemberRole::Owner,
                group_id: None,
                created_by: Some("u-bootstrap-test".to_string()),
                created_at: chrono::Utc::now(),
            })
            .await
            .expect("upsert_member");

        // Invariant (Task 2 follow-up): upsert_member must create a matching
        // accounts row. Without this assertion the next storage refactor could
        // silently drop the account-creation side-effect.
        let account_balance: i64 = sqlx::query_scalar(
            "SELECT balance FROM accounts WHERE org_id = $1 AND user_id = $2",
        )
        .bind("org_default")
        .bind("u-bootstrap-test")
        .fetch_one(&storage.pool)
        .await
        .expect("account row must exist after upsert_member");
        assert_eq!(account_balance, 0);
        let account_threshold: i64 = sqlx::query_scalar(
            "SELECT threshold FROM accounts WHERE org_id = $1 AND user_id = $2",
        )
        .bind("org_default")
        .bind("u-bootstrap-test")
        .fetch_one(&storage.pool)
        .await
        .expect("account row threshold must exist");
        assert_eq!(account_threshold, DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS);

        let members = storage.list_members("org_default").await.expect("list_members");
        let found = members
            .iter()
            .find(|m| m.user_id == "u-bootstrap-test")
            .expect("membership row present");
        assert_eq!(found.role, MemberRole::Owner.as_str());
        // The joined shape must also surface the user row.
        assert_eq!(found.username, "bootstrap_test");
        // Per-membership invariant surfaced through the join: every member
        // has an accounts row (created by upsert_member), so balance is 0
        // (not NULL → 0 via COALESCE, but a real 0).
        assert_eq!(found.balance, 0);
        assert_eq!(found.threshold, DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS);

        // cleanup
        sqlx::query("DELETE FROM accounts WHERE user_id = 'u-bootstrap-test' AND org_id = 'org_default'")
            .execute(&storage.pool)
            .await
            .expect("cleanup accounts");
        sqlx::query("DELETE FROM members WHERE user_id = 'u-bootstrap-test' AND org_id = 'org_default'")
            .execute(&storage.pool)
            .await
            .expect("cleanup members");
        sqlx::query("DELETE FROM users WHERE id = 'u-bootstrap-test'")
            .execute(&storage.pool)
            .await
            .expect("cleanup users");
    }

    /// Catalog visibility: platform-level rows (owner_org_id IS NULL) are visible
    /// to every org; org-private rows are visible only to their owner org.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn catalog_visibility_filter_works(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);

        // Two orgs.
        for slug in &["org-a-vis", "org-b-vis"] {
            // Owner user must exist first — orgs.owner_id has a deferred FK to users(id).
            sqlx::query(
                "INSERT INTO users (id, username, password, created_at, updated_at)
                 VALUES ($1, $2, 'x', NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
            )
            .bind(format!("u-{}", slug))
            .bind(slug)
            .execute(&storage.pool)
            .await
            .expect("insert owner user");
            storage
                .create_org(crate::types::CreateOrg {
                    id: slug.to_string(),
                    slug: slug.to_string(),
                    name: format!("{} org", slug),
                    owner_id: format!("u-{}", slug),
                })
                .await
                .expect("create_org");
        }

        // Platform-level model (owner_org_id = NULL) — visible to everyone.
        let platform_model = Model {
            id: "m-platform-vis".to_string(),
            name: "platform-visible-model".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: None,
            created_at: chrono::Utc::now(),
        };
        storage
            .create_model("org-a-vis", &platform_model)
            .await
            .expect("create platform-level model");

        // Org-A-private model — visible only to org-a-vis.
        let org_a_model = Model {
            id: "m-orga-private".to_string(),
            name: "orga-private-model".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: Some("org-a-vis".to_string()),
            created_at: chrono::Utc::now(),
        };
        storage
            .create_model("org-a-vis", &org_a_model)
            .await
            .expect("create org-a-private model");

        // Org-A sees both.
        let names_a: Vec<String> = storage
            .list_models("org-a-vis")
            .await
            .expect("list models for org-a")
            .into_iter()
            .map(|m| m.model.name)
            .collect();
        assert!(
            names_a.iter().any(|n| n == "platform-visible-model"),
            "org-a should see the platform model: {:?}",
            names_a
        );
        assert!(
            names_a.iter().any(|n| n == "orga-private-model"),
            "org-a should see its own private model: {:?}",
            names_a
        );

        // Org-B sees only the platform model, NOT org-a's private model.
        let names_b: Vec<String> = storage
            .list_models("org-b-vis")
            .await
            .expect("list models for org-b")
            .into_iter()
            .map(|m| m.model.name)
            .collect();
        assert!(
            names_b.iter().any(|n| n == "platform-visible-model"),
            "org-b should see the platform model: {:?}",
            names_b
        );
        assert!(
            !names_b.iter().any(|n| n == "orga-private-model"),
            "org-b must NOT see org-a's private model: {:?}",
            names_b
        );

        // Cleanup.
        sqlx::query("DELETE FROM models WHERE id IN ('m-platform-vis', 'm-orga-private')")
            .execute(&storage.pool)
            .await
            .expect("cleanup models");
        sqlx::query("DELETE FROM orgs WHERE id IN ('org-a-vis', 'org-b-vis')")
            .execute(&storage.pool)
            .await
            .expect("cleanup orgs");
        sqlx::query("DELETE FROM users WHERE id IN ('u-org-a-vis', 'u-org-b-vis')")
            .execute(&storage.pool)
            .await
            .expect("cleanup users");
    }

    /// Anti-shadowing: an org-private model must not be allowed to reuse a name
    /// that's already taken at the platform level. This protects the catalog
    /// invariant that a model name uniquely identifies either a platform row or
    /// a row owned by exactly one org — never both.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn anti_shadowing_rejects_org_private_with_platform_name(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);

        // Org for the private model.
        // Owner user first (FK constraint on orgs.owner_id → users.id is deferred
        // but committing a tx without the user present still fails).
        sqlx::query(
            "INSERT INTO users (id, username, password, created_at, updated_at)
             VALUES ('u-shadow', 'shadow', 'x', NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .execute(&storage.pool)
        .await
        .expect("insert owner user");
        storage
            .create_org(crate::types::CreateOrg {
                id: "org-shadow".to_string(),
                slug: "org-shadow".to_string(),
                name: "Shadow Org".to_string(),
                owner_id: "u-shadow".to_string(),
            })
            .await
            .expect("create_org");

        // Platform-level model named "gpt-4".
        let platform = Model {
            id: "m-platform-shadow".to_string(),
            name: "gpt-4".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: None,
            created_at: chrono::Utc::now(),
        };
        storage
            .create_model("org-shadow", &platform)
            .await
            .expect("create platform model");

        // Now attempt an org-private model with the SAME name. Should fail.
        let attempt = Model {
            id: "m-shadow-attempt".to_string(),
            name: "gpt-4".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: Some("org-shadow".to_string()),
            created_at: chrono::Utc::now(),
        };
        let res = storage.create_model("org-shadow", &attempt).await;
        assert!(
            res.is_err(),
            "creating an org-private model with a name that's already platform-level must fail"
        );
        // Sanity-check the error message mentions the reserved name.
        let msg = res.unwrap_err().to_string();
        assert!(
            msg.contains("gpt-4"),
            "error should reference the reserved name, got: {}",
            msg
        );

        // Cleanup.
        sqlx::query("DELETE FROM models WHERE id = 'm-platform-shadow'")
            .execute(&storage.pool)
            .await
            .expect("cleanup platform model");
        sqlx::query("DELETE FROM orgs WHERE id = 'org-shadow'")
            .execute(&storage.pool)
            .await
            .expect("cleanup org");
        sqlx::query("DELETE FROM users WHERE id = 'u-shadow'")
            .execute(&storage.pool)
            .await
            .expect("cleanup user");
    }

    /// Anti-shadowing check must NOT false-positive on names that are unique
    /// across platform + org scopes. Confirms the SELECT-WHERE-owner_org_id IS NULL
    /// predicate only matches platform-level rows.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn anti_shadowing_allows_unique_org_private_name(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);

        // Owner user + org (required by FK on orgs.owner_id).
        sqlx::query(
            "INSERT INTO users (id, username, password, created_at, updated_at)
             VALUES ('u-shadow-2', 'shadow2', 'x', NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .execute(&storage.pool)
        .await
        .expect("insert owner user");
        storage
            .create_org(crate::types::CreateOrg {
                id: "org-shadow-2".to_string(),
                slug: "org-shadow-2".to_string(),
                name: "Shadow Org 2".to_string(),
                owner_id: "u-shadow-2".to_string(),
            })
            .await
            .expect("create_org");

        // Platform-level "gpt-4".
        let platform = Model {
            id: "m-platform-unique".to_string(),
            name: "gpt-4".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: None,
            created_at: chrono::Utc::now(),
        };
        storage
            .create_model("org-shadow-2", &platform)
            .await
            .expect("seed platform model");

        // Org-private "my-finetune" — different name, must succeed.
        let org_private = Model {
            id: "m-org-unique".to_string(),
            name: "my-finetune".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: Some("org-shadow-2".to_string()),
            created_at: chrono::Utc::now(),
        };
        let result = storage.create_model("org-shadow-2", &org_private).await;
        assert!(
            result.is_ok(),
            "org-private model with unique name should succeed, got: {:?}",
            result
        );

        // Cleanup.
        sqlx::query("DELETE FROM models WHERE id IN ('m-platform-unique', 'm-org-unique')")
            .execute(&storage.pool)
            .await
            .expect("cleanup models");
        sqlx::query("DELETE FROM orgs WHERE id = 'org-shadow-2'")
            .execute(&storage.pool)
            .await
            .expect("cleanup org");
        sqlx::query("DELETE FROM users WHERE id = 'u-shadow-2'")
            .execute(&storage.pool)
            .await
            .expect("cleanup user");
    }

    /// Platform-level entry CAN be created even if an org-private entry with the
    /// same name already exists. The rule is directional: only org→platform
    /// shadowing is forbidden. This matches the visibility filter
    /// `(owner_org_id IS NULL OR owner_org_id = $1)` — platform rows are visible
    /// to everyone, so the org-private entry simply becomes unreachable from the
    /// creating org's perspective (acceptable; they asked for it).
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn anti_shadowing_allows_platform_to_shadow_org(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);

        sqlx::query(
            "INSERT INTO users (id, username, password, created_at, updated_at)
             VALUES ('u-shadow-3', 'shadow3', 'x', NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .execute(&storage.pool)
        .await
        .expect("insert owner user");
        storage
            .create_org(crate::types::CreateOrg {
                id: "org-shadow-3".to_string(),
                slug: "org-shadow-3".to_string(),
                name: "Shadow Org 3".to_string(),
                owner_id: "u-shadow-3".to_string(),
            })
            .await
            .expect("create_org");

        // Org-private first.
        let org_private = Model {
            id: "m-org-first".to_string(),
            name: "shared-name".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: Some("org-shadow-3".to_string()),
            created_at: chrono::Utc::now(),
        };
        storage
            .create_model("org-shadow-3", &org_private)
            .await
            .expect("seed org-private model");

        // Platform-level with same name — must succeed (directional rule).
        let platform = Model {
            id: "m-platform-second".to_string(),
            name: "shared-name".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: None,
            created_at: chrono::Utc::now(),
        };
        let result = storage.create_model("org-shadow-3", &platform).await;
        assert!(
            result.is_ok(),
            "platform-level entry with same name as an org-private entry should succeed (rule is directional), got: {:?}",
            result
        );

        // Cleanup.
        sqlx::query("DELETE FROM models WHERE id IN ('m-org-first', 'm-platform-second')")
            .execute(&storage.pool)
            .await
            .expect("cleanup models");
        sqlx::query("DELETE FROM orgs WHERE id = 'org-shadow-3'")
            .execute(&storage.pool)
            .await
            .expect("cleanup org");
        sqlx::query("DELETE FROM users WHERE id = 'u-shadow-3'")
            .execute(&storage.pool)
            .await
            .expect("cleanup user");
    }

    /// Regression: an org admin calling `set_provider_models` must NOT be able
    /// to delete platform-level (owner_org_id IS NULL) provider↔model rows.
    /// The previous implementation's DELETE matched both `IS NULL` and `= $2`,
    /// which let any org admin wipe platform-wide mappings by calling
    /// PUT /admin/providers/{id}/models with whatever payload.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn set_provider_models_preserves_platform_rows(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);

        // ── Seed: user, two orgs, provider, model, platform-level mapping ──
        sqlx::query(
            "INSERT INTO users (id, username, password, created_at, updated_at)
             VALUES ('u-pm-1', 'pmuser', 'x', NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .execute(&storage.pool)
        .await
        .expect("insert user");
        for org_id in ["org-pm-alpha", "org-pm-beta"] {
            storage
                .create_org(crate::types::CreateOrg {
                    id: org_id.to_string(),
                    slug: org_id.to_string(),
                    name: format!("{org_id} name"),
                    owner_id: "u-pm-1".to_string(),
                })
                .await
                .expect("create_org");
        }
        let provider = crate::types::Provider {
            id: "prov-pm-1".to_string(),
            owner_org_id: None,
            name: "PM Provider".into(),
            slug: "pm-prov".into(),
            endpoints: None,
            proxy_url: None,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        storage.create_provider("org-pm-alpha", &provider).await.expect("create_provider");
        let model = crate::types::Model {
            id: "m-pm-1".to_string(),
            name: "pm-model".to_string(),
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: None,
            created_at: chrono::Utc::now(),
        };
        storage.create_model("org-pm-alpha", &model).await.expect("create_model");

        // Platform-level provider↔model mapping (owner_org_id IS NULL).
        sqlx::query(
            "INSERT INTO provider_models (provider_id, model_id, owner_org_id, upstream_name, pricing_policy_id, created_at)
             VALUES ('prov-pm-1', 'm-pm-1', NULL, NULL, NULL, NOW())",
        )
        .execute(&storage.pool)
        .await
        .expect("seed platform provider_model");

        // ── Act: org-pm-beta "replaces" its mappings with an empty set ──
        storage
            .set_provider_models("org-pm-beta", "prov-pm-1", vec![])
            .await
            .expect("set_provider_models with empty list");

        // ── Assert: platform-level row is still there ──
        let surviving: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM provider_models
             WHERE provider_id = 'prov-pm-1' AND owner_org_id IS NULL",
        )
        .fetch_one(&storage.pool)
        .await
        .expect("count platform rows");
        assert_eq!(
            surviving.0, 1,
            "platform-level provider↔model mapping must survive an org-scoped set_provider_models call"
        );

        // Cleanup.
        sqlx::query("DELETE FROM provider_models WHERE provider_id = 'prov-pm-1'")
            .execute(&storage.pool).await.expect("cleanup provider_models");
        sqlx::query("DELETE FROM models WHERE id = 'm-pm-1'")
            .execute(&storage.pool).await.expect("cleanup model");
        sqlx::query("DELETE FROM providers WHERE id = 'prov-pm-1'")
            .execute(&storage.pool).await.expect("cleanup provider");
        for org_id in ["org-pm-alpha", "org-pm-beta"] {
            sqlx::query(&format!("DELETE FROM orgs WHERE id = '{org_id}'"))
                .execute(&storage.pool).await.expect("cleanup org");
        }
        sqlx::query("DELETE FROM users WHERE id = 'u-pm-1'")
            .execute(&storage.pool).await.expect("cleanup user");
    }
}

#[cfg(test)]
mod invitation_tests {
    use super::*;
    use crate::Storage;

    /// Helper: create a real Org row via the storage trait. Orgs.owner_id has
    /// a deferred FK to users(id), so the owner must be inserted first; this
    /// helper takes care of both. The slug/name are derived from `id` so the
    /// caller can predict them without capturing the return value.
    async fn make_test_org(storage: &PostgresStorage, id: &str, name: &str) -> crate::types::Org {
        let owner_id = format!("owner-{id}");
        sqlx::query(
            "INSERT INTO users (id, username, password, created_at, updated_at)
             VALUES ($1, $2, 'x', NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .bind(&owner_id)
        .bind(&owner_id)
        .execute(&storage.pool)
        .await
        .expect("insert owner user for org");
        storage
            .create_org(crate::types::CreateOrg {
                id: id.to_string(),
                slug: id.to_string(),
                name: name.to_string(),
                owner_id: owner_id.clone(),
            })
            .await
            .expect("create_org")
    }

    /// Helper: create a real User row. `username` is used as the id and the
    /// username column to keep the call sites short.
    async fn make_test_user(storage: &PostgresStorage, username: &str) -> crate::types::User {
        let now = chrono::Utc::now();
        let user = crate::types::User {
            id: username.to_string(),
            username: username.to_string(),
            password: "x".to_string(),
            platform_role: None,
            current_org_id: None,
            enabled: true,
            refresh_token: None,
            created_at: now,
            updated_at: now,
            email: None,
            email_verified_at: None,
            requires_email_verification: false,
            password_changed_at: now,
        };
        storage.create_user(&user).await.expect("create_user")
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn invitation_lifecycle_round_trip(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "acme", "Acme").await;
        let inviter = make_test_user(&storage, "alice").await;
        let now = chrono::Utc::now();

        let invitation = storage
            .create_invitation(
                &org.id,
                &crate::types::MemberRole::Admin,
                &inviter.id,
                "alice@example.com",
                now + chrono::Duration::days(7),
            )
            .await
            .expect("mint");
        assert!(invitation.accepted_at.is_none());
        assert!(invitation.revoked_at.is_none());
        assert_eq!(
            invitation.recipient_email.as_deref(),
            Some("alice@example.com")
        );

        let fetched = storage
            .get_invitation_by_token(&invitation.token)
            .await
            .expect("get_by_token")
            .expect("invitation present");
        assert_eq!(fetched.id, invitation.id);

        let pending = storage.list_invitations_for_org(&org.id).await.expect("list");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, invitation.id);

        storage
            .revoke_invitation(&org.id, &invitation.id)
            .await
            .expect("revoke");
        let revoked = storage
            .get_invitation_by_token(&invitation.token)
            .await
            .expect("get")
            .expect("present");
        assert!(revoked.revoked_at.is_some());
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn invitation_accept_creates_membership_and_marks_consumed(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "acme", "Acme").await;
        let inviter = make_test_user(&storage, "alice").await;
        let invitee = make_test_user(&storage, "bob").await;
        let now = chrono::Utc::now();

        let invitation = storage
            .create_invitation(
                &org.id,
                &crate::types::MemberRole::Member,
                &inviter.id,
                "bob@example.com",
                now + chrono::Duration::days(7),
            )
            .await
            .expect("mint");

        let member = storage
            .accept_invitation(&invitation.token, &invitee.id)
            .await
            .expect("accept")
            .expect("invitation was consumable");
        assert_eq!(member.user_id, invitee.id);
        assert_eq!(member.org_id, org.id);
        assert_eq!(member.role, crate::types::MemberRole::Member);

        // Invariant (Task 2 follow-up): accept_invitation must create a
        // matching accounts row alongside the membership row.
        let account_balance: i64 = sqlx::query_scalar(
            "SELECT balance FROM accounts WHERE org_id = $1 AND user_id = $2",
        )
        .bind(&org.id)
        .bind(&invitee.id)
        .fetch_one(&storage.pool)
        .await
        .expect("account row must exist after accept_invitation");
        assert_eq!(account_balance, 0);

        let second = storage
            .accept_invitation(&invitation.token, &invitee.id)
            .await
            .expect("no db error");
        assert!(second.is_none(), "second accept should be no-op");
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn invitation_token_entropy_is_unique(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "acme", "Acme").await;
        let inviter = make_test_user(&storage, "alice").await;
        let now = chrono::Utc::now();

        let mut seen = std::collections::HashSet::new();
        for i in 0..1000 {
            let inv = storage
                .create_invitation(
                    &org.id,
                    &crate::types::MemberRole::Member,
                    &inviter.id,
                    &format!("invitee-{i}@example.com"),
                    now + chrono::Duration::days(7),
                )
                .await
                .expect("mint");
            assert!(seen.insert(inv.token), "duplicate token generated");
        }
    }

    /// Org defaults round-trip: writes both fields, reads them back, verifies
    /// `None` is preserved, and confirms no cross-org interference.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn org_defaults_round_trip(pool: sqlx::PgPool) {
        use crate::types::OrgDefaults;

        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-a", "Org A").await;

        // 1. Initial state — both None.
        let initial = storage.get_org_defaults(&org.id).await.expect("get initial");
        assert_eq!(initial, OrgDefaults {
            default_rate_limit_rpm: None,
            default_budget_monthly_usd: None,
        });

        // 2. Write both fields.
        storage
            .set_org_defaults(&org.id, &OrgDefaults {
                default_rate_limit_rpm: Some(100),
                default_budget_monthly_usd: Some(5000),  // subunits (10⁸ per USD)
            })
            .await
            .expect("set both");

        let after = storage.get_org_defaults(&org.id).await.expect("get after set");
        assert_eq!(after.default_rate_limit_rpm, Some(100));
        assert_eq!(after.default_budget_monthly_usd, Some(5000));

        // 3. Clear rate limit only — budget must persist.
        storage
            .set_org_defaults(&org.id, &OrgDefaults {
                default_rate_limit_rpm: None,
                default_budget_monthly_usd: Some(5000),
            })
            .await
            .expect("clear rate limit");
        let cleared = storage.get_org_defaults(&org.id).await.expect("get after clear");
        assert_eq!(cleared.default_rate_limit_rpm, None);
        assert_eq!(cleared.default_budget_monthly_usd, Some(5000));

        // 4. Different org — independent state.
        let org_b = make_test_org(&storage, "org-b", "Org B").await;
        let b_initial = storage.get_org_defaults(&org_b.id).await.expect("get b initial");
        assert_eq!(b_initial, OrgDefaults {
            default_rate_limit_rpm: None,
            default_budget_monthly_usd: None,
        });
    }

    // ---- Phase 6: budget_counters ----

    /// Helper: create a real org + api_key row pair so usage_records FK and
    /// budget_counters FK are satisfied. Returns the key_id.
    async fn make_test_key_for_budget(storage: &PostgresStorage, org_id: &str, key_id: &str) -> String {
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO api_keys (id, org_id, name, key_hash, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, true, $5, $6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(key_id)
        .bind(org_id)
        .bind(key_id)
        .bind(format!("{key_id:0>64}")) // 64-char placeholder hash
        .bind(now)
        .bind(now)
        .execute(&storage.pool)
        .await
        .expect("seed api_key");
        key_id.to_string()
    }

    /// Helper: build a minimal UsageRecord with the given org_id, key_id, cost, and
    /// created_at. Other token fields are zeroed; only cost matters for the
    /// budget counter logic.
    fn mk_usage(org_id: &str, key_id: &str, cost: i64, created_at: chrono::DateTime<chrono::Utc>) -> crate::types::UsageRecord {
        crate::types::UsageRecord {
            id: format!("rec-{}-{}", key_id, uuid::Uuid::new_v4()),
            org_id: org_id.to_string(),
            request_id: None,
            key_id: key_id.to_string(),
            model_name: "test-model".to_string(),
            provider_id: "test-provider".to_string(),
            channel_id: None,
            protocol: crate::types::Protocol::Openai,
            input_tokens: Some(0),
            output_tokens: Some(0),
            cache_read_tokens: None,
            cache_creation_tokens: None,
            cost,
            pricing_policy: None,
            weighted_tokens: 0,
            user_id: None,
            created_at,
        }
    }

    /// Unknown key returns 0; record_usage accumulates spend into the counter.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn budget_counters_round_trip(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-budget", "Budget Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-round-trip").await;

        // 1. Unknown key → 0.
        let initial = storage.get_month_to_date_spend(&key_id).await.expect("initial mtd");
        assert_eq!(initial, 0, "unknown key should report 0 spend");

        // 2. After a $5 usage record, MTD should be $5 (500_000_000 subunits).
        let five_usd = crate::money::usd_to_units(5.0);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, five_usd, chrono::Utc::now()))
            .await
            .expect("record_usage #1");
        let after_five = storage.get_month_to_date_spend(&key_id).await.expect("mtd after 5");
        assert_eq!(after_five, five_usd, "MTD should reflect single $5 record");

        // 3. After a second $3 record, MTD should be $8 (increment, not replace).
        let three_usd = crate::money::usd_to_units(3.0);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, three_usd, chrono::Utc::now()))
            .await
            .expect("record_usage #2");
        let after_eight = storage.get_month_to_date_spend(&key_id).await.expect("mtd after 3");
        assert_eq!(after_eight, five_usd + three_usd, "MTD should accumulate across records");
    }

    /// A record dated in a prior month must NOT count toward this month's MTD.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn budget_counters_month_bucketing(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-bucket", "Bucket Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-bucket").await;

        // Insert a record dated 40 days ago (prior calendar month, guaranteed — longest calendar month is 31 days).
        let old_cost = crate::money::usd_to_units(10.0);
        let old_ts = chrono::Utc::now() - chrono::Duration::days(40);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, old_cost, old_ts))
            .await
            .expect("record_usage old");

        // Current-month MTD should be 0.
        let mtd = storage.get_month_to_date_spend(&key_id).await.expect("current mtd");
        assert_eq!(mtd, 0, "prior-month record must not count toward current MTD");

        // Sanity: confirm the old counter row exists with a different bucket.
        let any_row: Option<(i64,)> = sqlx::query_as(
            "SELECT accrued FROM budget_counters WHERE key_id = $1 AND month_bucket <> $2",
        )
        .bind(&key_id)
        .bind(format!("{}", chrono::Utc::now().format("%Y-%m")))
        .fetch_optional(&storage.pool)
        .await
        .expect("query old counter");
        assert_eq!(any_row.map(|(v,)| v), Some(old_cost), "old-month counter row should exist with old cost");
    }

    /// 10 parallel record_usage calls must produce MTD=$10 with no lost updates.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn budget_counters_concurrent_inserts(pool: sqlx::PgPool) {
        let storage = std::sync::Arc::new(crate::postgres::PostgresStorage::from_pool(pool));
        let org = make_test_org(&storage, "org-concurrent", "Concurrent Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-concurrent").await;

        let one_usd = crate::money::usd_to_units(1.0);
        let n = 10;
        let mut handles = Vec::with_capacity(n);
        for _ in 0..n {
            let storage = storage.clone();
            let org_id = org.id.clone();
            let key_id = key_id.clone();
            handles.push(tokio::spawn(async move {
                storage
                    .record_usage(&org_id, &mk_usage(&org_id, &key_id, one_usd, chrono::Utc::now()))
                    .await
                    .expect("record_usage in task");
            }));
        }
        for h in handles {
            h.await.expect("task panicked");
        }

        let mtd = storage.get_month_to_date_spend(&key_id).await.expect("mtd after concurrent");
        assert_eq!(mtd, one_usd * n as i64, "all 10 concurrent writes must be reflected (no lost updates)");
    }

    // ---- Phase 7: org-wide MTD aggregation ----

    /// Unknown org → 0 (no keys, no counters).
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_returns_zero_for_unknown_org(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        // Brand-new org has no keys → SUM must be 0, not null, not an error.
        let org = make_test_org(&storage, "org-mtd-empty", "Empty Org").await;
        let mtd = storage
            .get_org_month_to_date_spend(&org.id)
            .await
            .expect("get_org_month_to_date_spend on empty org");
        assert_eq!(mtd, 0, "empty org must report 0 MTD");
    }

    /// 3 keys, each with $5 spend this month → org total = $15.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_sums_across_keys(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-mtd-sum", "Sum Org").await;

        let five_usd = crate::money::usd_to_units(5.0);
        for n in 0..3 {
            let key_id = make_test_key_for_budget(&storage, &org.id, &format!("key-mtd-{n}")).await;
            storage
                .record_usage(
                    &org.id,
                    &mk_usage(&org.id, &key_id, five_usd, chrono::Utc::now()),
                )
                .await
                .expect("record_usage");
        }

        let mtd = storage
            .get_org_month_to_date_spend(&org.id)
            .await
            .expect("get_org_month_to_date_spend");
        assert_eq!(mtd, five_usd * 3, "MTD must sum across all keys in the org");
    }

    /// A record dated 40 days ago lands in a prior month bucket and must NOT count.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_excludes_other_months(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-mtd-prior", "Prior Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-prior").await;

        let old_ts = chrono::Utc::now() - chrono::Duration::days(40);
        let old_cost = crate::money::usd_to_units(10.0);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, old_cost, old_ts))
            .await
            .expect("record_usage old");

        let mtd = storage
            .get_org_month_to_date_spend(&org.id)
            .await
            .expect("get_org_month_to_date_spend");
        assert_eq!(mtd, 0, "prior-month spend must not count toward current MTD");
    }

    /// Key in org A's spend must NOT bleed into org B's MTD.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_no_cross_org_leak(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org_a = make_test_org(&storage, "org-mtd-a", "Org A").await;
        let org_b = make_test_org(&storage, "org-mtd-b", "Org B").await;

        let key_a = make_test_key_for_budget(&storage, &org_a.id, "key-a").await;
        let key_b = make_test_key_for_budget(&storage, &org_b.id, "key-b").await;

        let cost = crate::money::usd_to_units(7.0);
        storage
            .record_usage(&org_a.id, &mk_usage(&org_a.id, &key_a, cost, chrono::Utc::now()))
            .await
            .expect("record_usage a");
        storage
            .record_usage(&org_b.id, &mk_usage(&org_b.id, &key_b, cost, chrono::Utc::now()))
            .await
            .expect("record_usage b");

        let mtd_a = storage
            .get_org_month_to_date_spend(&org_a.id)
            .await
            .expect("get_org_month_to_date_spend a");
        assert_eq!(mtd_a, cost, "org A's MTD must exclude org B's spend");
    }

    /// Keys without spend this month report `mtd_units: 0`.
    /// Keys with spend report the correct accrued sum.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_keys_with_mtd_includes_per_key_spend(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-keys-mtd", "Keys Mtd Org").await;

        // key-with-cost: $4 spend this month
        let key_with_cost =
            make_test_key_for_budget(&storage, &org.id, "key-with-cost").await;
        let four_usd = crate::money::usd_to_units(4.0);
        storage
            .record_usage(
                &org.id,
                &mk_usage(&org.id, &key_with_cost, four_usd, chrono::Utc::now()),
            )
            .await
            .expect("record_usage");

        // key-no-cost: no usage records
        let key_no_cost =
            make_test_key_for_budget(&storage, &org.id, "key-no-cost").await;

        let result = storage
            .list_keys_paginated_with_mtd(&org.id, 1, 50)
            .await
            .expect("list_keys_paginated_with_mtd");

        // 2 keys total
        assert_eq!(result.total, 2, "should see both keys");

        let by_id: std::collections::HashMap<String, i64> = result
            .items
            .iter()
            .map(|x| (x.key.id.clone(), x.mtd_units))
            .collect();
        assert_eq!(
            by_id.get(&key_with_cost),
            Some(&four_usd),
            "key with $4 usage must report mtd_units = $4 in subunits"
        );
        assert_eq!(
            by_id.get(&key_no_cost),
            Some(&0),
            "key with no usage must report mtd_units = 0"
        );
    }

    /// A prior-month spend must NOT show up in the current month's MTD column.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_keys_with_mtd_excludes_other_months(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-keys-prior", "Keys Prior Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-prior-mtd").await;

        let old_ts = chrono::Utc::now() - chrono::Duration::days(40);
        let old_cost = crate::money::usd_to_units(20.0);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, old_cost, old_ts))
            .await
            .expect("record_usage old");

        let result = storage
            .list_keys_paginated_with_mtd(&org.id, 1, 50)
            .await
            .expect("list_keys_paginated_with_mtd");
        let row = result
            .items
            .iter()
            .find(|x| x.key.id == key_id)
            .expect("key must be in result");
        assert_eq!(row.mtd_units, 0, "prior-month spend must not count toward current MTD");
    }
}

#[cfg(test)]
mod phase4_tests {
    use crate::Storage;

    /// Helper: build a User literal with all 13 fields populated. Used by the
    /// phase4 round-trip tests so adding a new column later is one place to
    /// update. The 4 Phase 4 fields default sensibly for non-email flows.
    fn mk_user(id: &str, uname: &str, email: &str) -> crate::types::User {
        let now = chrono::Utc::now();
        crate::types::User {
            id: id.into(),
            username: uname.into(),
            password: "x".into(),
            platform_role: None,
            current_org_id: None,
            enabled: true,
            refresh_token: None,
            created_at: now,
            updated_at: now,
            email: Some(email.into()),
            email_verified_at: None,
            requires_email_verification: true,
            password_changed_at: now,
        }
    }

    /// A user with an email AND a verification mints → store → lookup →
    /// consume → email_verified_at flips and the row is marked consumed.
    /// Single end-to-end test for the verification lifecycle.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn email_verification_round_trip(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let user = mk_user("u-evt", "evt", "evt@example.com");
        storage.create_user(&user).await.expect("create_user");

        let verification = storage
            .create_email_verification(
                &user.id,
                &user.email.as_deref().unwrap(),
                chrono::Utc::now() + chrono::Duration::hours(24),
            )
            .await
            .expect("mint");
        assert!(verification.consumed_at.is_none());
        assert_eq!(verification.user_id, user.id);

        let fetched = storage
            .get_email_verification_by_token(&verification.token)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(fetched.id, verification.id);
        assert_eq!(fetched.email, "evt@example.com");

        // Consume succeeds the first time.
        let consumed = storage
            .consume_email_verification(&verification.token)
            .await
            .expect("consume");
        assert!(consumed, "first consume should succeed");
        // The user's email_verified_at is now set, requires_email_verification = false.
        let after = storage
            .get_user(&user.id)
            .await
            .expect("get_user")
            .expect("user present");
        assert!(after.email_verified_at.is_some());
        assert!(!after.requires_email_verification);

        // Second consume is a no-op (returns false; row already consumed).
        let second = storage
            .consume_email_verification(&verification.token)
            .await
            .expect("consume again");
        assert!(!second, "second consume must return false");
    }

    /// Atomic password reset round trip: mint → Success outcome writes the
    /// new hash + bumps `password_changed_at`; a second consume is `Consumed`.
    /// Exercises `consume_password_reset_and_set_password` end-to-end so the
    /// legacy non-atomic path stays out of the test loop.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn password_reset_round_trip(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let user = mk_user("u-prt", "prt", "prt@example.com");
        storage.create_user(&user).await.expect("create_user");

        let reset = storage
            .create_password_reset(&user.id, chrono::Utc::now() + chrono::Duration::hours(1))
            .await
            .expect("mint");
        assert!(reset.consumed_at.is_none());

        let fetched = storage
            .get_password_reset_by_token(&reset.token)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(fetched.id, reset.id);

        let before = storage
            .get_user(&user.id)
            .await
            .expect("get")
            .expect("user")
            .password_changed_at;

        let outcome = storage
            .consume_password_reset_and_set_password(&reset.token, "new-hash")
            .await
            .expect("consume");
        assert_eq!(
            matches!(outcome, crate::types::PasswordResetOutcome::Success),
            true,
            "first consume should be Success (got {outcome:?})"
        );

        let after = storage
            .get_user(&user.id)
            .await
            .expect("get")
            .expect("user");
        assert_eq!(
            after.password, "new-hash",
            "consume must write the new password hash atomically"
        );
        assert!(
            after.password_changed_at > before,
            "password_changed_at must advance after consume (before={before}, after={})",
            after.password_changed_at
        );

        let second = storage
            .consume_password_reset_and_set_password(&reset.token, "ignored")
            .await
            .expect("consume again");
        assert!(
            matches!(second, crate::types::PasswordResetOutcome::Consumed),
            "second consume must be Consumed (got {second:?})"
        );
    }

    /// Case-insensitive uniqueness on users.email (the migration defines
    /// the partial unique index as `LOWER(email)`). Inserting two rows
    /// whose emails differ only by case must fail the second insert.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn email_unique_index(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        storage
            .create_user(&mk_user("u3", "alpha", "dup@example.com"))
            .await
            .expect("first insert");
        let err = storage
            .create_user(&mk_user("u4", "beta", "DUP@example.com"))
            .await;
        assert!(
            err.is_err(),
            "expected unique violation on case-insensitive duplicate email, got Ok"
        );
    }
}