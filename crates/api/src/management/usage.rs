use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_storage::{PaginatedResponse, PaginationParams, UsageFilter, UsageRecord, UsageSummaryRecord};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    #[serde(flatten)]
    pub filter: UsageFilter,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<UsageQuery>,
) -> Result<Json<PaginatedResponse<UsageRecord>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let (page, page_size) = query.pagination.normalized();
    let mut filter = query.filter;

    if claims.role != "admin" {
        filter.user_id = Some(claims.sub);
    }

    let result = state
        .storage
        .query_usage_paginated(&filter, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct UsageSummaryQuery {
    #[serde(flatten)]
    pub filter: UsageFilter,
}

pub async fn get_usage_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<UsageSummaryQuery>,
) -> Result<Json<Vec<UsageSummaryRecord>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let mut filter = query.filter;

    if claims.role != "admin" {
        filter.user_id = Some(claims.sub);
    }

    let records = state
        .storage
        .query_usage_summary(&filter)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(records))
}
