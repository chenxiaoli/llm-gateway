use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use llm_gateway_auth::{generate_api_key, hash_api_key};
use llm_gateway_storage::{
    opt_units_to_usd, opt_usd_to_units,
    ApiKey, PaginatedResponse, PaginationParams,
};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

// --- JSON request structs (f64 for API boundary) ---

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub model_fallback_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyRequest {
    pub name: Option<String>,
    pub rate_limit: Option<Option<i64>>,
    pub budget_monthly: Option<Option<f64>>,
    pub enabled: Option<bool>,
    pub model_fallback_id: Option<Option<String>>,
}

// --- JSON response structs (f64 for API boundary) ---

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

#[derive(Serialize)]
pub struct KeyResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: Option<String>,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub enabled: bool,
    pub created_by: Option<String>,
    pub model_fallback_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ApiKey> for KeyResponse {
    fn from(k: ApiKey) -> Self {
        KeyResponse {
            id: k.id,
            name: k.name,
            key_prefix: k.key_prefix,
            rate_limit: k.rate_limit,
            budget_monthly: opt_units_to_usd(k.budget_monthly),
            enabled: k.enabled,
            created_by: k.created_by,
            model_fallback_id: k.model_fallback_id,
            created_at: k.created_at,
            updated_at: k.updated_at,
        }
    }
}

pub async fn create_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let budget_monthly_i64 = opt_usd_to_units(input.budget_monthly);

    let now = chrono::Utc::now();
    let raw_key = generate_api_key();
    let key_prefix = raw_key.chars().take(8).collect::<String>();
    let key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        key_hash: hash_api_key(&raw_key),
        key_prefix: Some(key_prefix),
        rate_limit: input.rate_limit,
        budget_monthly: budget_monthly_i64,
        enabled: true,
        created_by: Some(claims.sub),
        model_fallback_id: input.model_fallback_id,
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
        budget_monthly: opt_units_to_usd(created.budget_monthly),
        enabled: created.enabled,
        created_at: created.created_at,
    }))
}

pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<KeyResponse>>, ApiError> {
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

    Ok(Json(PaginatedResponse {
        items: result.items.into_iter().map(KeyResponse::from).collect(),
        total: result.total,
        page: result.page,
        page_size: result.page_size,
    }))
}

pub async fn get_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<KeyResponse>, ApiError> {
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

    Ok(Json(KeyResponse::from(key)))
}

pub async fn update_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateKeyRequest>,
) -> Result<Json<KeyResponse>, ApiError> {
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
    if let Some(budget_monthly) = input.budget_monthly { key.budget_monthly = opt_usd_to_units(budget_monthly); }
    if let Some(enabled) = input.enabled { key.enabled = enabled; }
    if let Some(model_fallback_id) = input.model_fallback_id { key.model_fallback_id = model_fallback_id; }
    key.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_key(&key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(KeyResponse::from(updated)))
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
