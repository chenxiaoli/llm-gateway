use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{Channel, CreateChannel, UpdateChannel};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(input): Json<CreateChannel>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let now = chrono::Utc::now();
    let channel = Channel {
        id: uuid::Uuid::new_v4().to_string(),
        provider_id,
        name: input.name,
        api_key: input.api_key,
        base_url: input.base_url,
        priority: input.priority.unwrap_or(0),
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_channel(&channel)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<Channel>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channels = state
        .storage
        .list_channels_by_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(channels))
}

pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    Ok(Json(channel))
}

pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateChannel>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    if let Some(name) = input.name {
        channel.name = name;
    }
    if let Some(api_key) = input.api_key {
        channel.api_key = api_key;
    }
    if let Some(base_url) = input.base_url {
        channel.base_url = base_url;
    }
    if let Some(priority) = input.priority {
        channel.priority = priority;
    }
    if let Some(enabled) = input.enabled {
        channel.enabled = enabled;
    }
    channel.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_channel(&channel)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .delete_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
