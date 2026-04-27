use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_storage::{AuditLog, AuditLogSummary, LogFilter, PaginatedResponse, PaginationParams};

use crate::error::ApiError;
use crate::extractors::{require_admin, require_auth};
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
) -> Result<Json<PaginatedResponse<AuditLogSummary>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let (page, page_size) = query.pagination.normalized();
    let mut filter = query.filter;
    if claims.role != "admin" {
        filter.user_id = Some(claims.sub);
    }

    let logs = state
        .storage
        .query_logs_paginated(&filter, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(logs))
}

pub async fn get_log(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AuditLog>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let log = state
        .storage
        .get_log(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Log {} not found", id)))?;

    Ok(Json(log))
}
