use llm_gateway_billing::{PricingCalculator, Usage};
use llm_gateway_storage::{Protocol, UsageRecord};
use std::sync::Arc;
use tracing::debug;

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
    audit_logger: Arc<llm_gateway_audit::AuditLogger>,
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
    let _ = audit_logger.log_request(
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
}