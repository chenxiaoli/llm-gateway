use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{ChannelModel, CreateChannelModel, UpdateChannelModel};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn create_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(input): Json<CreateChannelModel>,
) -> Result<Json<ChannelModel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // channel_id is required when creating by provider
    let channel_id = input.channel_id.as_ref()
        .ok_or(ApiError::BadRequest("channel_id is required".to_string()))?;

    // Verify channel belongs to provider
    let channel = state.storage.get_channel(channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Channel not found".to_string()))?;

    if channel.provider_id != provider_id {
        return Err(ApiError::BadRequest("Channel does not belong to provider".to_string()));
    }

    // Verify model exists
    let _model = state.storage.get_model_by_id(&input.model_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Model not found".to_string()))?;

    let now = chrono::Utc::now();
    let cm = ChannelModel {
        id: uuid::Uuid::new_v4().to_string(),
        channel_id: channel_id.clone(),
        model_id: input.model_id,
        upstream_model_name: input.upstream_model_name,
        priority_override: input.priority_override,
        pricing_policy_id: input.pricing_policy_id,
        markup_ratio: input.markup_ratio.unwrap_or(1.0),
        enabled: input.enabled.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };

    let created = state.storage.create_channel_model(&cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn create_channel_model_by_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(channel_id): Path<String>,
    Json(input): Json<CreateChannelModel>,
) -> Result<Json<ChannelModel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Verify channel exists
    let _channel = state.storage.get_channel(&channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Channel not found".to_string()))?;

    // Verify model exists by ID
    let _model = state.storage.get_model_by_id(&input.model_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Model not found".to_string()))?;

    let now = chrono::Utc::now();
    let cm = ChannelModel {
        id: uuid::Uuid::new_v4().to_string(),
        channel_id: channel_id.clone(),
        model_id: input.model_id,
        upstream_model_name: input.upstream_model_name,
        priority_override: input.priority_override,
        pricing_policy_id: input.pricing_policy_id,
        markup_ratio: input.markup_ratio.unwrap_or(1.0),
        enabled: input.enabled.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };

    let created = state.storage.create_channel_model(&cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_channel_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<ChannelModel>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Get channels for provider, then channel_models for those channels
    let channels = state.storage.list_channels_by_provider(&provider_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let channel_ids: Vec<String> = channels.iter().map(|c| c.id.clone()).collect();

    // Get all channel_models for these channels
    let mut all_models = Vec::new();
    for channel_id in channel_ids {
        let cms = state.storage.list_channel_models_by_channel(&channel_id).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        all_models.extend(cms);
    }

    Ok(Json(all_models))
}

pub async fn get_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ChannelModel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let cm = state.storage.get_channel_model(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("ChannelModel not found".to_string()))?;

    Ok(Json(cm))
}

pub async fn update_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateChannelModel>,
) -> Result<Json<ChannelModel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut cm = state.storage.get_channel_model(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("ChannelModel not found".to_string()))?;

    if let Some(upstream) = input.upstream_model_name {
        cm.upstream_model_name = Some(upstream);
    }
    if let Some(priority) = input.priority_override {
        cm.priority_override = priority;
    }
    if let Some(pricing_policy_id) = input.pricing_policy_id {
        cm.pricing_policy_id = pricing_policy_id;
    }
    if let Some(markup_ratio) = input.markup_ratio {
        cm.markup_ratio = markup_ratio;
    }
    if let Some(enabled) = input.enabled {
        cm.enabled = enabled;
    }
    cm.updated_at = chrono::Utc::now();

    let updated = state.storage.update_channel_model(&cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state.storage.delete_channel_model(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_channel_models_by_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<ChannelModel>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let models = state.storage.list_channel_models_by_channel(&channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(models))
}
