pub mod auth;
pub mod error;
pub mod extractors;
pub mod models;
pub mod user_models;
pub mod proxy;
pub mod workers;
pub mod management;
pub mod middleware;
pub mod janitor;

pub use crate::proxy::{ChannelRegistry, InMemoryChannelRegistry, ResolvedChannel, spawn_registry_refresh};
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use std::sync::Arc;
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub rate_limiter: Arc<RateLimiter>,
    pub jwt_secret: String,
    pub encryption_key: [u8; 32],
    pub nats_publisher: Option<std::sync::Arc<llm_gateway_nats_publisher::NatsPublisher>>,
    pub registry: Arc<dyn ChannelRegistry>,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemInfo {
    pub server_bind_address: String,
    pub database_driver: String,
    pub rate_limit_window_secs: i64,
    pub rate_limit_flush_interval_secs: i64,
    pub upstream_timeout_secs: u64,
    pub audit_retention_days: Option<i64>,
}

/// Intermediate task for audit/usage processing.
/// Parsed in proxy, published as UsageEvent + AuditEvent to NATS.
pub struct AuditTask {
    pub request_id: String,
    pub key_id: String,
    pub user_id: Option<String>,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: llm_gateway_storage::Protocol,
    pub stream: bool,
    pub request_body: String,
    pub response_bytes: Vec<u8>,
    pub status_code: i32,
    pub latency_ms: i64,
    pub pricing_policy: Option<llm_gateway_storage::PricingPolicy>,
    pub markup_ratio: i64,
    pub channel_id: Option<String>,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: Option<String>,
    pub upstream_url: Option<String>,
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
    /// Org scope for the audit row (Phase 1 multi-tenant). Populated from
    /// `api_key.org_id` at proxy entry. Defaults to "org_default" for
    /// legacy/synthetic tasks.
    pub org_id: String,
    /// Whether the requesting API key's owner held platform-admin
    /// privileges at request time.
    pub actor_is_platform_admin: bool,
    /// Per-upstream-attempt history collected during the failover loop.
    /// Always populated by the proxy (Vec is non-null, may be empty in
    /// pathological cases). The audit worker forwards it to the DB.
    pub routes: Vec<llm_gateway_storage::RouteAttempt>,
}
