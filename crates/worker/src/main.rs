use futures::StreamExt;
use llm_gateway_audit::AuditLogger;
use llm_gateway_nats_publisher::AckKind;
use llm_gateway_storage::{postgres::PostgresStorage, AppConfig, Protocol, Storage, UsageRecord};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    // Load config
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

    // Spawn usage worker with supervisor
    let usage_storage = storage.clone();
    let usage_nats = nats.clone();
    tokio::spawn(async move {
        loop {
            run_usage_worker(usage_storage.clone(), usage_nats.clone()).await;
            tracing::warn!("[USAGE-WORKER] exited, restarting in 5s");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    // Spawn audit worker with supervisor
    let audit_storage = storage.clone();
    let audit_nats = nats.clone();
    tokio::spawn(async move {
        loop {
            run_audit_worker(audit_storage.clone(), audit_nats.clone()).await;
            tracing::warn!("[AUDIT-WORKER] exited, restarting in 5s");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    // Wait for shutdown
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down worker");
    Ok(())
}

async fn run_usage_worker(storage: Arc<dyn Storage>, nats: Arc<llm_gateway_nats_publisher::NatsPublisher>) {
    tracing::info!("[USAGE-WORKER] Starting");

    let consumer = match nats.create_usage_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[USAGE-WORKER] Failed to create consumer: {}", e);
            return;
        }
    };

    let mut messages = match consumer.messages().await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("[USAGE-WORKER] Failed to subscribe: {}", e);
            return;
        }
    };

    while let Some(Ok(msg)) = messages.next().await {
        let event: llm_gateway_nats_publisher::UsageEvent = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("[USAGE-WORKER] Failed to deserialize: {}", e);
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
            tracing::warn!("[USAGE-WORKER] Failed to record usage: {}", e);
            let _ = msg.ack_with(AckKind::Nak(None)).await;
            continue;
        }

        let _ = msg.ack().await;
    }

    tracing::info!("[USAGE-WORKER] Exiting");
}

async fn run_audit_worker(storage: Arc<dyn Storage>, nats: Arc<llm_gateway_nats_publisher::NatsPublisher>) {
    tracing::info!("[AUDIT-WORKER] Starting");

    let audit_logger = AuditLogger::new(storage);
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
