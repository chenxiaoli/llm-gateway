use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_auth::{generate_api_key, hash_api_key};
use llm_gateway_storage::{ApiKey, CreateApiKey as StorageCreateApiKey, UpdateApiKey as StorageUpdateApiKey};

use crate::error::ApiError;
use crate::extractors::verify_admin_token;
use crate::AppState;

pub async fn create_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<StorageCreateApiKey>,
) -> Result<Json<ApiKey>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;

    let raw_key = generate_api_key();
    let key_hash = hash_api_key(&raw_key);
    let now = chrono::Utc::now();

    let api_key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        key_hash,
        rate_limit: input.rate_limit,
        budget_monthly: input.budget_monthly,
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_key(&api_key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Note: raw_key is not returned here; it will be handled in Task 19.
    Ok(Json(created))
}

pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKey>>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;

    let keys = state
        .storage
        .list_keys()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(keys))
}

pub async fn get_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiKey>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;

    let key = state
        .storage
        .get_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

    Ok(Json(key))
}

pub async fn update_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<StorageUpdateApiKey>,
) -> Result<Json<ApiKey>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;

    let mut key = state
        .storage
        .get_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

    // Apply partial updates
    if let Some(name) = input.name {
        key.name = name;
    }
    if let Some(rate_limit) = input.rate_limit {
        key.rate_limit = rate_limit;
    }
    if let Some(budget_monthly) = input.budget_monthly {
        key.budget_monthly = budget_monthly;
    }
    if let Some(enabled) = input.enabled {
        key.enabled = enabled;
    }
    key.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_key(&key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;

    state
        .storage
        .delete_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
