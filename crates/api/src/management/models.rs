use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{Model, UpdateModel as StorageUpdateModel};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Serialize)]
pub struct SyncModelsResponse {
    pub new: i32,
    pub updated: i32,
    pub models: Vec<SyncedModel>,
}

#[derive(Serialize)]
pub struct SyncedModel {
    pub name: String,
    pub model_type: Option<String>,
    pub created: bool,
}

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
    pub billing_type: Option<String>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
    pub enabled: Option<bool>,
}

pub async fn create_model_global(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateModelRequest>,
) -> Result<Json<Model>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let billing_type = input.billing_type.clone().unwrap_or_else(|| "per_token".to_string());

    let model = Model {
        id: input.name.clone(),
        name: input.name,
        model_type: None,
        pricing_policy_id: input.pricing_policy_id,
        billing_type,
        input_price: input.input_price.unwrap_or(0.0),
        output_price: input.output_price.unwrap_or(0.0),
        request_price: input.request_price.unwrap_or(0.0),
        enabled: input.enabled.unwrap_or(true),
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
    Json(input): Json<StorageUpdateModel>,
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
    if let Some(billing_type) = input.billing_type {
        model.billing_type = billing_type.unwrap_or_else(|| "per_token".to_string());
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

pub async fn sync_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<SyncModelsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let provider = state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    // Get channels to find API key
    let channels = state
        .storage
        .list_channels_by_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let api_key = channels
        .first()
        .ok_or_else(|| ApiError::NotFound("No channels found for provider".to_string()))?
        .api_key
        .clone();

    let client = reqwest::Client::new();
    let mut new_count = 0;
    let mut updated_count = 0;
    let mut synced_models: Vec<SyncedModel> = Vec::new();

    // Fetch from OpenAI-compatible endpoint
    let openai_endpoints: serde_json::Value = provider
        .endpoints
        .as_ref()
        .and_then(|e| serde_json::from_str(e).ok())
        .unwrap_or(serde_json::Value::Null);
    if let Some(base_url) = openai_endpoints
        .get("openai")
        .and_then(|v| v.as_str())
        .or(provider.base_url.as_deref())
    {
        let url = format!("{}/models", base_url.trim_end_matches('/'));
        match client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        for model_entry in data {
                            let name = match model_entry.get("id").and_then(|v| v.as_str()) {
                                Some(n) => n.to_string(),
                                None => continue,
                            };
                            let model_type = model_entry
                                .get("type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            // Check if model exists
                            let existing = state.storage.get_model(&name).await.map_err(|e| ApiError::Internal(e.to_string()))?;

                            if existing.is_some() {
                                updated_count += 1;
                                synced_models.push(SyncedModel {
                                    name,
                                    model_type,
                                    created: false,
                                });
                            } else {
                                // Create new model (disabled by default - requires admin review)
                                let model = Model {
                                    id: name.clone(),
                                    name: name.clone(),
                                    model_type: model_type.clone(),
                                    pricing_policy_id: None,
                                    billing_type: "per_token".to_string(),
                                    input_price: 0.0,
                                    output_price: 0.0,
                                    request_price: 0.0,
                                    enabled: false,
                                    created_at: chrono::Utc::now(),
                                };
                                let _ = state.storage.create_model(&model).await.map_err(|e| ApiError::Internal(e.to_string()));
                                new_count += 1;
                                synced_models.push(SyncedModel {
                                    name,
                                    model_type,
                                    created: true,
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from OpenAI endpoint: {}", e);
            }
        }
    }

    // Fetch from Anthropic endpoint
    let anthropic_endpoints: serde_json::Value = provider
        .endpoints
        .as_ref()
        .and_then(|e| serde_json::from_str(e).ok())
        .unwrap_or(serde_json::Value::Null);
    if let Some(base_url) = anthropic_endpoints
        .get("anthropic")
        .and_then(|v| v.as_str())
        .or(provider.base_url.as_deref())
    {
        let url = format!("{}/models", base_url.trim_end_matches('/'));
        match client
            .get(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        for model_entry in data {
                            let name = match model_entry.get("id").and_then(|v| v.as_str()) {
                                Some(n) => n.to_string(),
                                None => continue,
                            };
                            let model_type = model_entry
                                .get("type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            // Skip if already processed from OpenAI
                            if synced_models.iter().any(|m| m.name == name) {
                                continue;
                            }

                            // Check if model exists
                            let existing = state.storage.get_model(&name).await.map_err(|e| ApiError::Internal(e.to_string()))?;

                            if existing.is_some() {
                                updated_count += 1;
                                synced_models.push(SyncedModel {
                                    name,
                                    model_type,
                                    created: false,
                                });
                            } else {
                                // Create new model (disabled by default - requires admin review)
                                let model = Model {
                                    id: name.clone(),
                                    name: name.clone(),
                                    model_type: model_type.clone(),
                                    pricing_policy_id: None,
                                    billing_type: "per_token".to_string(),
                                    input_price: 0.0,
                                    output_price: 0.0,
                                    request_price: 0.0,
                                    enabled: false,
                                    created_at: chrono::Utc::now(),
                                };
                                let _ = state.storage.create_model(&model).await.map_err(|e| ApiError::Internal(e.to_string()));
                                new_count += 1;
                                synced_models.push(SyncedModel {
                                    name,
                                    model_type,
                                    created: true,
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from Anthropic endpoint: {}", e);
            }
        }
    }

    Ok(Json(SyncModelsResponse {
        new: new_count,
        updated: updated_count,
        models: synced_models,
    }))
}
