use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_storage::{AuditLog, LogFilter, PaginatedResponse, PaginationParams};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(flatten)]
    pub filter: LogFilter,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<LogsQuery>,
) -> Result<Json<PaginatedResponse<AuditLog>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let (page, page_size) = query.pagination.normalized();
    let logs = state
        .storage
        .query_logs_paginated(&query.filter, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(logs))
}
