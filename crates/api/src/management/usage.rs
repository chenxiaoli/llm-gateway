use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use llm_gateway_storage::{units_to_usd, PaginatedResponse, PaginationParams, UsageFilter, UsageRecord, UsageSummaryRecord};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

// --- JSON response wrappers with f64 cost fields ---

#[derive(Debug, Clone, Serialize)]
pub struct UsageRecordResponse {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cost: f64,
    pub user_id: Option<String>,
    pub created_at: String,
}

impl From<UsageRecord> for UsageRecordResponse {
    fn from(r: UsageRecord) -> Self {
        UsageRecordResponse {
            id: r.id,
            key_id: r.key_id,
            model_name: r.model_name,
            provider_id: r.provider_id,
            channel_id: r.channel_id,
            protocol: format!("{:?}", r.protocol).to_lowercase(),
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cache_read_tokens: r.cache_read_tokens,
            cache_creation_tokens: r.cache_creation_tokens,
            cost: units_to_usd(r.cost),
            user_id: r.user_id,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummaryResponse {
    pub model_name: String,
    pub total_input_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

impl From<UsageSummaryRecord> for UsageSummaryResponse {
    fn from(r: UsageSummaryRecord) -> Self {
        UsageSummaryResponse {
            model_name: r.model_name,
            total_input_tokens: r.total_input_tokens,
            total_cache_read_tokens: r.total_cache_read_tokens,
            total_cache_creation_tokens: r.total_cache_creation_tokens,
            total_output_tokens: r.total_output_tokens,
            total_cost: units_to_usd(r.total_cost),
            request_count: r.request_count,
        }
    }
}

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
) -> Result<Json<PaginatedResponse<UsageRecordResponse>>, ApiError> {
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

    Ok(Json(PaginatedResponse {
        items: result.items.into_iter().map(UsageRecordResponse::from).collect(),
        total: result.total,
        page: result.page,
        page_size: result.page_size,
    }))
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
) -> Result<Json<Vec<UsageSummaryResponse>>, ApiError> {
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

    Ok(Json(records.into_iter().map(UsageSummaryResponse::from).collect()))
}
