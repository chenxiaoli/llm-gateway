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
    pub server_host: Option<String>,
    pub audit_log_request: Option<bool>,
    pub audit_log_response: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct SettingsResponse {
    pub allow_registration: bool,
    pub server_host: String,
    pub audit_log_request: bool,
    pub audit_log_response: bool,
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let allow_reg = state.storage.get_setting("allow_registration").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let server_host = state.storage.get_setting("server_host").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_req = state.storage.get_setting("audit_log_request").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_res = state.storage.get_setting("audit_log_response").await.map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(SettingsResponse {
        allow_registration: allow_reg.map(|v| v == "true").unwrap_or(true),
        server_host: server_host.unwrap_or_default(),
        audit_log_request: audit_req.map(|v| v == "true").unwrap_or(true),
        audit_log_response: audit_res.map(|v| v == "true").unwrap_or(true),
    }))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    if let Some(ar) = Some(input.allow_registration) {
        state.storage.set_setting("allow_registration", if ar { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(sh) = input.server_host {
        state.storage.set_setting("server_host", &sh)
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(alr) = input.audit_log_request {
        state.storage.set_setting("audit_log_request", if alr { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(alp) = input.audit_log_response {
        state.storage.set_setting("audit_log_response", if alp { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Return updated settings
    let allow_reg = state.storage.get_setting("allow_registration").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let server_host = state.storage.get_setting("server_host").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_req = state.storage.get_setting("audit_log_request").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_res = state.storage.get_setting("audit_log_response").await.map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(SettingsResponse {
        allow_registration: allow_reg.map(|v| v == "true").unwrap_or(true),
        server_host: server_host.unwrap_or_default(),
        audit_log_request: audit_req.map(|v| v == "true").unwrap_or(true),
        audit_log_response: audit_res.map(|v| v == "true").unwrap_or(true),
    }))
}