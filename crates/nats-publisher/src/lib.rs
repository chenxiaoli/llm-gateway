use async_nats::jetstream::{self, consumer::PushConsumer, Context};
use serde::{Deserialize, Serialize};

// Re-export AckKind so consumers don't need a direct async-nats dependency.
pub use async_nats::jetstream::AckKind;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: String,
    pub request_id: String,
    pub key_id: String,
    pub user_id: Option<String>,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cost: i64,
    pub pricing_policy: Option<serde_json::Value>,
    pub weighted_tokens: i64,
    pub latency_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub request_id: String,
    pub key_id: String,
    pub user_id: Option<String>,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: String,
    pub stream: bool,
    pub status_code: i32,
    pub latency_ms: i64,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: Option<String>,
    pub upstream_url: Option<String>,
    pub request_body: String,
    pub response_body: String,
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
    pub created_at: String,
    /// Per-upstream-attempt history. None for legacy events or for
    /// code paths that don't collect route attempts. The `#[serde(default)]`
    /// attribute is critical: it lets the new gateway emit new events
    /// while an old worker is still running, and vice versa.
    #[serde(default)]
    pub routes: Option<Vec<llm_gateway_storage::RouteAttempt>>,
}

// ---------------------------------------------------------------------------
// NATS publisher
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Stream status info
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct StreamStatusInfo {
    pub name: String,
    pub messages: u64,
    pub bytes: u64,
    pub consumer_count: usize,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub max_messages: i64,
    pub max_age_secs: u64,
    pub pending_messages: u64,
}

const SEVEN_DAYS: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);
const THIRTY_DAYS: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 3600);

#[derive(Debug, Clone)]
pub struct NatsPublisher {
    js: Context,
}

impl NatsPublisher {
    /// Connect to the NATS server at `url` and create the required JetStream
    /// streams idempotently.
    pub async fn new(url: &str, token: Option<String>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = if let Some(token) = token {
            async_nats::connect_with_options(
                url,
                async_nats::ConnectOptions::new().token(token),
            ).await?
        } else {
            async_nats::connect(url).await?
        };
        let js = jetstream::new(client);

        // Create LLM_GATEWAY_USAGE stream (1 M messages, 7-day retention).
        js.create_stream(jetstream::stream::Config {
            name: "LLM_GATEWAY_USAGE".into(),
            subjects: vec!["gateway.usage".into()],
            max_messages: 1_000_000,
            retention: jetstream::stream::RetentionPolicy::Limits,
            discard: jetstream::stream::DiscardPolicy::Old,
            max_age: SEVEN_DAYS,
            ..Default::default()
        })
        .await
        .ok(); // idempotent -- ignore already-exists

        // Create LLM_GATEWAY_AUDIT stream (5 M messages, 30-day retention).
        js.create_stream(jetstream::stream::Config {
            name: "LLM_GATEWAY_AUDIT".into(),
            subjects: vec!["gateway.audit".into()],
            max_messages: 5_000_000,
            retention: jetstream::stream::RetentionPolicy::Limits,
            discard: jetstream::stream::DiscardPolicy::Old,
            max_age: THIRTY_DAYS,
            ..Default::default()
        })
        .await
        .ok(); // idempotent -- ignore already-exists

        Ok(Self { js })
    }

    /// Publish a usage event to the `gateway.usage` subject.
    pub async fn publish_usage(
        &self,
        event: &UsageEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::to_vec(event)?;
        self.js
            .publish("gateway.usage".to_string(), payload.into())
            .await?
            .await?;
        Ok(())
    }

    /// Publish an audit event to the `gateway.audit` subject.
    pub async fn publish_audit(
        &self,
        event: &AuditEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::to_vec(event)?;
        self.js
            .publish("gateway.audit".to_string(), payload.into())
            .await?
            .await?;
        Ok(())
    }

    /// Create a durable push consumer for the GATEWAY_USAGE stream.
    pub async fn create_usage_consumer(
        &self,
    ) -> Result<PushConsumer, Box<dyn std::error::Error + Send + Sync>> {
        let consumer = self
            .js
            .create_consumer_on_stream(
                jetstream::consumer::push::Config {
                    durable_name: Some("usage-worker".to_string()),
                    deliver_subject: "usage-worker-delivery".to_string(),
                    ..Default::default()
                },
                "LLM_GATEWAY_USAGE",
            )
            .await?;
        Ok(consumer)
    }

