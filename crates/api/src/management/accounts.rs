use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use llm_gateway_storage::{
    units_to_usd, usd_to_units,
    Account, AccountResponse as StorageAccountResponse,
    AddBalance, AddBalanceResult,
    PaginatedResponse, PaginationParams, TransactionResponse as StorageTransactionResponse,
    TransactionType,
};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

// --- JSON response wrappers with f64 monetary fields ---

#[derive(Serialize)]
pub struct AccountBalanceResponse {
    pub account: AccountJsonResponse,
    pub transactions: PaginatedResponse<TransactionJsonResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountJsonResponse {
    pub id: String,
    pub user_id: String,
    pub balance: f64,
    pub threshold: f64,
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<StorageAccountResponse> for AccountJsonResponse {
    fn from(a: StorageAccountResponse) -> Self {
        AccountJsonResponse {
            id: a.id,
            user_id: a.user_id,
            balance: units_to_usd(a.balance),
            threshold: units_to_usd(a.threshold),
            currency: a.currency,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionJsonResponse {
    pub id: String,
    pub account_id: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub amount: f64,
    pub balance_after: f64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub created_at: String,
}

impl From<StorageTransactionResponse> for TransactionJsonResponse {
    fn from(t: StorageTransactionResponse) -> Self {
        TransactionJsonResponse {
            id: t.id,
            account_id: t.account_id,
            transaction_type: t.transaction_type,
            amount: units_to_usd(t.amount),
            balance_after: units_to_usd(t.balance_after),
            description: t.description,
            reference_id: t.reference_id,
            created_at: t.created_at,
        }
    }
}

// --- JSON request structs (f64 for API boundary) ---

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub amount: f64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateThresholdRequest {
    pub threshold: f64,
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
        account: StorageAccountResponse::from(&account).into(),
        transactions: PaginatedResponse {
            items: transactions.items.iter().map(|t| StorageTransactionResponse::from(t).into()).collect(),
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
    Json(input): Json<CreateTransactionRequest>,
) -> Result<Json<AccountJsonResponse>, ApiError> {
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

    let amount_i64 = usd_to_units(input.amount);

    let req = AddBalance {
        account_id: account.id.clone(),
        amount: amount_i64,
        transaction_type: TransactionType::Credit,
        description: input.description.or_else(|| Some("Recharge".to_string())),
        reference_id: input.reference_id,
    };

    match state.storage.add_balance(&req).await {
        Ok(AddBalanceResult::Success(tx)) => {
            Ok(Json(AccountJsonResponse::from(StorageAccountResponse::from(&Account {
                id: tx.account_id,
                user_id,
                balance: tx.balance_after,
                threshold: account.threshold,
                currency: account.currency,
                created_at: account.created_at,
                updated_at: tx.created_at,
            }))))
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
    Json(input): Json<CreateTransactionRequest>,
) -> Result<Json<AccountJsonResponse>, ApiError> {
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

    let amount_i64 = usd_to_units(input.amount);

    // Guard: DebitRefund must not result in negative balance
    if tx_type == TransactionType::DebitRefund && account.balance < amount_i64 {
        return Err(ApiError::BadRequest(format!(
            "Insufficient balance for debit refund: balance={}, requested={}",
            units_to_usd(account.balance), input.amount
        )));
    }

    let req = AddBalance {
        account_id: account.id.clone(),
        amount: amount_i64,
        transaction_type: tx_type,
        description: input.description.or_else(|| Some("Manual adjustment".to_string())),
        reference_id: input.reference_id,
    };

    match state.storage.add_balance(&req).await {
        Ok(AddBalanceResult::Success(tx)) => {
            Ok(Json(AccountJsonResponse::from(StorageAccountResponse::from(&Account {
                id: tx.account_id,
                user_id,
                balance: tx.balance_after,
                threshold: account.threshold,
                currency: account.currency,
                created_at: account.created_at,
                updated_at: tx.created_at,
            }))))
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
    Json(input): Json<UpdateThresholdRequest>,
) -> Result<Json<AccountJsonResponse>, ApiError> {
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

    account.threshold = usd_to_units(input.threshold);
    account.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountJsonResponse::from(StorageAccountResponse::from(&updated))))
}
