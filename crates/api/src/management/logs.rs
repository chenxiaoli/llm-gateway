use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_org::{can_manage_channels, resolve_org_context};
use llm_gateway_storage::{AuditLog, AuditLogSummary, LogFilter, PaginatedResponse, PaginationParams};

use crate::error::ApiError;
use crate::extractors::require_auth;
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
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;

    let (page, page_size) = query.pagination.normalized();
    let mut filter = query.filter;
    if !can_manage_channels(&ctx) {
        filter.user_id = Some(claims.sub);
    }

    let logs = state
        .storage
        .query_logs_paginated(&ctx.org_id, &filter, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(logs))
}

pub async fn get_log(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AuditLog>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;

    let log = state
        .storage
        .get_log(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Log {} not found", id)))?;

    // Members may only fetch their own audit logs; otherwise they could read
    // request/response bodies of other members' LLM calls. Admins/owners (and
    // platform_admin via can_manage_channels) see all logs in the org. Return
    // 404 (not 403) to avoid leaking log existence.
    if !can_manage_channels(&ctx) && log.user_id.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Log {} not found", id)));
    }

    Ok(Json(log))
}
