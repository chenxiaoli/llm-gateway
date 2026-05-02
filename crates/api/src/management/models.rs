use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{Model, ProviderModel, ProviderModelInfo, UpdateModel};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn list_all_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<llm_gateway_storage::ModelWithProvider>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(models))
}

#[derive(serde::Deserialize)]
pub struct CreateModelRequest {
    pub name: String,
    pub pricing_policy_id: Option<String>,
}

pub async fn create_model_global(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateModelRequest>,
) -> Result<Json<Model>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let model = Model {
        id: input.name.clone(),
        name: input.name,
        model_type: None,
        pricing_policy_id: input.pricing_policy_id,
        created_at: chrono::Utc::now(),
    };

    let created = state
        .storage
        .create_model(&model)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn update_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(model_name): Path<String>,
    Json(input): Json<UpdateModel>,
) -> Result<Json<Model>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut model = state
        .storage
        .get_model(&model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?;

    // Apply partial updates
    if let Some(pricing_policy_id) = input.pricing_policy_id {
        model.pricing_policy_id = pricing_policy_id;
    }

    let updated = state
        .storage
        .update_model(&model)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(model_name): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Verify the model exists
    let _model = state
        .storage
        .get_model(&model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?;

    state
        .storage
        .delete_model(&model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_provider_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<ProviderModelInfo>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let models = state
        .storage
        .list_provider_models(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(models))
}

#[derive(serde::Deserialize)]
pub struct UpdateProviderModelsRequest {
    pub models: Vec<ProviderModelInput>,
}

#[derive(serde::Deserialize)]
pub struct ProviderModelInput {
    pub model_id: String,
    pub upstream_name: Option<String>,
    pub pricing_policy_id: Option<String>,
}

pub async fn update_provider_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(input): Json<UpdateProviderModelsRequest>,
) -> Result<Json<Vec<ProviderModelInfo>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let models: Vec<ProviderModel> = input.models.into_iter().map(|m| ProviderModel {
        provider_id: provider_id.clone(),
        model_id: m.model_id,
        upstream_name: m.upstream_name,
        pricing_policy_id: m.pricing_policy_id,
        created_at: chrono::Utc::now(),
    }).collect();

    state
        .storage
        .set_provider_models(&provider_id, models)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let result = state
        .storage
        .list_provider_models(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(result))
}
