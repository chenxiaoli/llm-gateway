use llm_gateway_audit::AuditLogger;
use llm_gateway_billing::PricingCalculator;
use llm_gateway_storage::{PricingPolicy, Protocol, Usage, UsageRecord};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::AuditTask;

/// Parse usage from response bytes.
/// Returns (input_tokens, output_tokens, cache_read_tokens).
/// For streaming (SSE): extract from last JSON chunk before "data: [DONE]"
/// For non-streaming (JSON): extract from "usage" field.
fn parse_usage(bytes: &[u8], stream: bool, proto: Protocol) -> (Option<i64>, Option<i64>, Option<i64>) {
    let parse_value = |usage: Option<&serde_json::Value>| -> (Option<i64>, Option<i64>, Option<i64>) {
        match proto {
            Protocol::Openai => {
                let input = usage.and_then(|u| u.get("prompt_tokens").and_then(|t| t.as_i64()));
                let output = usage.and_then(|u| u.get("completion_tokens").and_then(|t| t.as_i64()));
                // OpenAI o1 series: usage.prompt_tokens_details.cache_read_tokens
                let cache_read = usage
                    .and_then(|u| u.get("prompt_tokens_details"))
                    .and_then(|d| d.get("cache_read_tokens"))
                    .and_then(|t| t.as_i64());
                (input, output, cache_read)
            }
            Protocol::Anthropic => {
                let input = usage.and_then(|u| u.get("input_tokens").and_then(|t| t.as_i64()));
                let output = usage.and_then(|u| u.get("output_tokens").and_then(|t| t.as_i64()));
                // Anthropic: usage.cache_read_input_tokens
                let cache_read = usage
                    .and_then(|u| u.get("cache_read_input_tokens"))
                    .and_then(|t| t.as_i64());
                (input, output, cache_read)
            }
        }
    };

    if stream {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None, None),
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
            if json_str.contains("\"usage\"") {
                last_usage = Some(json_str);
            }
        }
        if let Some(json_str) = last_usage {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                return parse_value(v.get("usage"));
            }
        }
        (None, None, None)
    } else {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None, None),
        };
        let v: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return (None, None, None),
        };
        parse_value(v.get("usage"))
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
        let (input_tokens, output_tokens, cache_read_tokens) =
            parse_usage(&task.response_bytes, task.stream, proto);

        // Calculate cost using PricingCalculator with policy config
        let cost = if let Some(config) = &task.pricing_policy_config {
            let policy = PricingPolicy {
                id: String::new(),
                name: String::new(),
                billing_type: task.pricing_policy_billing_type.clone(),
                config: config.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            let usage = Usage {
                input_tokens: input_tokens.unwrap_or(0),
                output_tokens: output_tokens.unwrap_or(0),
                input_chars: None,
                output_chars: None,
                request_count: 1,
                cache_read_tokens,
            };
            let raw_cost = PricingCalculator.calculate_cost(&policy, &usage);
            raw_cost * task.markup_ratio
        } else {
            0.0
        };

        tracing::info!(
            "[AUDIT-WORKER] Parsed: input={:?}, output={:?}, cache_read={:?}, cost={}",
            input_tokens, output_tokens, cache_read_tokens, cost
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
            cache_read_tokens,
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
            task.request_headers.as_deref(),
            task.response_headers.as_deref(),
        ).await {
            tracing::error!("[AUDIT-WORKER] Failed to log audit request: {}", e);
        }

        tracing::info!("[AUDIT-WORKER] Task completed: key_id={}, model={}", task.key_id, task.model_name);
    }
    tracing::info!("[AUDIT-WORKER] Worker exiting, channel closed");
}
