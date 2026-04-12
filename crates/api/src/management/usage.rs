use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;
use std::collections::HashSet;

use llm_gateway_storage::{UsageFilter, UsageRecord};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(filter): Query<UsageFilter>,
) -> Result<Json<Vec<UsageRecord>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let records = state
        .storage
        .query_usage(&filter)
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
        let filtered: Vec<UsageRecord> = records
            .into_iter()
            .filter(|r| user_key_ids.contains(&r.key_id))
            .collect();
        return Ok(Json(filtered));
    }

    Ok(Json(records))
}
