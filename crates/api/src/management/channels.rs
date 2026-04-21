use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::collections::HashMap;
use std::sync::Arc;

use llm_gateway_encryption::{decrypt, encrypt};
use llm_gateway_storage::{Channel, ChannelModel, CreateChannel, UpdateChannel, UpdateChannelApiKey};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

/// Summary of a channel model with model name resolved from the models table.
#[derive(Debug, serde::Serialize)]
pub struct ChannelModelInfo {
    pub id: String,
    pub model_id: String,
    pub model_name: String,
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub enabled: bool,
}

/// Channel with its associated models (resolved model names included).
#[derive(Debug, serde::Serialize)]
pub struct ChannelWithModels {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<f64>,
    pub weight: Option<i32>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub models: Vec<ChannelModelInfo>,
}

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
        .unwrap_or_default()
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
) -> Result<Json<Vec<ChannelWithModels>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channels = state
        .storage
        .list_channels()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Batch-fetch all channel models and all models (for name resolution)
    let all_cms = state
        .storage
        .list_channel_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let all_models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build model_id → model_name lookup map
    let model_name_map: HashMap<String, String> = all_models
        .into_iter()
        .map(|m| (m.model.id.clone(), m.model.name.clone()))
        .collect();

    // Group channel models by channel_id
    let mut cms_by_channel: HashMap<String, Vec<ChannelModel>> = HashMap::new();
    for cm in all_cms {
        cms_by_channel.entry(cm.channel_id.clone()).or_default().push(cm);
    }

    let result: Vec<ChannelWithModels> = channels
        .into_iter()
        .map(|c| {
            let api_key = decrypt(&c.api_key, &state.encryption_key).unwrap_or_else(|_| c.api_key.clone());
            let channel_id = c.id.clone();
            let models: Vec<ChannelModelInfo> = cms_by_channel
                .remove(&channel_id)
                .unwrap_or_default()
                .into_iter()
                .map(|cm| ChannelModelInfo {
                    id: cm.id,
                    model_id: cm.model_id.clone(),
                    model_name: model_name_map.get(&cm.model_id).cloned().unwrap_or_else(|| cm.model_id.clone()),
                    upstream_model_name: cm.upstream_model_name,
                    priority_override: cm.priority_override,
                    pricing_policy_id: cm.pricing_policy_id,
                    markup_ratio: cm.markup_ratio,
                    enabled: cm.enabled,
                })
                .collect();
            ChannelWithModels {
                id: c.id,
                provider_id: c.provider_id,
                name: c.name,
                api_key,
                priority: c.priority,
                pricing_policy_id: c.pricing_policy_id,
                markup_ratio: c.markup_ratio,
                rpm_limit: c.rpm_limit,
                tpm_limit: c.tpm_limit,
                balance: c.balance,
                weight: c.weight,
                enabled: c.enabled,
                created_at: c.created_at.to_rfc3339(),
                updated_at: c.updated_at.to_rfc3339(),
                models,
            }
        })
        .collect();

    Ok(Json(result))
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
