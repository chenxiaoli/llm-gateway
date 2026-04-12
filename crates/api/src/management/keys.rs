use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_auth::{generate_api_key, hash_api_key};
use llm_gateway_storage::{ApiKey, CreateApiKey as StorageCreateApiKey, UpdateApiKey as StorageUpdateApiKey};

use crate::error::ApiError;
use crate::extractors::verify_admin_token;
use crate::AppState;

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id: String,
    pub name: String,
    pub key: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<StorageCreateApiKey>,
) -> Result<Json<CreateKeyResponse>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;

    let now = chrono::Utc::now();
    let raw_key = generate_api_key();
    let key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        key_hash: hash_api_key(&raw_key),
        rate_limit: input.rate_limit,
        budget_monthly: input.budget_monthly,
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_key(&key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(CreateKeyResponse {
        id: created.id,
        name: created.name,
        key: raw_key,
        rate_limit: created.rate_limit,
        budget_monthly: created.budget_monthly,
        enabled: created.enabled,
        created_at: created.created_at,
    }))
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
