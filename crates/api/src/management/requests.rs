use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{AuditLog, Transaction, UsageRecord};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct RequestDetailsResponse {
    pub transaction: Option<Transaction>,
    pub usage: Option<UsageRecord>,
    pub audit: Option<AuditLog>,
}

pub async fn get_request_details(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<Json<RequestDetailsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let transaction = state
        .storage
        .get_transaction_by_request_id(&request_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let usage = state
        .storage
        .get_usage_by_request_id(&request_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let audit = state
        .storage
        .get_audit_by_request_id(&request_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(RequestDetailsResponse {
        transaction,
        usage,
        audit,
    }))
}
