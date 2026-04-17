pub mod auth;
pub mod error;
pub mod extractors;
pub mod openai;
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

/// Task sent to background worker for async processing
pub struct AuditTask {
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: llm_gateway_storage::Protocol,
    pub stream: bool,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
}
