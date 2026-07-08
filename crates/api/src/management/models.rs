use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use llm_gateway_org::{can_create_org_catalog, can_create_platform_catalog, can_mutate_catalog_entry, OrgContext};
use llm_gateway_storage::{Model, ProviderModel, ProviderModelInfo, UpdateModel};

use crate::error::ApiError;
use crate::AppState;

pub async fn list_all_models(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<llm_gateway_storage::ModelWithProvider>>, ApiError> {
    let models = state
        .storage
        .list_models(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(models))
}

#[derive(serde::Deserialize)]
pub struct CreateModelRequest {
    pub name: String,
    pub pricing_policy_id: Option<String>,
    pub owner_org_id: Option<String>,
}

pub async fn create_model_global(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(input): Json<CreateModelRequest>,
) -> Result<Json<Model>, ApiError> {
    // Decide ownership: explicit body value wins, otherwise non-platform-admin
    // callers get an org-private entry; platform_admin defaults to platform-level.
    let owner_org_id = input.owner_org_id.clone().or_else(|| {
        if can_create_platform_catalog(&ctx) { None } else { Some(ctx.org_id.clone()) }
    });
    if owner_org_id.as_deref() == Some(ctx.org_id.as_str()) {
        if !can_create_org_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else if owner_org_id.is_none() {
        if !can_create_platform_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else {
        return Err(ApiError::Forbidden);
    }

    let model = Model {
        id: input.name.clone(),
        owner_org_id,
        name: input.name,
        model_type: None,
        pricing_policy_id: input.pricing_policy_id,
        created_at: chrono::Utc::now(),
    };

    let created = state
        .storage
        .create_model(&ctx.org_id, &model)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn update_model(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, model_name)): Path<(String, String)>,
    Json(input): Json<UpdateModel>,
) -> Result<Json<Model>, ApiError> {
    let mut model = state
        .storage
        .get_model(&ctx.org_id, &model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?;

    if !can_mutate_catalog_entry(&ctx, model.owner_org_id.as_deref()) {
        return Err(ApiError::Forbidden);
    }

    // Apply partial updates
    if let Some(pricing_policy_id) = input.pricing_policy_id {
        model.pricing_policy_id = pricing_policy_id;
    }

    let updated = state
        .storage
        .update_model(&ctx.org_id, &model)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_model(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, model_name)): Path<(String, String)>,
) -> Result<axum::http::StatusCode, ApiError> {
    let model = state
        .storage
        .get_model(&ctx.org_id, &model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?;

    if !can_mutate_catalog_entry(&ctx, model.owner_org_id.as_deref()) {
        return Err(ApiError::Forbidden);
    }

    state
        .storage
        .delete_model(&ctx.org_id, &model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_provider_models(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, provider_id)): Path<(String, String)>,
) -> Result<Json<Vec<ProviderModelInfo>>, ApiError> {
    let models = state
        .storage
        .list_provider_models(&ctx.org_id, &provider_id)
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
    ctx: OrgContext,
    Path((_org_slug, provider_id)): Path<(String, String)>,
    Json(input): Json<UpdateProviderModelsRequest>,
) -> Result<Json<Vec<ProviderModelInfo>>, ApiError> {
    // Gate on catalog-write permission — provider_models are catalog data.
    if !can_create_org_catalog(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let models: Vec<ProviderModel> = input.models.into_iter().map(|m| ProviderModel {
        provider_id: provider_id.clone(),
        model_id: m.model_id,
        owner_org_id: Some(ctx.org_id.clone()),
        upstream_name: m.upstream_name,
        pricing_policy_id: m.pricing_policy_id,
        created_at: chrono::Utc::now(),
    }).collect();

    state
        .storage
        .set_provider_models(&ctx.org_id, &provider_id, models)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let result = state
        .storage
        .list_provider_models(&ctx.org_id, &provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(result))
}
