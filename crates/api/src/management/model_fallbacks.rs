use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use llm_gateway_org::OrgContext;
use llm_gateway_storage::{CreateModelFallback, ModelFallbackConfig, UpdateModelFallback};

use crate::error::ApiError;
use crate::AppState;

pub async fn create_model_fallback(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(input): Json<CreateModelFallback>,
) -> Result<Json<ModelFallbackConfig>, ApiError> {
    // model_fallbacks is platform-level config (not org-scoped).
    if !ctx.is_platform_admin() {
        return Err(ApiError::Forbidden);
    }

    let config = ModelFallbackConfig {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        config: input.config,
        created_by: Some(ctx.user_id.clone()),
        created_at: chrono::Utc::now(),
    };

    let created = state
        .storage
        .create_model_fallback(&config)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_model_fallbacks(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<ModelFallbackConfig>>, ApiError> {
    let all = state
        .storage
        .list_model_fallbacks()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let filtered = if ctx.is_platform_admin() {
        all
    } else {
        all.into_iter()
            .filter(|f| f.created_by.as_deref() == Some(ctx.user_id.as_str()))
            .collect()
    };

    Ok(Json(filtered))
}

pub async fn get_model_fallback(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<Json<ModelFallbackConfig>, ApiError> {
    let config = state
        .storage
        .get_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!(
            "Model fallback '{}' not found",
            id
        )))?;

    if !ctx.is_platform_admin() && config.created_by.as_deref() != Some(ctx.user_id.as_str()) {
        return Err(ApiError::NotFound(format!(
            "Model fallback '{}' not found",
            id
        )));
    }

    Ok(Json(config))
}

pub async fn update_model_fallback(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
    Json(input): Json<UpdateModelFallback>,
) -> Result<Json<ModelFallbackConfig>, ApiError> {
    let mut config = state
        .storage
        .get_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!(
            "Model fallback '{}' not found",
            id
        )))?;

    if !ctx.is_platform_admin() && config.created_by.as_deref() != Some(ctx.user_id.as_str()) {
        return Err(ApiError::NotFound(format!(
            "Model fallback '{}' not found",
            id
        )));
    }

    if let Some(name) = input.name {
        config.name = name;
    }
    if let Some(new_config) = input.config {
        config.config = new_config;
    }

    let updated = state
        .storage
        .update_model_fallback(&config)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_model_fallback(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<axum::http::StatusCode, ApiError> {
    let config = state
        .storage
        .get_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!(
            "Model fallback '{}' not found",
            id
        )))?;

    if !ctx.is_platform_admin() && config.created_by.as_deref() != Some(ctx.user_id.as_str()) {
        return Err(ApiError::NotFound(format!(
            "Model fallback '{}' not found",
            id
        )));
    }

    state
        .storage
        .delete_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
