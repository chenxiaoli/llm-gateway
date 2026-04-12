use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{UsageFilter, UsageRecord};

use crate::error::ApiError;
use crate::extractors::verify_admin_token;
use crate::AppState;

pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(filter): Query<UsageFilter>,
) -> Result<Json<Vec<UsageRecord>>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;

    let records = state
        .storage
        .query_usage(&filter)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(records))
}
