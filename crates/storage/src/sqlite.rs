use crate::migrations::INIT_SQL;
use crate::types::*;
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
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteProviderRow {
    id: String,
    name: String,
    api_key: String,
    openai_base_url: Option<String>,
    anthropic_base_url: Option<String>,
    enabled: i64,
    created_at: String,
    updated_at: String,
}

impl From<SqliteProviderRow> for Provider {
    fn from(r: SqliteProviderRow) -> Self {
        Provider {
            id: r.id,
            name: r.name,
            api_key: r.api_key,
            openai_base_url: r.openai_base_url,
            anthropic_base_url: r.anthropic_base_url,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

#[derive(FromRow)]
struct SqliteModelRow {
    name: String,
    provider_id: String,
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
            name: r.name,
            provider_id: r.provider_id,
            billing_type: parse_billing_type(&r.billing_type),
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
    name: String,
    provider_id: String,
    billing_type: String,
    input_price: f64,
    output_price: f64,
    request_price: f64,
    enabled: i64,
    created_at: String,
    provider_name: String,
    openai_base_url: Option<String>,
    anthropic_base_url: Option<String>,
}

impl From<SqliteModelWithProviderRow> for ModelWithProvider {
    fn from(r: SqliteModelWithProviderRow) -> Self {
        ModelWithProvider {
            model: Model {
                name: r.name,
                provider_id: r.provider_id,
                billing_type: parse_billing_type(&r.billing_type),
                input_price: r.input_price,
                output_price: r.output_price,
                request_price: r.request_price,
                enabled: r.enabled != 0,
                created_at: parse_rfc3339(&r.created_at),
            },
            provider_name: r.provider_name,
            openai_compatible: r.openai_base_url.is_some(),
            anthropic_compatible: r.anthropic_base_url.is_some(),
        }
    }
}

#[derive(FromRow)]
struct SqliteUsageRow {
    id: String,
    key_id: String,
    model_name: String,
    provider_id: String,
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
    protocol: String,
    request_body: String,
    response_body: String,
    status_code: i32,
    latency_ms: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    created_at: String,
}

impl From<SqliteAuditRow> for AuditLog {
    fn from(r: SqliteAuditRow) -> Self {
        AuditLog {
            id: r.id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            protocol: parse_protocol(&r.protocol),
            request_body: r.request_body,
            response_body: r.response_body,
            status_code: r.status_code,
            latency_ms: r.latency_ms,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            created_at: parse_rfc3339(&r.created_at),
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
        for stmt in INIT_SQL.split(';') {
            let trimmed = stmt.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed).execute(&self.pool).await?;
            }
        }
        Ok(())
    }

    // ---- API Keys ----

    async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, DbErr> {
        sqlx::query(
            "INSERT INTO api_keys (id, name, key_hash, rate_limit, budget_monthly, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&key.id)
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled as i64)
        .bind(key.created_at.to_rfc3339())
        .bind(key.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(key.clone())
    }

    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_at, updated_at
             FROM api_keys WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, DbErr> {
        let row: Option<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_at, updated_at
             FROM api_keys WHERE key_hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ApiKey::from))
    }

    async fn list_keys(&self) -> Result<Vec<ApiKey>, DbErr> {
        let rows: Vec<SqliteKeyRow> = sqlx::query_as(
            "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_at, updated_at
             FROM api_keys",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ApiKey::from).collect())
    }

    async fn update_key(&self, key: &ApiKey) -> Result<ApiKey, DbErr> {
        sqlx::query(
            "UPDATE api_keys SET name = ?, key_hash = ?, rate_limit = ?, budget_monthly = ?,
             enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(key.rate_limit)
        .bind(key.budget_monthly)
        .bind(key.enabled as i64)
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
            "INSERT INTO providers (id, name, api_key, openai_base_url, anthropic_base_url, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.api_key)
        .bind(&provider.openai_base_url)
        .bind(&provider.anthropic_base_url)
        .bind(provider.enabled as i64)
        .bind(provider.created_at.to_rfc3339())
        .bind(provider.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }

    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, DbErr> {
        let row: Option<SqliteProviderRow> = sqlx::query_as(
            "SELECT id, name, api_key, openai_base_url, anthropic_base_url, enabled, created_at, updated_at
             FROM providers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Provider::from))
    }

    async fn list_providers(&self) -> Result<Vec<Provider>, DbErr> {
        let rows: Vec<SqliteProviderRow> = sqlx::query_as(
            "SELECT id, name, api_key, openai_base_url, anthropic_base_url, enabled, created_at, updated_at
             FROM providers",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Provider::from).collect())
    }

    async fn update_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "UPDATE providers SET name = ?, api_key = ?, openai_base_url = ?, anthropic_base_url = ?,
             enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&provider.name)
        .bind(&provider.api_key)
        .bind(&provider.openai_base_url)
        .bind(&provider.anthropic_base_url)
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

    // ---- Models ----

    async fn create_model(&self, model: &Model) -> Result<Model, DbErr> {
        sqlx::query(
            "INSERT INTO models (name, provider_id, billing_type, input_price, output_price, request_price, enabled, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&model.name)
        .bind(&model.provider_id)
        .bind(billing_type_str(&model.billing_type))
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
            "SELECT name, provider_id, billing_type, input_price, output_price, request_price, enabled, created_at
             FROM models WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Model::from))
    }

    async fn list_models(&self) -> Result<Vec<ModelWithProvider>, DbErr> {
        let rows: Vec<SqliteModelWithProviderRow> = sqlx::query_as(
            "SELECT m.name, m.provider_id, m.billing_type, m.input_price, m.output_price, m.request_price,
                    m.enabled, m.created_at, p.name as provider_name, p.openai_base_url, p.anthropic_base_url
             FROM models m
             JOIN providers p ON m.provider_id = p.id
             WHERE m.enabled = 1 AND p.enabled = 1",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ModelWithProvider::from).collect())
    }

    async fn update_model(&self, model: &Model) -> Result<Model, DbErr> {
        sqlx::query(
            "UPDATE models SET provider_id = ?, billing_type = ?, input_price = ?, output_price = ?,
             request_price = ?, enabled = ? WHERE name = ?",
        )
        .bind(&model.provider_id)
        .bind(billing_type_str(&model.billing_type))
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
            "INSERT INTO key_model_rate_limits (key_id, model_name, rpm, tpm)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(key_id, model_name) DO UPDATE SET rpm = ?, tpm = ?",
        )
        .bind(&limit.key_id)
        .bind(&limit.model_name)
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
        model_name: &str,
    ) -> Result<Option<KeyModelRateLimit>, DbErr> {
        let row: Option<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT key_id, model_name, rpm, tpm FROM key_model_rate_limits
             WHERE key_id = ? AND model_name = ?",
        )
        .bind(key_id)
        .bind(model_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| KeyModelRateLimit {
            key_id: r.0,
            model_name: r.1,
            rpm: r.2,
            tpm: r.3,
        }))
    }

