use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;

use llm_gateway_auth::hash_api_key;
use llm_gateway_storage::ModelWithProvider;

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;

/// GET /v1/models — list models available via enabled channels
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    // Auth check - any valid key can list models
    let raw_token = extract_bearer_token(&headers)?;
    let token_hash = hash_api_key(&raw_token);
    let _api_key = state
        .storage
        .get_key_by_hash(&token_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    // Get all models with their providers
    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Filter to only models that have at least one enabled ChannelModel
    let mut result: Vec<ModelWithProvider> = Vec::new();
    for m in models {
        // Check if model has any channel models
        let channel_models = state
            .storage
            .get_channel_models_for_model(&m.model.id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        if !channel_models.is_empty() {
            result.push(m);
        }
    }

    // Convert to OpenAI format
    let openai_models: Vec<Value> = result
        .iter()
        .map(|m| {
            json!({
                "id": m.model.name,
                "object": "model",
                "created": m.model.created_at.timestamp(),
                "owned_by": m.provider_name,
            })
        })
        .collect();

    Ok(Json(json!({
        "object": "list",
        "data": openai_models,
    })))
}