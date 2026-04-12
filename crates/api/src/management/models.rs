use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{CreateModel as StorageCreateModel, Model, UpdateModel as StorageUpdateModel};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<Model>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let models = state
        .storage
        .list_models_by_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(models))
}

pub async fn create_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(input): Json<StorageCreateModel>,
) -> Result<Json<Model>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Verify provider exists
    let _provider = state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let model = Model {
        name: input.name,
        provider_id,
        billing_type: input.billing_type,
        input_price: input.input_price.unwrap_or(0.0),
        output_price: input.output_price.unwrap_or(0.0),
        request_price: input.request_price.unwrap_or(0.0),
        enabled: true,
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
    Path((provider_id, model_name)): Path<(String, String)>,
    Json(input): Json<StorageUpdateModel>,
) -> Result<Json<Model>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut model = state
        .storage
        .get_model(&model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?;

    // Verify the model belongs to the specified provider
    if model.provider_id != provider_id {
        return Err(ApiError::NotFound(format!(
            "Model '{}' not found under provider '{}'",
            model_name, provider_id
        )));
    }

    // Apply partial updates
    if let Some(billing_type) = input.billing_type {
        model.billing_type = billing_type;
    }
    if let Some(input_price) = input.input_price {
        model.input_price = input_price;
    }
    if let Some(output_price) = input.output_price {
        model.output_price = output_price;
    }
    if let Some(request_price) = input.request_price {
        model.request_price = request_price;
    }
    if let Some(enabled) = input.enabled {
        model.enabled = enabled;
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
    Path((provider_id, model_name)): Path<(String, String)>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Verify the model belongs to the specified provider
    let model = state
        .storage
        .get_model(&model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?;

    if model.provider_id != provider_id {
        return Err(ApiError::NotFound(format!(
            "Model '{}' not found under provider '{}'",
            model_name, provider_id
        )));
    }

    state
        .storage
        .delete_model(&model_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
