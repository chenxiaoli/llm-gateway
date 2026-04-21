use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_encryption::{decrypt, encrypt};
use llm_gateway_storage::{Channel, ChannelModel, CreateChannel, UpdateChannel, UpdateChannelApiKey};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateChannel>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let provider_id = input.provider_id.clone();
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Channel name must not be empty".to_string()));
    }
    if name.len() > 100 {
        return Err(ApiError::BadRequest("Channel name must be at most 100 characters".to_string()));
    }

    state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let now = chrono::Utc::now();
    let encrypted_key = encrypt(&input.api_key, &state.encryption_key)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let channel = Channel {
        id: uuid::Uuid::new_v4().to_string(),
        provider_id,
        name,
        api_key: encrypted_key,
        priority: input.priority.unwrap_or(0),
        pricing_policy_id: input.pricing_policy_id,
        markup_ratio: input.markup_ratio.unwrap_or(1.0),
        rpm_limit: input.rpm_limit,
        tpm_limit: input.tpm_limit,
        balance: input.balance,
        weight: input.weight,
        enabled: input.enabled.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };

    let models: Vec<ChannelModel> = input
        .models
        .iter()
        .map(|m| {
            let now = chrono::Utc::now();
            ChannelModel {
                id: uuid::Uuid::new_v4().to_string(),
                channel_id: channel.id.clone(),
                model_id: m.model_id.clone(),
                upstream_model_name: m.upstream_model_name.clone(),
                priority_override: m.priority_override,
                pricing_policy_id: m.pricing_policy_id.clone(),
                markup_ratio: m.markup_ratio.unwrap_or(1.0),
                enabled: m.enabled.unwrap_or(true),
                created_at: now,
                updated_at: now,
            }
        })
        .collect();

    // Validate all models exist before creating anything
    for m in &models {
        state
            .storage
            .get_model_by_id(&m.model_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or(ApiError::NotFound(format!("Model '{}' not found", m.model_id)))?;
    }

    let created = state
        .storage
        .create_channel_with_models(&channel, models)
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

pub async fn list_all_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Channel>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channels = state
        .storage
        .list_channels()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Decrypt api_key for each channel
    let channels = channels.into_iter().map(|mut c| {
        c.api_key = decrypt(&c.api_key, &state.encryption_key).unwrap_or_else(|_| c.api_key);
        c
    }).collect();

    Ok(Json(channels))
}

pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    // Decrypt api_key for display
    channel.api_key = decrypt(&channel.api_key, &state.encryption_key)
        .unwrap_or_else(|_| channel.api_key);

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
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err(ApiError::BadRequest("Channel name must not be empty".to_string()));
        }
        if trimmed.len() > 100 {
            return Err(ApiError::BadRequest("Channel name must be at most 100 characters".to_string()));
        }
        channel.name = trimmed;
    }
    if let Some(priority) = input.priority {
        channel.priority = priority;
    }
    if let Some(enabled) = input.enabled {
        channel.enabled = enabled;
    }
    if let Some(rpm_limit) = input.rpm_limit {
        channel.rpm_limit = rpm_limit;
    }
    if let Some(tpm_limit) = input.tpm_limit {
        channel.tpm_limit = tpm_limit;
    }
    if let Some(balance) = input.balance {
        channel.balance = balance;
    }
    if let Some(weight) = input.weight {
        channel.weight = weight;
    }
    channel.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_channel(&channel)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

/// Dedicated endpoint for updating a channel's API key.
/// Separated from general channel updates to prevent accidental key clearing.
pub async fn update_channel_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateChannelApiKey>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    if input.api_key.is_empty() {
        return Err(ApiError::BadRequest("API key must not be empty".to_string()));
    }

    channel.api_key = encrypt(&input.api_key, &state.encryption_key)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
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
