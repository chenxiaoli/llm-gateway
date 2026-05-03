use futures::StreamExt;
use llm_gateway_audit::AuditLogger;
use llm_gateway_storage::{Protocol, UsageRecord};
use std::sync::Arc;

/// Parse usage from response bytes.
/// Returns (input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens).
/// For streaming (SSE): extract from last JSON chunk before "data: [DONE]"
/// For non-streaming (JSON): extract from "usage" field.
pub fn parse_usage(bytes: &[u8], stream: bool, proto: Protocol) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    let parse_value = |usage: Option<&serde_json::Value>| -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
        match proto {
            Protocol::Openai => {
                let prompt_tokens = usage.and_then(|u| u.get("prompt_tokens")).and_then(|t| t.as_i64());
                let output = usage.and_then(|u| u.get("completion_tokens")).and_then(|t| t.as_i64());
                let cache_read = usage
                    .and_then(|u| u.get("prompt_tokens_details"))
                    .and_then(|d| d.get("cache_read_tokens"))
                    .and_then(|t| t.as_i64());
                let cache_creation = usage
                    .and_then(|u| u.get("prompt_tokens_details"))
                    .and_then(|d| d.get("cached_tokens"))
                    .and_then(|t| t.as_i64());
                // OpenAI prompt_tokens includes cache; store non-cache input only
                let input = prompt_tokens.unwrap_or(0) - cache_read.unwrap_or(0) - cache_creation.unwrap_or(0);
                (Some(input), output, cache_read, cache_creation)
            }
            Protocol::Anthropic => {
                // Anthropic input_tokens already excludes cache; store as-is
                let input = usage.and_then(|u| u.get("input_tokens")).and_then(|t| t.as_i64());
                let output = usage.and_then(|u| u.get("output_tokens")).and_then(|t| t.as_i64());
                let cache_read = usage
                    .and_then(|u| u.get("cache_read_input_tokens"))
                    .and_then(|t| t.as_i64());
                let cache_creation = usage
                    .and_then(|u| u.get("cache_creation_input_tokens"))
                    .and_then(|t| t.as_i64());
                (input, output, cache_read, cache_creation)
            }
        }
    };

    if stream {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None, None, None),
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
        (None, None, None, None)
    } else {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None, None, None),
        };
        let v: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return (None, None, None, None),
        };
        parse_value(v.get("usage"))
    }
}

/// Calculate cost from token usage and pricing policy.
pub fn calculate_cost(
    pricing_policy_config: &Option<serde_json::Value>,
    pricing_policy_billing_type: &str,
    markup_ratio: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
) -> i64 {
    use llm_gateway_billing::PricingCalculator;
    use llm_gateway_storage::{PricingPolicy, Usage};

    if let Some(config) = pricing_policy_config {
        let policy = PricingPolicy {
            id: String::new(),
            name: String::new(),
            billing_type: pricing_policy_billing_type.to_string(),
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
            cache_creation_tokens,
        };
        let raw_cost = PricingCalculator.calculate_cost(&policy, &usage);
        raw_cost * markup_ratio / 10_000
    } else {
        0
    }
}

/// NATS consumer worker: reads usage events from JetStream and writes to DB.
pub async fn start_nats_usage_worker(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
) {
    tracing::info!("[NATS-USAGE-WORKER] Starting");

    let consumer = match nats.create_usage_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[NATS-USAGE-WORKER] Failed to create consumer: {}", e);
            return;
        }
    };

    let mut messages = consumer.messages().await.unwrap_or_else(|e| {
        tracing::error!("[NATS-USAGE-WORKER] Failed to subscribe: {}", e);
        panic!("NATS usage consumer subscribe failed");
    });

    while let Some(Ok(msg)) = messages.next().await {
        let event: llm_gateway_nats_publisher::UsageEvent = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("[NATS-USAGE-WORKER] Failed to deserialize: {}", e);
                let _ = msg.ack().await;
                continue;
            }
        };

        let record = UsageRecord {
            id: event.id,
            key_id: event.key_id,
            user_id: event.user_id,
            model_name: event.model_name,
            provider_id: event.provider_id,
            channel_id: event.channel_id,
            protocol: match event.protocol.as_str() {
                "anthropic" => Protocol::Anthropic,
                _ => Protocol::Openai,
            },
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
            cache_read_tokens: event.cache_read_tokens,
            cache_creation_tokens: event.cache_creation_tokens,
            cost: event.cost,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.created_at)
                .map(|dt| dt.to_utc())
                .unwrap_or_else(|_| chrono::Utc::now()),
        };

        if let Err(e) = storage.record_usage(&record).await {
            tracing::warn!("[NATS-USAGE-WORKER] Failed to record usage: {}", e);
            let _ = msg.ack_with(llm_gateway_nats_publisher::AckKind::Nak(None)).await;
            continue;
        }

        let _ = msg.ack().await;
    }

    tracing::info!("[NATS-USAGE-WORKER] Exiting");
}

/// NATS consumer worker: reads audit events from JetStream and writes to DB.
pub async fn start_nats_audit_worker(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
) {
    tracing::info!("[NATS-AUDIT-WORKER] Starting");

    let audit_logger = AuditLogger::new(storage);
    let consumer = match nats.create_audit_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[NATS-AUDIT-WORKER] Failed to create consumer: {}", e);
            return;
        }
    };

    let mut messages = consumer.messages().await.unwrap_or_else(|e| {
        tracing::error!("[NATS-AUDIT-WORKER] Failed to subscribe: {}", e);
        panic!("NATS audit consumer subscribe failed");
    });

    while let Some(Ok(msg)) = messages.next().await {
        let event: llm_gateway_nats_publisher::AuditEvent = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("[NATS-AUDIT-WORKER] Failed to deserialize: {}", e);
                let _ = msg.ack().await;
                continue;
            }
        };

        let proto = match event.protocol.as_str() {
            "anthropic" => Protocol::Anthropic,
            _ => Protocol::Openai,
        };

        if let Err(e) = audit_logger.log_request(
            &event.key_id,
            event.user_id.as_deref(),
            &event.model_name,
            &event.provider_id,
            event.channel_id.as_deref(),
            proto,
            event.stream,
            &event.request_body,
            &event.response_body,
            event.status_code,
            event.latency_ms,
            None,
            None,
            event.original_model.as_deref(),
            event.upstream_model.as_deref(),
            event.model_override_reason.as_deref(),
            event.request_path.as_deref(),
            event.upstream_url.as_deref(),
            event.request_headers.as_deref(),
            event.response_headers.as_deref(),
        ).await {
            tracing::warn!("[NATS-AUDIT-WORKER] Failed to log audit: {}", e);
            let _ = msg.ack_with(llm_gateway_nats_publisher::AckKind::Nak(None)).await;
            continue;
        }

        let _ = msg.ack().await;
    }

    tracing::info!("[NATS-AUDIT-WORKER] Exiting");
}
