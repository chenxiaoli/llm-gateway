pub mod auth;
pub mod error;
pub mod extractors;
pub mod models;
pub mod proxy;
pub mod workers;
pub mod management;

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
}

/// Task sent to background worker for async processing.
/// Worker parses response_bytes for usage, calculates cost, and writes to DB.
pub struct AuditTask {
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: llm_gateway_storage::Protocol,
    pub stream: bool,
    pub request_body: String,
    pub response_bytes: Vec<u8>,
    pub status_code: i32,
    pub latency_ms: i64,
    // Pricing params (worker parses usage and calculates cost)
    pub billing_type: Option<String>,
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub channel_id: Option<String>,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
}
