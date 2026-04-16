use axum::http::HeaderMap;
use llm_gateway_storage::{SeedData, get_available_providers, get_available_models};

use crate::error::ApiError;
use crate::extractors::require_admin;

/// Get available seed providers/models for import
pub async fn get_seed_data(
    headers: HeaderMap,
) -> Result<axum::Json<SeedData>, ApiError> {
    // This endpoint doesn't need admin auth - it's just reading static JSON

    let providers = get_available_providers();
    let models = get_available_models();

    Ok(axum::Json(SeedData { providers, models }))
}