use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{
    Account, AccountResponse, AddBalance, AddBalanceResult, CreateTransaction,
    PaginatedResponse, PaginationParams, TransactionResponse,
    TransactionType, UpdateAccountThreshold,
};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Serialize)]
pub struct AccountBalanceResponse {
    pub account: AccountResponse,
    pub transactions: PaginatedResponse<TransactionResponse>,
}

pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<AccountBalanceResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    let (page, page_size) = pagination.normalized();
    let transactions = state
        .storage
        .list_transactions(&account.id, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountBalanceResponse {
        account: AccountResponse::from(&account),
        transactions: PaginatedResponse {
            items: transactions.items.iter().map(TransactionResponse::from).collect(),
            total: transactions.total,
            page: transactions.page,
            page_size: transactions.page_size,
        },
    }))
}

pub async fn recharge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(input): Json<CreateTransaction>,
) -> Result<Json<AccountResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    if input.amount <= 0.0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    // Look up account for account_id
    let account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    let req = AddBalance {
        account_id: account.id.clone(),
        amount: input.amount,
        transaction_type: TransactionType::Credit,
        description: input.description.or_else(|| Some("Recharge".to_string())),
        reference_id: input.reference_id,
    };

    match state.storage.add_balance(&req).await {
        Ok(AddBalanceResult::Success(tx)) => {
            Ok(Json(AccountResponse::from(&Account {
                id: tx.account_id,
                user_id,
                balance: tx.balance_after,
                threshold: account.threshold,
                currency: account.currency,
                created_at: account.created_at,
                updated_at: tx.created_at,
            })))
        }
        Ok(AddBalanceResult::AccountNotFound) => {
            Err(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))
        }
        Err(e) => Err(ApiError::Internal(e.to_string())),
    }
}

pub async fn adjust(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(input): Json<CreateTransaction>,
) -> Result<Json<AccountResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    if input.amount <= 0.0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    let tx_type = match input.transaction_type.as_str() {
        "credit_adjustment" => TransactionType::CreditAdjustment,
        "debit_refund" => TransactionType::DebitRefund,
        _ => {
            return Err(ApiError::BadRequest(
                "type must be 'credit_adjustment' or 'debit_refund'".to_string(),
            ))
        }
    };

    // Look up account
    let account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    // Guard: DebitRefund must not result in negative balance
    if tx_type == TransactionType::DebitRefund && account.balance < input.amount {
        return Err(ApiError::BadRequest(format!(
            "Insufficient balance for debit refund: balance={}, requested={}",
            account.balance, input.amount
        )));
    }

    let req = AddBalance {
        account_id: account.id.clone(),
        amount: input.amount,
        transaction_type: tx_type,
        description: input.description.or_else(|| Some("Manual adjustment".to_string())),
        reference_id: input.reference_id,
    };

    match state.storage.add_balance(&req).await {
        Ok(AddBalanceResult::Success(tx)) => {
            Ok(Json(AccountResponse::from(&Account {
                id: tx.account_id,
                user_id,
                balance: tx.balance_after,
                threshold: account.threshold,
                currency: account.currency,
                created_at: account.created_at,
                updated_at: tx.created_at,
            })))
        }
        Ok(AddBalanceResult::AccountNotFound) => {
            Err(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))
        }
        Err(e) => Err(ApiError::Internal(e.to_string())),
    }
}

pub async fn update_threshold(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(input): Json<UpdateAccountThreshold>,
) -> Result<Json<AccountResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    if input.threshold < 0.0 {
        return Err(ApiError::BadRequest("Threshold must be non-negative".to_string()));
    }

    account.threshold = input.threshold;
    account.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountResponse::from(&updated)))
}
