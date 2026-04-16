use llm_gateway_audit::AuditLogger;
use llm_gateway_billing::{PricingCalculator, Usage};
use llm_gateway_storage::{Protocol, UsageRecord};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

use crate::AuditTask;

/// Background worker: listens to audit channel and logs to database
/// Does NOT block proxy response - fire-and-forget via MPSC channel
pub async fn start_audit_worker(storage: Arc<dyn llm_gateway_storage::Storage>, mut rx: mpsc::Receiver<AuditTask>) {
    tracing::info!("[AUDIT-WORKER] Starting audit worker");
    let audit_logger = Arc::new(AuditLogger::new(storage));
    while let Some(task) = rx.recv().await {
        tracing::debug!("[AUDIT-WORKER] Received task: key_id={}, model={}, stream={}", task.key_id, task.model_name, task.stream);
        let _ = audit_worker(
            audit_logger.clone(),
            task.key_id,
            task.model_name,
            task.provider_id,
            task.protocol,
            task.stream,
            task.request_body,
            task.response_body,
            task.status_code,
            task.latency_ms,
            task.input_tokens,
            task.output_tokens,
        ).await;
        tracing::debug!("[AUDIT-WORKER] Task completed");
    }
    tracing::info!("[AUDIT-WORKER] Worker exiting, channel closed");
}

/// Async worker: calculates usage and writes to usage_records
/// Runs independently - does NOT block proxy response
pub async fn usage_worker(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    key_id: String,
    model_name: String,
    provider_id: String,
    channel_id: Option<String>,
    protocol: Protocol,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    pricing_policy_id: Option<String>,
    model_input_price: f64,
    model_output_price: f64,
    model_request_price: f64,
) {
    let usage = Usage::from_tokens(input_tokens, output_tokens, 1);
    let calculator = PricingCalculator;

    let cost = if let Some(policy_id) = &pricing_policy_id {
        match storage.get_pricing_policy(policy_id).await {
            Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
            _ => 0.0,
        }
    } else {
        llm_gateway_billing::calculate_cost(
            &llm_gateway_storage::BillingType::Token,
            input_tokens,
            output_tokens,
            model_input_price,
            model_output_price,
            model_request_price,
        ).cost
    };

    let record = UsageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        key_id,
        model_name,
        provider_id,
        channel_id,
        protocol,
        input_tokens,
        output_tokens,
        cost,
        created_at: chrono::Utc::now(),
    };

    if let Err(e) = storage.record_usage(&record).await {
        debug!("Failed to record usage: {}", e);
    }
}

/// Async worker: logs audit record
/// Runs independently - does NOT block proxy response
pub async fn audit_worker(
    audit_logger: Arc<AuditLogger>,
    key_id: String,
    model_name: String,
    provider_id: String,
    protocol: Protocol,
    stream: bool,
    request_body: String,
    response_body: String,
    status_code: i32,
    latency_ms: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
) {
    tracing::debug!("[AUDIT-WORKER] Calling log_request: key_id={}, stream={}", key_id, stream);
    let result = audit_logger.log_request(
        &key_id,
        &model_name,
        &provider_id,
        protocol,
        stream,
        &request_body,
        &response_body,
        status_code,
        latency_ms,
        input_tokens,
        output_tokens,
    ).await;
    match result {
        Ok(()) => tracing::debug!("[AUDIT-WORKER] log_request success"),
        Err(e) => tracing::warn!("[AUDIT-WORKER] log_request failed: {}", e),
    }
}