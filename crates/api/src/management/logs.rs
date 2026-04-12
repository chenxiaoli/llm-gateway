use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{AuditLog, LogFilter};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(filter): Query<LogFilter>,
) -> Result<Json<Vec<AuditLog>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let logs = state
        .storage
        .query_logs(&filter)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(logs))
}
