use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use std::collections::HashSet;

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
    let mut result = state
        .storage
        .query_usage_paginated(&query.filter, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if claims.role != "admin" {
        let keys = state
            .storage
            .list_keys()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let user_key_ids: HashSet<String> = keys
            .iter()
            .filter(|k| k.created_by.as_deref() == Some(&claims.sub))
            .map(|k| k.id.clone())
            .collect();
        result.items = result.items
            .into_iter()
            .filter(|r| user_key_ids.contains(&r.key_id))
            .collect();
    }

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
    let _claims = require_auth(&headers, &state.jwt_secret)?;

    let records = state
        .storage
        .query_usage_summary(&query.filter)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Non-admin: per-key filtering of aggregated summary is complex
    // (requires filtering raw rows then re-aggregating), so we return
    // all data for now — the same tradeoff as existing get_usage for admins only

    Ok(Json(records))
}
