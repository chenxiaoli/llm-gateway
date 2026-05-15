use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use llm_gateway_encryption::{decrypt, encrypt};
use llm_gateway_storage::{
    bps_to_ratio, opt_units_to_usd, opt_usd_to_units, ratio_to_bps,
    Channel, ChannelModel, TimeSlot, UpdateChannelApiKey,
};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

/// Summary of a channel model with model name resolved from the models table.
#[derive(Debug, serde::Serialize)]
pub struct ChannelModelInfo {
    pub id: String,
    pub model_id: String,
    pub model_name: String,
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub enabled: bool,
}

/// Channel with its associated models (resolved model names included).
#[derive(Debug, serde::Serialize)]
pub struct ChannelWithModels {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<f64>,
    pub weight: Option<i32>,
    pub enabled: bool,
    pub available_hours: Option<Vec<TimeSlot>>,
    pub group: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub models: Vec<ChannelModelInfo>,
}

/// Channel response for JSON output (f64 for monetary/markup fields).
#[derive(Debug, serde::Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<f64>,
    pub weight: Option<i32>,
    pub enabled: bool,
    pub available_hours: Option<Vec<TimeSlot>>,
    pub created_by: Option<String>,
    pub group: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Channel> for ChannelResponse {
    fn from(c: Channel) -> Self {
        ChannelResponse {
            id: c.id,
            provider_id: c.provider_id,
            name: c.name,
            api_key: c.api_key,
            priority: c.priority,
            pricing_policy_id: c.pricing_policy_id,
            markup_ratio: bps_to_ratio(c.markup_ratio),
            rpm_limit: c.rpm_limit,
            tpm_limit: c.tpm_limit,
            balance: opt_units_to_usd(c.balance),
            weight: c.weight,
            enabled: c.enabled,
            available_hours: c.available_hours,
            created_by: c.created_by,
            group: c.group,
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

// --- JSON request structs (f64 for API boundary) ---

#[derive(Debug, Deserialize)]
pub struct CreateChannelModelInput {
    pub model_id: String,
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: Option<f64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: Option<f64>,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<f64>,
    pub weight: Option<i32>,
    pub enabled: Option<bool>,
    pub available_hours: Option<Vec<TimeSlot>>,
    pub models: Option<Vec<CreateChannelModelInput>>,
    pub group: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<Option<String>>,
    pub markup_ratio: Option<f64>,
    pub enabled: Option<bool>,
    pub rpm_limit: Option<Option<i64>>,
    pub tpm_limit: Option<Option<i64>>,
    pub balance: Option<Option<f64>>,
    pub weight: Option<Option<i32>>,
    pub available_hours: Option<Option<Vec<TimeSlot>>>,
    pub group: Option<Option<String>>,
}

pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let claims = require_admin(&headers, &state.jwt_secret)?;

    let provider_id = input.provider_id.clone();
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Channel name must not be empty".to_string()));
    }
    if name.len() > 100 {
        return Err(ApiError::BadRequest("Channel name must be at most 100 characters".to_string()));
    }

    state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let now = chrono::Utc::now();
    let encrypted_key = encrypt(&input.api_key, &state.encryption_key)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let channel = Channel {
        id: uuid::Uuid::new_v4().to_string(),
        provider_id,
        name,
        api_key: encrypted_key,
        priority: input.priority.unwrap_or(0),
        pricing_policy_id: input.pricing_policy_id,
        markup_ratio: ratio_to_bps(input.markup_ratio.unwrap_or(1.0)),
        rpm_limit: input.rpm_limit,
        tpm_limit: input.tpm_limit,
        balance: opt_usd_to_units(input.balance),
        weight: input.weight,
        enabled: input.enabled.unwrap_or(true),
        available_hours: input.available_hours,
        created_by: Some(claims.sub),
        group: input.group,
        created_at: now,
        updated_at: now,
    };

    let models: Vec<ChannelModel> = input
        .models
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let now = chrono::Utc::now();
            ChannelModel {
                id: uuid::Uuid::new_v4().to_string(),
                channel_id: channel.id.clone(),
                model_id: m.model_id.clone(),
                upstream_model_name: m.upstream_model_name.clone(),
                priority_override: m.priority_override,
                pricing_policy_id: m.pricing_policy_id.clone(),
                markup_ratio: ratio_to_bps(m.markup_ratio.unwrap_or(1.0)),
                enabled: m.enabled.unwrap_or(true),
                created_at: now,
                updated_at: now,
            }
        })
        .collect();

    // Validate all models exist before creating anything
    for m in &models {
        state
            .storage
            .get_model_by_id(&m.model_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or(ApiError::NotFound(format!("Model '{}' not found", m.model_id)))?;
    }

    let created = state
        .storage
        .create_channel_with_models(&channel, models)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ChannelResponse::from(created)))
}

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<ChannelResponse>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channels = state
        .storage
        .list_channels_by_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(channels.into_iter().map(ChannelResponse::from).collect()))
}

