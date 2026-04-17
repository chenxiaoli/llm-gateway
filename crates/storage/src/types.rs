use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- Pagination ---

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub page: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub page_size: Option<i64>,
}

fn deserialize_i64_opt<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        None => Ok(None),
        Some(v) => v.parse::<i64>().map(Some).map_err(serde::de::Error::custom),
    }
}

impl PaginationParams {
    pub fn normalized(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        (page, page_size)
    }
}

// --- API Keys ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub rate_limit: Option<i64>,       // global RPM, None = unlimited
    pub budget_monthly: Option<f64>,   // monthly budget cap, None = unlimited
    pub enabled: bool,
    pub created_by: Option<String>,
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
    pub slug: String,
    pub base_url: Option<String>,           // single fallback URL
    pub endpoints: Option<String>,      // JSON {"openai": "...", "anthropic": "..."}
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProvider {
    pub name: String,
    pub slug: Option<String>,
    pub base_url: Option<String>,
    pub endpoints: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub base_url: Option<Option<String>>,
    pub endpoints: Option<Option<String>>,
    pub enabled: Option<bool>,
}

// --- Channels ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,  // NEW
    pub markup_ratio: f64,                  // NEW, default 1.0
    pub rpm_limit: Option<i64>,     // NEW: requests per minute
    pub tpm_limit: Option<i64>,     // NEW: tokens per minute
    pub balance: Option<f64>,       // NEW: remaining quota in USD
    pub weight: Option<i32>,        // NEW: routing weight (default 100)
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<String>,  // NEW
    pub markup_ratio: Option<f64>,          // NEW
    pub rpm_limit: Option<i64>,    // NEW
    pub tpm_limit: Option<i64>,    // NEW
    pub balance: Option<f64>,      // NEW
    pub weight: Option<i32>,       // NEW
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<Option<String>>,
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<Option<String>>,  // NEW
    pub markup_ratio: Option<f64>,                  // NEW
    pub enabled: Option<bool>,
    pub rpm_limit: Option<Option<i64>>,  // NEW: None=keep, Some(None)=clear
    pub tpm_limit: Option<Option<i64>>,  // NEW
    pub balance: Option<Option<f64>>,    // NEW
    pub weight: Option<Option<i32>>,     // NEW
}

// --- Models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,           // primary key
    pub name: String,          // display name
    pub provider_id: String,
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,  // nullable FK to pricing_policies
    pub billing_type: String,
    pub input_price: f64,     // per 1M tokens
    pub output_price: f64,    // per 1M tokens
    pub request_price: f64,   // per request
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

// Deprecated: kept for migration compatibility only
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BillingType {
    Token,
    Request,
}

// --- Pricing Policies ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingPolicy {
    pub id: String,
    pub name: String,
    pub billing_type: String,        // "per_token", "per_request", "per_character", "tiered_token", "hybrid"
    pub config: serde_json::Value,   // billing-type-specific config
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePricingPolicy {
    pub name: String,
    pub billing_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePricingPolicy {
    pub name: Option<String>,
    pub billing_type: Option<String>,
    pub config: Option<serde_json::Value>,
}

// --- Usage for pricing calculation ---

#[derive(Debug, Clone, Copy)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub input_chars: Option<i64>,
    pub output_chars: Option<i64>,
    pub request_count: i64,
}

impl Usage {
    pub fn from_tokens(input: Option<i64>, output: Option<i64>, requests: i64) -> Self {
        Usage {
            input_tokens: input.unwrap_or(0),
            output_tokens: output.unwrap_or(0),
            input_chars: None,
            output_chars: None,
            request_count: requests,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub pricing_policy_id: Option<String>,  // NEW
    pub billing_type: Option<String>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub pricing_policy_id: Option<Option<String>>,  // None=keep, Some(None)=clear
    pub billing_type: Option<Option<String>>,  // None=keep, Some(None)=clear
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

// --- Channel Models (Junction Table) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,   // NEW: for upstream cost
    pub markup_ratio: f64,                  // NEW, default 1.0
    pub billing_type: Option<String>,       // NEW: billing_type
    pub input_price: Option<f64>,            // NEW: input_price
    pub output_price: Option<f64>,          // NEW: output_price
    pub request_price: Option<f64>,         // NEW: request_price
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelModel {
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,   // NEW
    pub markup_ratio: Option<f64>,         // NEW
    pub billing_type: Option<String>,       // NEW: billing_type
    pub input_price: Option<f64>,          // NEW: input_price
    pub output_price: Option<f64>,         // NEW: output_price
    pub request_price: Option<f64>,        // NEW: request_price
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelModel {
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<Option<i32>>,  // None=keep, Some(None)=clear
    pub cost_policy_id: Option<Option<String>>,  // NEW
    pub markup_ratio: Option<f64>,                // NEW
    pub billing_type: Option<Option<String>>,     // NEW: None=keep, Some(None)=clear
    pub input_price: Option<f64>,                // NEW
    pub output_price: Option<f64>,               // NEW
    pub request_price: Option<f64>,              // NEW
    pub enabled: Option<bool>,
}

// --- Key-Model Rate Limits ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyModelRateLimit {
    pub key_id: String,
    pub model_id: String,
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
    pub channel_id: Option<String>,
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

fn deserialize_datetime_opt<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        None => Ok(None),
        Some(v) => DateTime::parse_from_rfc3339(&v)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Deserialize)]
pub struct UsageFilter {
    pub key_id: Option<String>,
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub since: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub until: Option<DateTime<Utc>>,
}

// --- Audit Logs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: Protocol,
    pub stream: bool,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub key_id: Option<String>,
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub since: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub until: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub offset: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
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
    pub refresh_token: Option<String>,
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
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub rate_limit: RateLimitConfig,
    pub upstream: UpstreamConfig,
    pub audit: AuditConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub encryption_key: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub allow_registration: Option<bool>,
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
