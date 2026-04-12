use llm_gateway_storage::{AuditLog, Protocol, Storage};
use std::sync::Arc;

pub struct AuditLogger {
    storage: Arc<dyn Storage>,
}

impl AuditLogger {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub async fn log_request(
        &self,
        key_id: &str,
        model_name: &str,
        provider_id: &str,
        protocol: Protocol,
        request_body: &str,
        response_body: &str,
        status_code: i32,
        latency_ms: i64,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let log = AuditLog {
            id: uuid::Uuid::new_v4().to_string(),
            key_id: key_id.to_string(),
            model_name: model_name.to_string(),
            provider_id: provider_id.to_string(),
            protocol,
            request_body: request_body.to_string(),
            response_body: response_body.to_string(),
            status_code,
            latency_ms,
            input_tokens,
            output_tokens,
            created_at: chrono::Utc::now(),
        };
        self.storage.insert_log(&log).await
    }
}
