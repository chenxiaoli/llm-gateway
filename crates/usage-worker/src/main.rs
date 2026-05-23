use futures::StreamExt;
use llm_gateway_nats_publisher::AckKind;
use llm_gateway_storage::{postgres::PostgresStorage, AppConfig, DeductBalance, DeductBalanceResult, Protocol, Storage, TransactionType, UsageRecord};
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
    loop {
        run_usage_worker(storage.clone(), nats.clone()).await;
        tracing::warn!("[USAGE-WORKER] exited, restarting in 5s");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
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
            request_id: Some(event.request_id),
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
            pricing_policy: event.pricing_policy,
            weighted_tokens: event.weighted_tokens,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.created_at)
                .map(|dt| dt.to_utc())
                .unwrap_or_else(|_| chrono::Utc::now()),
        };

        if let Err(e) = storage.record_usage(&record).await {
            tracing::warn!("[USAGE-WORKER] Failed to record usage: {}", e);
            let _ = msg.ack_with(AckKind::Nak(None)).await;
            continue;
        }

        // Per-request deduction
        if let Some(ref user_id) = record.user_id {
            if record.cost > 0 {
                match storage.get_account_by_user_id(user_id).await {
                    Ok(Some(account)) => {
                        // Idempotency: skip if already deducted for this request
                        match record.request_id.as_deref() {
                            Some(rid) => match storage.get_transaction_by_request_id(rid).await {
                                Ok(None) => {
                                    let req = DeductBalance {
                                        account_id: account.id,
                                        amount: record.cost,
                                        transaction_type: TransactionType::Debit,
                                        description: Some(format!("{} - {}", record.model_name, rid)),
                                        reference_id: None,
                                        request_id: Some(rid.to_string()),
                                    };
                                    match storage.deduct_balance(&req).await {
                                        Ok(DeductBalanceResult::Success(_)) => {}
                                        Ok(DeductBalanceResult::InsufficientBalance { current_balance, requested }) => {
                                            tracing::warn!(
                                                "[USAGE] Insufficient balance for user={}, balance={}, cost={}",
                                                user_id, current_balance, requested
                                            );
                                        }
                                        Ok(DeductBalanceResult::AccountNotFound) => {
                                            tracing::warn!("[USAGE] Account not found for user={}", user_id);
                                        }
                                        Err(e) => {
                                            tracing::error!("[USAGE] Deduction failed for request_id={}: {}", rid, e);
                                        }
                                    }
                                }
                                Ok(Some(_)) => {
                                    tracing::debug!("[USAGE] Already deducted for request_id={}", rid);
                                }
                                Err(e) => {
                                    tracing::error!("[USAGE] Idempotency check failed for request_id={}: {}", rid, e);
                                }
                            },
                            None => {
                                tracing::warn!("[USAGE] No request_id for usage record {}, skipping deduction", record.id);
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::debug!("[USAGE] No account for user={}, skipping deduction", user_id);
                    }
                    Err(e) => {
                        tracing::error!("[USAGE] Failed to lookup account for user={}: {}", user_id, e);
                    }
                }
            }
        }

        let _ = msg.ack().await;
    }

    tracing::info!("[USAGE-WORKER] Exiting");
}
