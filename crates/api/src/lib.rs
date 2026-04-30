pub mod auth;
pub mod error;
pub mod extractors;
pub mod models;
pub mod proxy;
pub mod settlement;
pub mod workers;
pub mod management;

pub use crate::proxy::{ChannelRegistry, InMemoryChannelRegistry, ResolvedChannel, spawn_registry_refresh};
pub use settlement::{start_settlement_worker, SettlementTrigger};

use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub rate_limiter: Arc<RateLimiter>,
    pub audit_logger: Arc<AuditLogger>,
    pub jwt_secret: String,
    pub encryption_key: [u8; 32],
    pub audit_tx: mpsc::Sender<AuditTask>,
    pub registry: Arc<dyn ChannelRegistry>,
    pub settlement_tx: mpsc::Sender<settlement::SettlementTrigger>,
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

/// Task sent to background worker for async processing.
/// Worker parses response_bytes for usage, calculates cost, and writes to DB.
pub struct AuditTask {
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
    // Pricing params (worker parses usage and calculates cost)
    pub pricing_policy_config: Option<serde_json::Value>,  // config from PricingPolicy
    pub pricing_policy_billing_type: String,               // billing type from PricingPolicy
    pub markup_ratio: i64,                                // markup ratio in basis points (1.0 = 10_000)
    pub channel_id: Option<String>,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: Option<String>,
    pub upstream_url: Option<String>,
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
}
