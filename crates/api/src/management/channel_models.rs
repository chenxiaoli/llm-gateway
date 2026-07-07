use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use llm_gateway_org::{can_access_channel, can_manage_channels, OrgContext};
use llm_gateway_storage::{bps_to_ratio, ratio_to_bps, ChannelModel};

use crate::error::ApiError;
use crate::AppState;

// --- JSON response wrapper with f64 markup_ratio ---

#[derive(Debug, Clone, Serialize)]
pub struct ChannelModelResponse {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ChannelModel> for ChannelModelResponse {
    fn from(cm: ChannelModel) -> Self {
        ChannelModelResponse {
            id: cm.id,
            channel_id: cm.channel_id,
            model_id: cm.model_id,
            upstream_model_name: cm.upstream_model_name,
            priority_override: cm.priority_override,
            pricing_policy_id: cm.pricing_policy_id,
            markup_ratio: bps_to_ratio(cm.markup_ratio),
            enabled: cm.enabled,
            created_at: cm.created_at.to_rfc3339(),
            updated_at: cm.updated_at.to_rfc3339(),
        }
    }
}

// --- JSON request structs (f64 markup_ratio for API boundary) ---

#[derive(Debug, Deserialize)]
pub struct CreateChannelModelRequest {
    pub channel_id: Option<String>,
    pub model_id: String,
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: Option<f64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelModelRequest {
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<Option<i32>>,
    pub pricing_policy_id: Option<Option<String>>,
    pub markup_ratio: Option<f64>,
    pub enabled: Option<bool>,
}

pub async fn create_channel_model(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, provider_id)): Path<(String, String)>,
    Json(input): Json<CreateChannelModelRequest>,
) -> Result<Json<ChannelModelResponse>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // channel_id is required when creating by provider
    let channel_id = input.channel_id.as_ref()
        .ok_or(ApiError::BadRequest("channel_id is required".to_string()))?;

    // Verify channel belongs to provider (and is visible to caller).
    let channel = state.storage.get_channel(&ctx.org_id, channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Channel not found".to_string()))?;
    if !can_access_channel(&ctx, channel.group_id.as_deref()) {
        return Err(ApiError::NotFound("Channel not found".to_string()));
    }

    if channel.provider_id != provider_id {
        return Err(ApiError::BadRequest("Channel does not belong to provider".to_string()));
    }

    // Verify model exists
    let _model = state.storage.get_model_by_id(&ctx.org_id, &input.model_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Model not found".to_string()))?;

    let now = chrono::Utc::now();
    let cm = ChannelModel {
        id: uuid::Uuid::new_v4().to_string(),
        org_id: ctx.org_id.clone(),
        channel_id: channel_id.clone(),
        model_id: input.model_id,
        upstream_model_name: input.upstream_model_name,
        priority_override: input.priority_override,
        pricing_policy_id: input.pricing_policy_id,
        markup_ratio: ratio_to_bps(input.markup_ratio.unwrap_or(1.0)),
        enabled: input.enabled.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };

    let created = state.storage.create_channel_model(&ctx.org_id, &cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ChannelModelResponse::from(created)))
}

pub async fn create_channel_model_by_channel(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, channel_id)): Path<(String, String)>,
    Json(input): Json<CreateChannelModelRequest>,
) -> Result<Json<ChannelModelResponse>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // Verify channel exists and caller can manage it.
    let channel = state.storage.get_channel(&ctx.org_id, &channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Channel not found".to_string()))?;
    if !can_access_channel(&ctx, channel.group_id.as_deref()) {
        return Err(ApiError::NotFound("Channel not found".to_string()));
    }

    // Verify model exists by ID
    let _model = state.storage.get_model_by_id(&ctx.org_id, &input.model_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Model not found".to_string()))?;

    let now = chrono::Utc::now();
    let cm = ChannelModel {
        id: uuid::Uuid::new_v4().to_string(),
        org_id: ctx.org_id.clone(),
        channel_id: channel_id.clone(),
        model_id: input.model_id,
        upstream_model_name: input.upstream_model_name,
        priority_override: input.priority_override,
        pricing_policy_id: input.pricing_policy_id,
        markup_ratio: ratio_to_bps(input.markup_ratio.unwrap_or(1.0)),
        enabled: input.enabled.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };

    let created = state.storage.create_channel_model(&ctx.org_id, &cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ChannelModelResponse::from(created)))
}

pub async fn list_channel_models(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, provider_id)): Path<(String, String)>,
) -> Result<Json<Vec<ChannelModelResponse>>, ApiError> {
    // Get channels for provider, then channel_models for those channels
    let mut channels = state.storage.list_channels_by_provider(&ctx.org_id, &provider_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if !can_manage_channels(&ctx) {
        channels.retain(|c| can_access_channel(&ctx, c.group_id.as_deref()));
    }

    let channel_ids: Vec<String> = channels.iter().map(|c| c.id.clone()).collect();

    // Get all channel_models for these channels
    let mut all_models = Vec::new();
    for channel_id in channel_ids {
        let cms = state.storage.list_channel_models_by_channel(&ctx.org_id, &channel_id).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        all_models.extend(cms);
    }

    Ok(Json(all_models.into_iter().map(ChannelModelResponse::from).collect()))
}

pub async fn get_channel_model(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<Json<ChannelModelResponse>, ApiError> {
    let cm = state.storage.get_channel_model(&ctx.org_id, &id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("ChannelModel not found".to_string()))?;

    // Members can read a channel_model only if its parent channel is visible.
    if !can_manage_channels(&ctx) {
        let channel = state.storage.get_channel(&ctx.org_id, &cm.channel_id).await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or(ApiError::NotFound("ChannelModel not found".to_string()))?;
        if !can_access_channel(&ctx, channel.group_id.as_deref()) {
            return Err(ApiError::NotFound("ChannelModel not found".to_string()));
        }
    }

    Ok(Json(ChannelModelResponse::from(cm)))
}

pub async fn update_channel_model(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
    Json(input): Json<UpdateChannelModelRequest>,
) -> Result<Json<ChannelModelResponse>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let mut cm = state.storage.get_channel_model(&ctx.org_id, &id).await
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
        cm.markup_ratio = ratio_to_bps(markup_ratio);
    }
    if let Some(enabled) = input.enabled {
        cm.enabled = enabled;
    }
    cm.updated_at = chrono::Utc::now();

    let updated = state.storage.update_channel_model(&ctx.org_id, &cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ChannelModelResponse::from(updated)))
}

pub async fn delete_channel_model(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<axum::http::StatusCode, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }

    state.storage.delete_channel_model(&ctx.org_id, &id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_channel_models_by_channel(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, channel_id)): Path<(String, String)>,
) -> Result<Json<Vec<ChannelModelResponse>>, ApiError> {
    // Verify the parent channel exists and the caller can read it.
    let channel = state.storage.get_channel(&ctx.org_id, &channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Channel not found".to_string()))?;
    if !can_access_channel(&ctx, channel.group_id.as_deref()) {
        return Err(ApiError::NotFound("Channel not found".to_string()));
    }

    let models = state.storage.list_channel_models_by_channel(&ctx.org_id, &channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(models.into_iter().map(ChannelModelResponse::from).collect()))
}
