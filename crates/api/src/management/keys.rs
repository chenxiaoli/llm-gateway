use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_auth::{generate_api_key, hash_api_key};
use llm_gateway_storage::{ApiKey, CreateApiKey as StorageCreateApiKey, PaginatedResponse, PaginationParams, UpdateApiKey as StorageUpdateApiKey};

use crate::error::ApiError;
use crate::extractors::require_auth;
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
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let now = chrono::Utc::now();
    let raw_key = generate_api_key();
    let key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        key_hash: hash_api_key(&raw_key),
        rate_limit: input.rate_limit,
        budget_monthly: input.budget_monthly,
        enabled: true,
        created_by: Some(claims.sub),
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
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<ApiKey>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let (page, page_size) = pagination.normalized();
    let result = if claims.role == "admin" {
        state
            .storage
            .list_keys_paginated(page, page_size)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
    } else {
        state
            .storage
            .list_keys_paginated_for_user(&claims.sub, page, page_size)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
    };

    Ok(Json(result))
}

pub async fn get_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiKey>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let key = state
        .storage
        .get_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

    if claims.role != "admin" && key.created_by.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Key '{}' not found", id)));
    }

    Ok(Json(key))
}

pub async fn update_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<StorageUpdateApiKey>,
) -> Result<Json<ApiKey>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let mut key = state
        .storage
        .get_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

    if claims.role != "admin" && key.created_by.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Key '{}' not found", id)));
    }

    if let Some(name) = input.name { key.name = name; }
    if let Some(rate_limit) = input.rate_limit { key.rate_limit = rate_limit; }
    if let Some(budget_monthly) = input.budget_monthly { key.budget_monthly = budget_monthly; }
    if let Some(enabled) = input.enabled { key.enabled = enabled; }
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
    let claims = require_auth(&headers, &state.jwt_secret)?;

    if claims.role != "admin" {
        let key = state
            .storage
            .get_key(&id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

        if key.created_by.as_deref() != Some(&claims.sub) {
            return Err(ApiError::NotFound(format!("Key '{}' not found", id)));
        }
    }

    state
        .storage
        .delete_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
