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
    pub latency_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
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
}

// ---------------------------------------------------------------------------
// NATS publisher
// ---------------------------------------------------------------------------

const SEVEN_DAYS: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);
const THIRTY_DAYS: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 3600);

#[derive(Debug, Clone)]
pub struct NatsPublisher {
    js: Context,
}

impl NatsPublisher {
    /// Connect to the NATS server at `url` and create the required JetStream
    /// streams idempotently.
    pub async fn new(url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = async_nats::connect(url).await?;
        let js = jetstream::new(client);

        // Create GATEWAY_USAGE stream (1 M messages, 7-day retention).
        js.create_stream(jetstream::stream::Config {
            name: "GATEWAY_USAGE".into(),
            subjects: vec!["gateway.usage".into()],
            max_messages: 1_000_000,
            retention: jetstream::stream::RetentionPolicy::Limits,
            discard: jetstream::stream::DiscardPolicy::Old,
            max_age: SEVEN_DAYS,
            ..Default::default()
        })
        .await
        .ok(); // idempotent -- ignore already-exists

        // Create GATEWAY_AUDIT stream (5 M messages, 30-day retention).
        js.create_stream(jetstream::stream::Config {
            name: "GATEWAY_AUDIT".into(),
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
                "GATEWAY_USAGE",
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
                "GATEWAY_AUDIT",
            )
            .await?;
        Ok(consumer)
    }

    /// Return a reference to the underlying JetStream context.
    pub fn js_context(&self) -> &Context {
        &self.js
    }
}