    /// Create a durable push consumer for the GATEWAY_AUDIT stream.
    pub async fn create_audit_consumer(
        &self,
    ) -> Result<PushConsumer, Box<dyn std::error::Error + Send + Sync>> {
        let consumer = self
            .js
            .create_consumer_on_stream(
                jetstream::consumer::push::Config {
                    durable_name: Some("audit-worker".to_string()),
                    deliver_subject: "audit-worker-delivery".to_string(),
                    ..Default::default()
                },
                "LLM_GATEWAY_AUDIT",
            )
            .await?;
        Ok(consumer)
    }

    /// Return a reference to the underlying JetStream context.
    pub fn js_context(&self) -> &Context {
        &self.js
    }

    /// Fetch status information for a named JetStream stream.
    pub async fn stream_info(&self, name: &str) -> Result<StreamStatusInfo, String> {
        let mut stream = self.js.get_stream(name).await.map_err(|e| e.to_string())?;
        let info = stream.info().await.map_err(|e| e.to_string())?;

        // Extract owned values so the mutable borrow on `info` is dropped.
        let stream_name = info.config.name.clone();
        let messages = info.state.messages;
        let bytes = info.state.bytes;
        let consumer_count = info.state.consumer_count;
        let first_sequence = info.state.first_sequence;
        let last_sequence = info.state.last_sequence;
        let max_messages = info.config.max_messages;
        let max_age_secs = info.config.max_age.as_secs();
        let _ = info;

        let mut pending_messages: u64 = 0;
        let consumer_names: Vec<String> = futures::TryStreamExt::try_collect(stream.consumer_names()).await.map_err(|e| e.to_string())?;
        if consumer_names.is_empty() {
            // No consumers exist → all stream messages are unconsumed
            pending_messages = messages;
        } else {
            for cname in &consumer_names {
                if let Ok(ci) = stream.consumer_info(cname).await {
                    pending_messages += ci.num_pending;
                }
            }
        }

        Ok(StreamStatusInfo {
            name: stream_name,
            messages,
            bytes,
            consumer_count,
            first_sequence,
            last_sequence,
            max_messages,
            max_age_secs,
            pending_messages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_event_routes_serde_default_is_none() {
        // Old events without the `routes` field should deserialize with
        // routes = None (this is the whole point of #[serde(default)]).
        let old_event_json = r#"{
            "id": "x", "request_id": "r", "key_id": "k", "model_name": "m",
            "provider_id": "p", "protocol": "openai", "stream": false,
            "status_code": 200, "latency_ms": 100, "request_body": "{}",
            "response_body": "{}", "created_at": "2026-07-01T00:00:00Z"
        }"#;
        let event: AuditEvent = serde_json::from_str(old_event_json)
            .expect("old event should still deserialize");
        assert!(event.routes.is_none());
    }

    #[test]
    fn audit_event_routes_serde_round_trip() {
        let event = AuditEvent {
            id: "x".into(),
            request_id: "r".into(),
            key_id: "k".into(),
            user_id: None,
            model_name: "m".into(),
            provider_id: "p".into(),
            channel_id: None,
            protocol: "openai".into(),
            stream: false,
            status_code: 200,
            latency_ms: 100,
            original_model: None,
            upstream_model: None,
            model_override_reason: None,
            request_path: None,
            upstream_url: None,
            request_body: "{}".into(),
            response_body: "{}".into(),
            request_headers: None,
            response_headers: None,
            created_at: "2026-07-01T00:00:00Z".into(),
            routes: Some(vec![llm_gateway_storage::RouteAttempt {
                model: "m".into(),
                channel_id: "c".into(),
                channel_name: None,
                status_code: 200,
                error_message: None,
                latency_ms: 100,
                started_at: chrono::Utc::now(),
            }]),
        };
        let s = serde_json::to_string(&event).expect("serialize");
        let back: AuditEvent = serde_json::from_str(&s).expect("deserialize");
        match back.routes.as_deref() {
            Some([route]) => {
                assert_eq!(route.model, "m", "model should round-trip");
                assert_eq!(route.channel_id, "c", "channel_id should round-trip");
                assert_eq!(route.status_code, 200, "status_code should round-trip");
                assert_eq!(route.latency_ms, 100, "latency_ms should round-trip");
                assert!(
                    route.error_message.is_none(),
                    "error_message should remain None"
                );
            }
            Some(routes) => panic!("expected exactly one route, got {}", routes.len()),
            None => panic!("routes should be Some after round-trip"),
        }
    }
}
