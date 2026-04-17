use llm_gateway_storage::{AuditLog, Protocol, Storage};
use std::sync::Arc;

pub struct AuditLogger {
    storage: Arc<dyn Storage>,
}

pub struct SettingSnapshot {
    pub audit_log_request: bool,
    pub audit_log_response: bool,
}

impl AuditLogger {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub async fn get_settings(&self) -> SettingSnapshot {
        let audit_req = self.storage.get_setting("audit_log_request").await.ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true);
        let audit_res = self.storage.get_setting("audit_log_response").await.ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true);

        SettingSnapshot {
            audit_log_request: audit_req,
            audit_log_response: audit_res,
        }
    }

    pub async fn log_request(
        &self,
        key_id: &str,
        model_name: &str,
        provider_id: &str,
        protocol: Protocol,
        stream: bool,
        request_body: &str,
        response_body: &str,
        status_code: i32,
        latency_ms: i64,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let settings = self.get_settings().await;
        let request_body = if settings.audit_log_request {
            request_body
        } else {
            "{}"
        };
        let response_body = if settings.audit_log_response {
            response_body
        } else {
            "{}"
        };
        let log = AuditLog {
            id: uuid::Uuid::new_v4().to_string(),
            key_id: key_id.to_string(),
            model_name: model_name.to_string(),
            provider_id: provider_id.to_string(),
            channel_id: None,
            protocol,
            stream,
            request_body: request_body.to_string(),
            response_body: response_body.to_string(),
            status_code,
            latency_ms,
            input_tokens,
            output_tokens,
            created_at: chrono::Utc::now(),
            original_model: None,
            upstream_model: None,
            model_override_reason: None,
        };
        self.storage.insert_log(&log).await
    }
}
