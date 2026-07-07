use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::AppState;
use llm_gateway_org::OrgContext;

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub allow_registration: Option<bool>,
    pub server_host: Option<String>,
    pub audit_log_request: Option<bool>,
    pub audit_log_response: Option<bool>,
    pub currency: Option<String>,
}

#[derive(serde::Serialize)]
pub struct SettingsResponse {
    pub allow_registration: bool,
    pub server_host: String,
    pub audit_log_request: bool,
    pub audit_log_response: bool,
    pub currency: String,
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<SettingsResponse>, ApiError> {
    // Phase 1: all settings exposed at /admin/settings are platform-level
    // (allow_registration, server_host, currency, audit toggles). Restrict to
    // platform_admin. Org-scoped settings will land in Phase 2 via a new
    // /api/v1/{org_slug}/settings route pair.
    if !ctx.is_platform_admin() {
        return Err(ApiError::Forbidden);
    }

    let allow_reg = state.storage.get_platform_setting("allow_registration").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let server_host = state.storage.get_platform_setting("server_host").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_req = state.storage.get_platform_setting("audit_log_request").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_res = state.storage.get_platform_setting("audit_log_response").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let currency = state.storage.get_platform_setting("currency").await.map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(SettingsResponse {
        allow_registration: allow_reg.map(|v| v == "true").unwrap_or(true),
        server_host: server_host.unwrap_or_default(),
        audit_log_request: audit_req.map(|v| v == "true").unwrap_or(true),
        audit_log_response: audit_res.map(|v| v == "true").unwrap_or(true),
        currency: currency.unwrap_or_else(|| "USD".to_string()),
    }))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, ApiError> {
    if !ctx.is_platform_admin() {
        return Err(ApiError::Forbidden);
    }

    if let Some(ar) = input.allow_registration {
        state.storage.set_platform_setting("allow_registration", if ar { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(sh) = input.server_host {
        state.storage.set_platform_setting("server_host", &sh)
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(alr) = input.audit_log_request {
        state.storage.set_platform_setting("audit_log_request", if alr { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(alp) = input.audit_log_response {
        state.storage.set_platform_setting("audit_log_response", if alp { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(c) = input.currency {
        let c = c.to_uppercase();
        if c != "USD" && c != "CNY" {
            return Err(ApiError::BadRequest("Currency must be USD or CNY".to_string()));
        }
        state.storage.set_platform_setting("currency", &c)
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Return updated settings
    let allow_reg = state.storage.get_platform_setting("allow_registration").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let server_host = state.storage.get_platform_setting("server_host").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_req = state.storage.get_platform_setting("audit_log_request").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_res = state.storage.get_platform_setting("audit_log_response").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let currency = state.storage.get_platform_setting("currency").await.map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(SettingsResponse {
        allow_registration: allow_reg.map(|v| v == "true").unwrap_or(true),
        server_host: server_host.unwrap_or_default(),
        audit_log_request: audit_req.map(|v| v == "true").unwrap_or(true),
        audit_log_response: audit_res.map(|v| v == "true").unwrap_or(true),
        currency: currency.unwrap_or_else(|| "USD".to_string()),
    }))
}
