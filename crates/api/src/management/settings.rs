use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub allow_registration: bool,
}

#[derive(serde::Serialize)]
pub struct SettingsResponse {
    pub allow_registration: bool,
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let value = state.storage.get_setting("allow_registration").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let allow_registration = value.map(|v| v == "true").unwrap_or(true);
    Ok(Json(SettingsResponse { allow_registration }))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    state.storage.set_setting("allow_registration", if input.allow_registration { "true" } else { "false" })
        .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(SettingsResponse { allow_registration: input.allow_registration }))
}