pub async fn list_all_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ChannelWithModels>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channels = state
        .storage
        .list_channels()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Batch-fetch all channel models and all models (for name resolution)
    let all_cms = state
        .storage
        .list_channel_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let all_models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build model_id -> model_name lookup map
    let model_name_map: HashMap<String, String> = all_models
        .into_iter()
        .map(|m| (m.model.id.clone(), m.model.name.clone()))
        .collect();

    // Group channel models by channel_id
    let mut cms_by_channel: HashMap<String, Vec<ChannelModel>> = HashMap::new();
    for cm in all_cms {
        cms_by_channel.entry(cm.channel_id.clone()).or_default().push(cm);
    }

    let result: Vec<ChannelWithModels> = channels
        .into_iter()
        .map(|c| {
            let api_key = decrypt(&c.api_key, &state.encryption_key).unwrap_or_else(|_| c.api_key.clone());
            let channel_id = c.id.clone();
            let models: Vec<ChannelModelInfo> = cms_by_channel
                .remove(&channel_id)
                .unwrap_or_default()
                .into_iter()
                .map(|cm| ChannelModelInfo {
                    id: cm.id,
                    model_id: cm.model_id.clone(),
                    model_name: model_name_map.get(&cm.model_id).cloned().unwrap_or_else(|| cm.model_id.clone()),
                    upstream_model_name: cm.upstream_model_name,
                    priority_override: cm.priority_override,
                    pricing_policy_id: cm.pricing_policy_id,
                    markup_ratio: bps_to_ratio(cm.markup_ratio),
                    enabled: cm.enabled,
                })
                .collect();
            ChannelWithModels {
                id: c.id,
                provider_id: c.provider_id,
                name: c.name,
                api_key,
                priority: c.priority,
                pricing_policy_id: c.pricing_policy_id,
                markup_ratio: bps_to_ratio(c.markup_ratio),
                rpm_limit: c.rpm_limit,
                tpm_limit: c.tpm_limit,
                balance: opt_units_to_usd(c.balance),
                weight: c.weight,
                enabled: c.enabled,
                available_hours: c.available_hours,
                group: c.group,
                created_at: c.created_at.to_rfc3339(),
                updated_at: c.updated_at.to_rfc3339(),
                models,
            }
        })
        .collect();

    Ok(Json(result))
}

pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ChannelResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    // Decrypt api_key for display
    channel.api_key = decrypt(&channel.api_key, &state.encryption_key)
        .unwrap_or_else(|_| channel.api_key);

    Ok(Json(ChannelResponse::from(channel)))
}

pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    if let Some(name) = input.name {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err(ApiError::BadRequest("Channel name must not be empty".to_string()));
        }
        if trimmed.len() > 100 {
            return Err(ApiError::BadRequest("Channel name must be at most 100 characters".to_string()));
        }
        channel.name = trimmed;
    }
    if let Some(priority) = input.priority {
        channel.priority = priority;
    }
    if let Some(enabled) = input.enabled {
        channel.enabled = enabled;
    }
    if let Some(rpm_limit) = input.rpm_limit {
        channel.rpm_limit = rpm_limit;
    }
    if let Some(tpm_limit) = input.tpm_limit {
        channel.tpm_limit = tpm_limit;
    }
    if let Some(markup_ratio) = input.markup_ratio {
        channel.markup_ratio = ratio_to_bps(markup_ratio);
    }
    if let Some(balance) = input.balance {
        channel.balance = opt_usd_to_units(balance);
    }
    if let Some(weight) = input.weight {
        channel.weight = weight;
    }
    if let Some(available_hours) = input.available_hours {
        channel.available_hours = available_hours;
    }
    if let Some(group) = input.group {
        channel.group = group;
    }
    channel.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_channel(&channel)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ChannelResponse::from(updated)))
}

/// Dedicated endpoint for updating a channel's API key.
/// Separated from general channel updates to prevent accidental key clearing.
pub async fn update_channel_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateChannelApiKey>,
) -> Result<Json<ChannelResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    if input.api_key.is_empty() {
        return Err(ApiError::BadRequest("API key must not be empty".to_string()));
    }

    channel.api_key = encrypt(&input.api_key, &state.encryption_key)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    channel.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_channel(&channel)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ChannelResponse::from(updated)))
}

pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .delete_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct TestChannelQuery {
    pub endpoint_key: Option<String>,
    pub stream: Option<bool>,
}

pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<TestChannelQuery>,
) -> Result<Json<llm_gateway_storage::ChannelTestResult>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    let provider = state
        .storage
        .get_provider(&channel.provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", channel.provider_id)))?;

    let channel_models = state
        .storage
        .list_channel_models_by_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let cm = channel_models
        .iter()
        .find(|cm| cm.enabled)
        .ok_or(ApiError::BadRequest("No enabled models on this channel".to_string()))?;

    let model_name = match &cm.upstream_model_name {
        Some(name) => name.clone(),
        None => state
            .storage
            .get_model_by_id(&cm.model_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .map(|m| m.name)
            .unwrap_or_else(|| cm.model_id.clone()),
    };

    let endpoints: serde_json::Value = provider
        .endpoints
        .and_then(|e| serde_json::from_str(&e).ok())
        .unwrap_or(serde_json::Value::Null);

    let endpoint_key = query.endpoint_key.as_deref().unwrap_or("openai");
    let base_url = endpoints
        .get(endpoint_key)
        .and_then(|v| v.as_str())
        .or_else(|| endpoints.get("default").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim_end_matches('/');

    let protocol = if endpoint_key == "anthropic" {
        crate::proxy::ProxyProtocol::Anthropic
    } else {
        crate::proxy::ProxyProtocol::OpenAI
    };
    let request_path = match protocol {
        crate::proxy::ProxyProtocol::OpenAI => "/v1/chat/completions",
        crate::proxy::ProxyProtocol::Anthropic => "/v1/messages",
    };
    let upstream_url = crate::proxy::build_upstream_url(base_url, request_path, protocol);
    let is_anthropic = matches!(protocol, crate::proxy::ProxyProtocol::Anthropic);

    let api_key = decrypt(&channel.api_key, &state.encryption_key)
        .unwrap_or_else(|_| channel.api_key.clone());

    let start = std::time::Instant::now();
    let client = reqwest::Client::new();

    let is_stream = query.stream.unwrap_or(false);

    let result = if is_anthropic {
        let mut body = serde_json::json!({
            "model": model_name,
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 5
        });
        if is_stream {
            body["stream"] = serde_json::json!(true);
        }
        let mut req = client
            .post(&upstream_url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(30));
        if is_stream {
            req = req.header("Accept", "text/event-stream");
        }
        req.json(&body).send().await
    } else {
        let mut body = serde_json::json!({
            "model": model_name,
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 5
        });
        if is_stream {
            body["stream"] = serde_json::json!(true);
        }
        let mut req = client
            .post(&upstream_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(30));
        if is_stream {
            req = req.header("Accept", "text/event-stream");
        }
        req.json(&body).send().await
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status();
            let status_code = status.as_u16();
            let body_text = resp.text().await.unwrap_or_else(|_| String::new());

            if status.is_success() {
                if is_stream {
                    // SSE responses are not JSON, just check first few lines
                    let preview = body_text.lines().take(5).collect::<Vec<_>>().join("\n");
                    Ok(Json(llm_gateway_storage::ChannelTestResult {
                        success: true,
                        latency_ms,
                        model: model_name.clone(),
                        error: None,
                        response_data: Some(if preview.len() < body_text.len() {
                            format!("{}...\n\n[{} total bytes]", preview, body_text.len())
                        } else {
                            preview
                        }),
                    }))
                } else {
                    // Parse JSON and check for top-level error field
                    if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                        if body_json.get("error").is_some() {
                            return Ok(Json(llm_gateway_storage::ChannelTestResult {
                                success: false,
                                latency_ms,
                                model: model_name.clone(),
                                error: Some(format!("{}: {}", status_code, body_text)),
                                response_data: Some(body_text),
                            }));
                        }
                    }
                    Ok(Json(llm_gateway_storage::ChannelTestResult {
                        success: true,
                        latency_ms,
                        model: model_name,
                        error: None,
                        response_data: Some(body_text),
                    }))
                }
            } else {
                Ok(Json(llm_gateway_storage::ChannelTestResult {
                    success: false,
                    latency_ms,
                    model: model_name,
                    error: Some(format!("{} {}", status_code, body_text)),
                    response_data: Some(body_text),
                }))
            }
        }
        Err(e) => Ok(Json(llm_gateway_storage::ChannelTestResult {
            success: false,
            latency_ms,
            model: model_name,
            error: Some(e.to_string()),
            response_data: None,
        })),
    }
}
