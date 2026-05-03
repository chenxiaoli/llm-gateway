use futures::StreamExt;
use llm_gateway_audit::AuditLogger;
use llm_gateway_nats_publisher::AckKind;
use llm_gateway_storage::{postgres::PostgresStorage, AppConfig, Protocol, Storage};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let config_str = std::fs::read_to_string("config.toml")?;
    let config_str = shellexpand::env(&config_str)?.to_string();
    let config: AppConfig = toml::from_str(&config_str)?;

    // Connect to PostgreSQL
    let url = config.database.url.as_deref().ok_or("database.url is required")?;
    tracing::info!("Connecting to PostgreSQL: {}", url.split('@').last().unwrap_or("***"));
    let storage: Arc<dyn Storage> = {
        let db = PostgresStorage::new(url).await?;
        db.run_migrations().await?;
        Arc::new(db)
    };

    // Connect to NATS
    let nats_cfg = config.nats.as_ref().ok_or("[nats] section is required")?;
    let nats = Arc::new(llm_gateway_nats_publisher::NatsPublisher::new(&nats_cfg.url).await?);
    tracing::info!("Connected to NATS: {}", nats_cfg.url);

    // Run with supervisor
    let audit_logger = AuditLogger::new(storage);
    loop {
        run_audit_worker(&audit_logger, nats.clone()).await;
        tracing::warn!("[AUDIT-WORKER] exited, restarting in 5s");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn run_audit_worker(
    audit_logger: &AuditLogger,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
) {
    tracing::info!("[AUDIT-WORKER] Starting");

    let consumer = match nats.create_audit_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[AUDIT-WORKER] Failed to create consumer: {}", e);
            return;
        }
    };

    let mut messages = match consumer.messages().await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("[AUDIT-WORKER] Failed to subscribe: {}", e);
            return;
        }
    };

    while let Some(Ok(msg)) = messages.next().await {
        let event: llm_gateway_nats_publisher::AuditEvent = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("[AUDIT-WORKER] Failed to deserialize: {}", e);
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
            tracing::warn!("[AUDIT-WORKER] Failed to log audit: {}", e);
            let _ = msg.ack_with(AckKind::Nak(None)).await;
            continue;
        }

        let _ = msg.ack().await;
    }

    tracing::info!("[AUDIT-WORKER] Exiting");
}
