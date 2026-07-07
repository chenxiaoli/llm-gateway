use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use std::sync::Arc;

use llm_gateway_org::{can_create_org_catalog, can_create_platform_catalog, resolve_org_context};
use llm_gateway_storage::{CreatePricingPolicy, PricingPolicy, PricingPolicyWithCounts, UpdatePricingPolicy};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreatePricingPolicy>,
) -> Result<Json<PricingPolicy>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;

    let owner_org_id = input.owner_org_id.clone().or_else(|| {
        if can_create_platform_catalog(&ctx) { None } else { Some(ctx.org_id.clone()) }
    });
    if owner_org_id.as_deref() == Some(ctx.org_id.as_str()) {
        if !can_create_org_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else if owner_org_id.is_none() {
        if !can_create_platform_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else {
        return Err(ApiError::Forbidden);
    }

    let now = chrono::Utc::now();
    let policy = PricingPolicy {
        id: uuid::Uuid::new_v4().to_string(),
        owner_org_id,
        name: input.name,
        billing_type: input.billing_type,
        config: input.config,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_pricing_policy(&ctx.org_id, &policy)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<PricingPolicyWithCounts>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;

    let policies = state
        .storage
        .list_pricing_policies_with_counts(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(policies))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<PricingPolicy>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;

    let policy = state
        .storage
        .get_pricing_policy(&ctx.org_id, &id)
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
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;

    let existing = state
        .storage
        .get_pricing_policy(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Pricing policy '{}' not found", id)))?;

    if let Some(ref owner_org_id) = existing.owner_org_id {
        if owner_org_id != &ctx.org_id || !can_create_org_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else if !can_create_platform_catalog(&ctx) {
        return Err(ApiError::Forbidden);
    }

    state
        .storage
        .delete_pricing_policy(&ctx.org_id, &id)
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
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;

    let existing = state
        .storage
        .get_pricing_policy(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Pricing policy '{}' not found", id)))?;

    if let Some(ref owner_org_id) = existing.owner_org_id {
        if owner_org_id != &ctx.org_id || !can_create_org_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else if !can_create_platform_catalog(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let updated = PricingPolicy {
        id: existing.id,
        owner_org_id: existing.owner_org_id,
        name: input.name.unwrap_or(existing.name),
        billing_type: input.billing_type.unwrap_or(existing.billing_type),
        config: input.config.unwrap_or(existing.config),
        created_at: existing.created_at,
        updated_at: chrono::Utc::now(),
    };

    let result = state
        .storage
        .update_pricing_policy(&ctx.org_id, &updated)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(result))
}
