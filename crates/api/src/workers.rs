use llm_gateway_audit::AuditLogger;
use llm_gateway_storage::{Protocol, UsageRecord};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::AuditTask;

/// Parse usage from response bytes.
/// For streaming (SSE): extract from last JSON chunk before "data: [DONE]"
/// For non-streaming (JSON): extract from "usage" field
fn parse_usage(bytes: &[u8], stream: bool, proto: Protocol) -> (Option<i64>, Option<i64>) {
    if stream {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None),
        };
        let mut last_usage: Option<&str> = None;
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let json_str = &line["data: ".len()..];
            if json_str == "[DONE]" {
                break;
            }
            // Check if this line has "usage"
            if json_str.contains("\"usage\"") {
                last_usage = Some(json_str);
            }
        }
        if let Some(json_str) = last_usage {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                let usage = v.get("usage");
                let (input, output) = match proto {
                    Protocol::Openai => (
                        usage.and_then(|u| u.get("prompt_tokens").and_then(|t| t.as_i64())),
                        usage.and_then(|u| u.get("completion_tokens").and_then(|t| t.as_i64())),
                    ),
                    Protocol::Anthropic => (
                        usage.and_then(|u| u.get("input_tokens").and_then(|t| t.as_i64())),
                        usage.and_then(|u| u.get("output_tokens").and_then(|t| t.as_i64())),
                    ),
                };
                return (input, output);
            }
        }
        (None, None)
    } else {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None),
        };
        let v: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return (None, None),
        };
        let usage = match v.get("usage") {
            Some(u) => u,
            None => return (None, None),
        };
        match proto {
            Protocol::Openai => (
                usage.get("prompt_tokens").and_then(|t| t.as_i64()),
                usage.get("completion_tokens").and_then(|t| t.as_i64()),
            ),
            Protocol::Anthropic => (
                usage.get("input_tokens").and_then(|t| t.as_i64()),
                usage.get("output_tokens").and_then(|t| t.as_i64()),
            ),
        }
    }
}

/// Background worker: receives audit tasks, parses usage, calculates cost, writes DB
pub async fn start_audit_worker(storage: Arc<dyn llm_gateway_storage::Storage>, mut rx: mpsc::Receiver<AuditTask>) {
    tracing::info!("[AUDIT-WORKER] Starting audit worker");
    let audit_logger = Arc::new(AuditLogger::new(storage.clone()));

    while let Some(task) = rx.recv().await {
        tracing::info!(
            "[AUDIT-WORKER] Task received: key_id={}, model={}, stream={}, response_bytes_len={}",
            task.key_id, task.model_name, task.stream, task.response_bytes.len()
        );

        // Parse usage from response bytes
        let proto = task.protocol.clone();
        let (input_tokens, output_tokens) = parse_usage(&task.response_bytes, task.stream, proto);

        // Calculate cost
        let cost = llm_gateway_billing::calculate_cost(
            &match task.billing_type.as_deref() {
                Some("request") => llm_gateway_storage::BillingType::Request,
                _ => llm_gateway_storage::BillingType::Token,
            },
            input_tokens,
            output_tokens,
            task.input_price,
            task.output_price,
            task.request_price,
        ).cost;

        tracing::info!(
            "[AUDIT-WORKER] Parsed: input={:?}, output={:?}, cost={}",
            input_tokens, output_tokens, cost
        );

        // Write usage record
        let record = UsageRecord {
            id: uuid::Uuid::new_v4().to_string(),
            key_id: task.key_id.clone(),
            model_name: task.model_name.clone(),
            provider_id: task.provider_id.clone(),
            channel_id: task.channel_id.clone(),
            protocol: task.protocol.clone(),
            input_tokens,
            output_tokens,
            cost,
            created_at: chrono::Utc::now(),
        };
        if let Err(e) = storage.record_usage(&record).await {
            tracing::error!("[AUDIT-WORKER] Failed to record usage: {}", e);
        }

        // Write audit log (only if not streaming, since full SSE response can be large)
        let response_body = if task.stream {
            if task.response_bytes.len() < 100_000 {
                String::from_utf8_lossy(&task.response_bytes).to_string()
            } else {
                "[streaming response truncated]".to_string()
            }
        } else {
            String::from_utf8_lossy(&task.response_bytes).to_string()
        };

        if let Err(e) = audit_logger.log_request(
            &task.key_id,
            &task.model_name,
            &task.provider_id,
            task.protocol,
            task.stream,
            &task.request_body,
            &response_body,
            task.status_code,
            task.latency_ms,
            input_tokens,
            output_tokens,
            task.original_model.as_deref(),
            task.upstream_model.as_deref(),
            task.model_override_reason.as_deref(),
            task.request_path.as_deref(),
            task.upstream_url.as_deref(),
        ).await {
            tracing::error!("[AUDIT-WORKER] Failed to log audit request: {}", e);
        }

        tracing::info!("[AUDIT-WORKER] Task completed: key_id={}, model={}", task.key_id, task.model_name);
    }
    tracing::info!("[AUDIT-WORKER] Worker exiting, channel closed");
}
