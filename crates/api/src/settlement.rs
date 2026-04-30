//! Background settlement worker — deducts balance from accounts based on usage records.
//! Runs every N minutes (default 1 min), summarizes usage cost per user, creates debit transactions.

use llm_gateway_storage::TransactionType;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// Task sent to the settlement worker (triggers immediate run).
#[derive(Debug)]
pub struct SettlementTrigger;

/// Background task: periodically settles usage charges.
pub async fn start_settlement_worker(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    mut trigger_rx: mpsc::Receiver<SettlementTrigger>,
    interval_secs: u64,
) {
    tracing::info!("[SETTLEMENT] Starting settlement worker (interval: {}s)", interval_secs);
    let mut ticker = interval(Duration::from_secs(interval_secs));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                run_settlement(&storage).await;
            }
            _ = trigger_rx.recv() => {
                tracing::info!("[SETTLEMENT] Triggered immediate settlement");
                run_settlement(&storage).await;
            }
        }
    }
}

async fn run_settlement(storage: &Arc<dyn llm_gateway_storage::Storage>) {
    tracing::info!("[SETTLEMENT] Running settlement task");

    let checkpoint_key = "last_settlement_time";
    let now = chrono::Utc::now();
    let last_time = match storage.get_setting(checkpoint_key).await {
        Ok(Some(ts)) => {
            chrono::DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| now - Duration::from_secs(300))
        }
        _ => now - Duration::from_secs(300),
    };

    // Query usage costs grouped by user_id (single query instead of N+1)
    let user_costs = storage
        .query_usage_cost_by_user(last_time, now)
        .await
        .unwrap_or_default();

    if user_costs.is_empty() {
        tracing::debug!("[SETTLEMENT] No usage records in window, skipping");
        let _ = storage.set_setting(checkpoint_key, &now.to_rfc3339()).await;
        return;
    }

    // Resolve user_id → account_id
    let mut account_charges: HashMap<String, i64> = HashMap::new();

    for (user_id, cost) in &user_costs {
        if let Some(account) = storage
            .get_account_by_user_id(user_id)
            .await
            .unwrap_or(None)
        {
            *account_charges.entry(account.id.clone()).or_insert(0) += cost;
        }
    }

    if account_charges.is_empty() {
        tracing::debug!("[SETTLEMENT] No accounts found for usage records");
        let _ = storage.set_setting(checkpoint_key, &now.to_rfc3339()).await;
        return;
    }

    let batch_reference = format!("batch_{}", now.timestamp_micros());

    for (account_id, total_cost) in &account_charges {
        if *total_cost <= 0 {
            continue;
        }

        // Idempotency: skip if already settled for this batch
        if storage
            .get_transaction_by_reference(&account_id, &batch_reference)
            .await
            .unwrap_or(None)
            .is_some()
        {
            tracing::debug!(
                "[SETTLEMENT] Account {} already settled for batch {}, skipping",
                account_id,
                batch_reference
            );
            continue;
        }

        let req = llm_gateway_storage::DeductBalance {
            account_id: account_id.clone(),
            amount: *total_cost,
            transaction_type: TransactionType::Debit,
            description: Some("Usage settlement".to_string()),
            reference_id: Some(batch_reference.clone()),
        };

        match storage.deduct_balance(&req).await {
            Ok(llm_gateway_storage::DeductBalanceResult::Success(tx)) => {
                tracing::info!(
                    "[SETTLEMENT] Deducted ${} from account {} (new balance: ${})",
                    total_cost, account_id, tx.balance_after
                );
            }
            Ok(llm_gateway_storage::DeductBalanceResult::InsufficientBalance { current_balance, requested: _ }) => {
                tracing::warn!(
                    "[SETTLEMENT] Insufficient balance for account {}: balance=${}, cost=${}",
                    account_id, current_balance, total_cost
                );
            }
            Ok(llm_gateway_storage::DeductBalanceResult::AccountNotFound) => {
                tracing::warn!("[SETTLEMENT] Account {} not found", account_id);
            }
            Err(e) => {
                tracing::error!(
                    "[SETTLEMENT] Failed to deduct from account {}: {}",
                    account_id, e
                );
            }
        }
    }

    let _ = storage.set_setting(checkpoint_key, &now.to_rfc3339()).await;
    tracing::info!(
        "[SETTLEMENT] Settlement complete, {} accounts processed",
        account_charges.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_trigger_debug() {
        let trigger = SettlementTrigger;
        let debug_str = format!("{:?}", trigger);
        assert!(debug_str.contains("SettlementTrigger"));
    }

    #[test]
    fn test_batch_reference_format() {
        let ts = chrono::Utc::now().timestamp();
        let reference = format!("batch_{}", ts);
        assert!(reference.starts_with("batch_"));
        assert!(reference.len() > 6);
    }
}
