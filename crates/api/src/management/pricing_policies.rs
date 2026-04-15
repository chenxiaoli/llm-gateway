use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{CreatePricingPolicy, PricingPolicy};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreatePricingPolicy>,
) -> Result<Json<PricingPolicy>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let now = chrono::Utc::now();
    let policy = PricingPolicy {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        billing_type: input.billing_type,
        config: input.config,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_pricing_policy(&policy)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<PricingPolicy>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let policies = state
        .storage
        .list_pricing_policies()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(policies))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<PricingPolicy>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let policy = state
        .storage
        .get_pricing_policy(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    match policy {
        Some(p) => Ok(Json(p)),
        None => Err(ApiError::NotFound("pricing policy not found".to_string())),
    }
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(), ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .delete_pricing_policy(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(())
}