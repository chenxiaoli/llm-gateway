use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{CreatePricingPolicy, PricingPolicy, PricingPolicyWithCounts, UpdatePricingPolicy};

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
) -> Result<Json<Vec<PricingPolicyWithCounts>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let policies = state
        .storage
        .list_pricing_policies_with_counts()
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
) -> Result<StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .delete_pricing_policy(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdatePricingPolicy>,
) -> Result<Json<PricingPolicy>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let existing = state
        .storage
        .get_pricing_policy(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Pricing policy '{}' not found", id)))?;

    let updated = PricingPolicy {
        id: existing.id,
        name: input.name.unwrap_or(existing.name),
        billing_type: input.billing_type.unwrap_or(existing.billing_type),
        config: input.config.unwrap_or(existing.config),
        created_at: existing.created_at,
        updated_at: chrono::Utc::now(),
    };

    let result = state
        .storage
        .update_pricing_policy(&updated)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(result))
}