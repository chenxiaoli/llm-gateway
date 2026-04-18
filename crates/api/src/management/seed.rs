use llm_gateway_storage::{SeedData, get_available_providers, get_available_models};

use crate::error::ApiError;

/// Get available seed providers/models for import
pub async fn get_seed_data() -> Result<axum::Json<SeedData>, ApiError> {
    // This endpoint doesn't need admin auth - it's just reading static JSON

    let providers = get_available_providers();
    let models = get_available_models();

    Ok(axum::Json(SeedData { providers, models }))
}