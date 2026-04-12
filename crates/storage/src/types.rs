use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- API Keys ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub rate_limit: Option<i64>,       // global RPM, None = unlimited
    pub budget_monthly: Option<f64>,   // monthly budget cap, None = unlimited
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiKey {
    pub name: Option<String>,
    pub rate_limit: Option<Option<i64>>,
    pub budget_monthly: Option<Option<f64>>,
    pub enabled: Option<bool>,
}

// --- Providers ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProvider {
    pub name: String,
    pub api_key: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub openai_base_url: Option<Option<String>>,
    pub anthropic_base_url: Option<Option<String>>,
    pub enabled: Option<bool>,
}

// --- Models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub provider_id: String,
    pub billing_type: BillingType,
    pub input_price: f64,     // per 1M tokens
    pub output_price: f64,    // per 1M tokens
    pub request_price: f64,   // per request
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BillingType {
    Token,
    Request,
}

#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub billing_type: BillingType,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub billing_type: Option<BillingType>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelWithProvider {
    pub model: Model,
    pub provider_name: String,
    pub openai_compatible: bool,
    pub anthropic_compatible: bool,
}

// --- Key-Model Rate Limits ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyModelRateLimit {
    pub key_id: String,
    pub model_name: String,
    pub rpm: i64,
    pub tpm: i64,
}

// --- Usage Records ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: Protocol,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cost: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Openai,
    Anthropic,
}

#[derive(Debug, Deserialize)]
pub struct UsageFilter {
    pub key_id: Option<String>,
    pub model_name: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

// --- Audit Logs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: Protocol,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub key_id: Option<String>,
    pub model_name: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

// --- Users ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub role: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub role: Option<String>,
    pub enabled: Option<bool>,
}

// --- Settings ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub allow_registration: bool,
}

// --- Config ---

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub admin: AdminConfig,
    pub database: DatabaseConfig,
    pub rate_limit: RateLimitConfig,
    pub upstream: UpstreamConfig,
    pub audit: AuditConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct AdminConfig {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub driver: String,
    pub sqlite_path: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitConfig {
    pub flush_interval_secs: i64,
    pub window_size_secs: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamConfig {
    pub timeout_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct AuditConfig {
    pub retention_days: Option<i64>,
}
