use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{
    AccountResponse, CreateTransaction, PaginatedResponse,
    PaginationParams, TransactionResponse, UpdateAccountThreshold,
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

    let mut account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    if input.amount <= 0.0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    let now = chrono::Utc::now();
    let new_balance = account.balance + input.amount;

    let transaction = llm_gateway_storage::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: account.id.clone(),
        transaction_type: llm_gateway_storage::TransactionType::Credit,
        amount: input.amount,
        balance_after: new_balance,
        description: input.description.or_else(|| Some("Recharge".to_string())),
        reference_id: input.reference_id,
        created_at: now,
    };

    account.balance = new_balance;
    account.updated_at = now;

    state
        .storage
        .create_transaction(&transaction)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let updated = state
        .storage
        .update_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountResponse::from(&updated)))
}

pub async fn adjust(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(input): Json<CreateTransaction>,
) -> Result<Json<AccountResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    if input.amount <= 0.0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    let tx_type = match input.transaction_type.as_str() {
        "credit_adjustment" => llm_gateway_storage::TransactionType::CreditAdjustment,
        "debit_refund" => llm_gateway_storage::TransactionType::DebitRefund,
        _ => {
            return Err(ApiError::BadRequest(
                "type must be 'credit_adjustment' or 'debit_refund'".to_string(),
            ))
        }
    };

    let now = chrono::Utc::now();
    let new_balance = if tx_type == llm_gateway_storage::TransactionType::CreditAdjustment {
        account.balance + input.amount
    } else {
        account.balance - input.amount
    };

    let transaction = llm_gateway_storage::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: account.id.clone(),
        transaction_type: tx_type,
        amount: input.amount,
        balance_after: new_balance,
        description: input.description.or_else(|| Some("Manual adjustment".to_string())),
        reference_id: input.reference_id,
        created_at: now,
    };

    account.balance = new_balance;
    account.updated_at = now;

    state
        .storage
        .create_transaction(&transaction)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let updated = state
        .storage
        .update_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountResponse::from(&updated)))
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