    async fn list_key_model_rate_limits(&self, key_id: &str) -> Result<Vec<KeyModelRateLimit>, DbErr> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT key_id, model_name, rpm, tpm FROM key_model_rate_limits WHERE key_id = ?",
        )
        .bind(key_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| KeyModelRateLimit {
                key_id: r.0,
                model_name: r.1,
                rpm: r.2,
                tpm: r.3,
            })
            .collect())
    }

    async fn delete_key_model_rate_limit(&self, key_id: &str, model_name: &str) -> Result<(), DbErr> {
        sqlx::query(
            "DELETE FROM key_model_rate_limits WHERE key_id = ? AND model_name = ?",
        )
        .bind(key_id)
        .bind(model_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- Usage ----

    async fn record_usage(&self, usage: &UsageRecord) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO usage_records (id, key_id, model_name, provider_id, protocol, input_tokens, output_tokens, cost, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&usage.id)
        .bind(&usage.key_id)
        .bind(&usage.model_name)
        .bind(&usage.provider_id)
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
            "SELECT id, key_id, model_name, provider_id, protocol, input_tokens, output_tokens, cost, created_at
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

    // ---- Audit ----

    async fn insert_log(&self, log: &AuditLog) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO audit_logs (id, key_id, model_name, provider_id, protocol, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.key_id)
        .bind(&log.model_name)
        .bind(&log.provider_id)
        .bind(protocol_str(&log.protocol))
        .bind(&log.request_body)
        .bind(&log.response_body)
        .bind(log.status_code)
        .bind(log.latency_ms)
        .bind(log.input_tokens)
        .bind(log.output_tokens)
        .bind(log.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<AuditLog>, DbErr> {
        let mut sql = String::from(
            "SELECT id, key_id, model_name, provider_id, protocol, request_body, response_body,
             status_code, latency_ms, input_tokens, output_tokens, created_at
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

    // ---- Rate Limit Counters ----

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
}
