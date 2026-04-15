use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::StreamExt;
use std::sync::Arc;
use serde_json::{json, Value};
use std::time::Instant;

use llm_gateway_auth::hash_api_key;
use llm_gateway_billing::{PricingCalculator, Usage};
use llm_gateway_encryption::decrypt;
use llm_gateway_provider::openai::OpenAiProvider;
use llm_gateway_provider::Provider;
use llm_gateway_storage::{ApiKey, Channel, Protocol, UsageRecord};

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;

/// POST /v1/chat/completions — proxy to upstream OpenAI-compatible provider.

#[allow(clippy::too_many_arguments)]
async fn record_stream_usage(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    audit_logger: Arc<llm_gateway_audit::AuditLogger>,
    key_id: String,
    body: String,
    model_name: String,
    provider_id: String,
    channel_id: String,
    protocol: Protocol,
    response_desc: &str,
    status_code: i32,
    latency_ms: i64,
    pricing_policy_id: Option<String>,
    model_input_price: f64,
    model_output_price: f64,
    model_request_price: f64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
) {
    // Use PricingCalculator
    let usage = Usage::from_tokens(input_tokens, output_tokens, 1);
    let calculator = PricingCalculator;

    // Get policy from storage or fallback to legacy pricing
    let cost = if let Some(policy_id) = &pricing_policy_id {
        match storage.get_pricing_policy(policy_id).await {
            Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
            _ => 0.0,
        }
    } else {
        // Fallback to legacy pricing if no policy (backwards compat)
        llm_gateway_billing::calculate_cost(
            &llm_gateway_storage::BillingType::Token, // default to token
            input_tokens,
            output_tokens,
            model_input_price,
            model_output_price,
            model_request_price,
        ).cost
    };
    let usage = UsageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        key_id,
        model_name,
        provider_id,
        channel_id: Some(channel_id),
        protocol: protocol.clone(),
        input_tokens,
        output_tokens,
        cost,
        created_at: chrono::Utc::now(),
    };
    let _ = storage.record_usage(&usage).await;
    let _ = audit_logger.log_request(
        &usage.key_id, &usage.model_name, &usage.provider_id,
        protocol, &body, response_desc, status_code,
        latency_ms, usage.input_tokens, usage.output_tokens,
    ).await;
}

pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, ApiError> {
    // 1. Extract bearer token, hash it, look up API key
    let raw_token = extract_bearer_token(&headers)?;
    let token_hash = hash_api_key(&raw_token);
    let api_key = state
        .storage
        .get_key_by_hash(&token_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    // 2. Check key.enabled
    if !api_key.enabled {
        return Err(ApiError::Forbidden);
    }

    // 3. Parse model from request JSON body
    let req_json: Value = serde_json::from_str(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;
    let model_name = req_json
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or(ApiError::BadRequest("Missing 'model' field".to_string()))?
        .to_string();

    // 4. Check rate limits (global key level)
    let (rpm_limit, tpm_limit) = get_rate_limits(&state, &api_key, &model_name, None).await;

    // Check per-key-per-model rate limit
    let allowed = state
        .rate_limiter
        .check_and_increment(
            &api_key.id,
            &model_name,
            rpm_limit,
            tpm_limit,
            None,
        )
        .await;

    if !allowed {
        return Err(ApiError::RateLimited);
    }

    // Also check global key rate limit if set
    if let Some(global_rpm) = api_key.rate_limit {
        let global_allowed = state
            .rate_limiter
            .check_and_increment(
                &api_key.id,
                "__global__",
                Some(global_rpm),
                None,
                None,
            )
            .await;
        if !global_allowed {
            return Err(ApiError::RateLimited);
        }
    }

    // 5. Find model in storage -> get provider
    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let model_entry = models
        .iter()
        .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase() && m.openai_compatible && m.model.enabled)
        .ok_or(ApiError::NotFound(format!("Model '{}' not found or not available via OpenAI", model_name)))?;

    let provider_id = &model_entry.model.provider_id;
    let provider = state
        .storage
        .get_provider(provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Internal(format!("Provider '{}' not found", provider_id)))?;

    if !provider.enabled {
        return Err(ApiError::NotFound(format!("Provider '{}' is disabled", provider_id)));
    }

    // Get channels that support this model (via channel_models)
    // If no mapping exists, fall back to all provider channels
    let channels = match state.storage.get_channels_for_model(&model_name).await {
        Ok(channels) if !channels.is_empty() => channels,
        _ => {
            // Fallback: use all enabled channels for provider
            state.storage.list_enabled_channels_by_provider(provider_id).await
                .map_err(|e| ApiError::Internal(e.to_string()))?
        }
    };
    if channels.is_empty() {
        return Err(ApiError::Internal(format!("Provider '{}' has no enabled channels", provider_id)));
    }

    // Get channel_models to find upstream_model_name
    let channel_models = state.storage.get_channel_models_for_model(&model_name).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // 7. Check for streaming
    let is_stream = req_json
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    let client = reqwest::Client::new();

    if is_stream {
        let start = Instant::now();

        // Failover loop over channels
        let mut last_error = String::new();
        for channel in &channels {
            // Get upstream_model_name for this channel
            let upstream_name = channel_models
                .iter()
                .find(|cm| cm.channel_id == channel.id)
                .map(|cm| cm.upstream_model_name.as_str())
                .unwrap_or(&model_name);

            // Create modified body with upstream model name
            let modified_body = if upstream_name != &model_name {
                let mut req_json_modified = req_json.clone();
                if let Some(model_obj) = req_json_modified.get_mut("model") {
                    *model_obj = serde_json::Value::String(upstream_name.to_string());
                }
                serde_json::to_string(&req_json_modified).unwrap_or_else(|_| body.clone())
            } else {
                body.clone()
            };

            // Decrypt the channel's api_key for the request
            let api_key_value = decrypt(&channel.api_key, &state.encryption_key)
                .unwrap_or_else(|_| channel.api_key.clone());

            // Get base_url: first try channel, then provider's endpoints JSON, then fallback to provider base_url
            let base_url = if let Some(url) = channel.base_url.as_deref() {
                url.to_string()
            } else {
                // Parse provider endpoints JSON to find OpenAI endpoint
                let endpoints: serde_json::Value = provider
                    .endpoints
                    .as_ref()
                    .and_then(|e| serde_json::from_str(e).ok())
                    .unwrap_or(serde_json::Value::Null);
                endpoints
                    .get("openai")
                    .and_then(|v| v.as_str())
                    .or(provider.base_url.as_deref())
                    .ok_or_else(|| ApiError::Internal(format!("Provider '{}' has no OpenAI endpoint", provider_id)))?
                    .to_string()
            };

            let url = format!("{}/v1/chat/completions", base_url);
            let upstream_resp = match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key_value))
                .header("Content-Type", "application/json")
                .body(modified_body)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = format!("Connection error on channel '{}': {}", channel.name, e);
                    continue;
                }
            };

            let status = upstream_resp.status().as_u16();

            // 4xx: client error, return immediately (no failover)
            if status != 200 && status < 500 {
                let error_body = upstream_resp.text().await.unwrap_or_default();
                return Ok((
                    StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
                    error_body,
                ).into_response());
            }

            // 5xx: server error, try next channel
            if status != 200 && status >= 500 {
                last_error = format!("Server error {} on channel '{}'", status, channel.name);
                continue;
            }

            // status == 200: forward the SSE stream
            let byte_stream = upstream_resp.bytes_stream();
            let storage_clone = state.storage.clone();
            let audit_logger_clone = state.audit_logger.clone();
            let key_id = api_key.id.clone();
            let provider_id = provider.id.clone();
            let channel_id = channel.id.clone();
            let model_name_clone = model_name.clone();
            let pricing_policy_id = model_entry.model.pricing_policy_id.clone();
            let model_input_price = model_entry.model.input_price;
            let model_output_price = model_entry.model.output_price;
            let model_request_price = model_entry.model.request_price;

            let event_stream = futures::stream::unfold(
                (byte_stream, String::new(), None as Option<i64>, None as Option<i64>),
                move |(mut byte_stream, mut buffer, mut input_tokens, mut output_tokens)| {
                    let storage = storage_clone.clone();
                    let audit_logger = audit_logger_clone.clone();
                    let key_id = key_id.clone();
                    let body = body.clone();
                    let provider_id = provider_id.clone();
                    let channel_id = channel_id.clone();
                    let model_name = model_name_clone.clone();
                    let pricing_policy_id = pricing_policy_id.clone();
                    let start = start;
                    let model_input_price = model_input_price;
                    let model_output_price = model_output_price;
                    let model_request_price = model_request_price;
                    async move {
                        loop {
                            // Try to extract a complete SSE event from buffer
                            while let Some(pos) = buffer.find("\n\n") {
                                let event_text = buffer[..pos].to_string();
                                buffer = buffer[pos + 2..].to_string();

                                // Parse "data: ..." lines
                                for line in event_text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if data.trim() == "[DONE]" {
                                            // Stream finished — spawn audit + billing
                                            let latency_ms = start.elapsed().as_millis() as i64;
                                            tokio::spawn(async move {
                                                // Use PricingCalculator
                                                let usage = Usage::from_tokens(
                                                    Some(input_tokens.unwrap_or(0)),
                                                    Some(output_tokens.unwrap_or(0)),
                                                    1,
                                                );
                                                let calculator = PricingCalculator;

                                                // Get policy from storage or fallback to legacy pricing
                                                let cost = if let Some(policy_id) = &pricing_policy_id {
                                                    match storage.get_pricing_policy(policy_id).await {
                                                        Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
                                                        _ => 0.0,
                                                    }
                                                } else {
                                                    // Fallback to legacy pricing if no policy (backwards compat)
                                                    llm_gateway_billing::calculate_cost(
                                                        &llm_gateway_storage::BillingType::Token, // default to token
                                                        input_tokens,
                                                        output_tokens,
                                                        model_input_price,
                                                        model_output_price,
                                                        model_request_price,
                                                    ).cost
                                                };
                                                let usage = UsageRecord {
                                                    id: uuid::Uuid::new_v4().to_string(),
                                                    key_id: key_id.clone(),
                                                    model_name: model_name.clone(),
                                                    provider_id: provider_id.clone(),
                                                    channel_id: Some(channel_id.clone()),
                                                    protocol: Protocol::Openai,
                                                    input_tokens,
                                                    output_tokens,
                                                    cost,
                                                    created_at: chrono::Utc::now(),
                                                };
                                                let _ = storage.record_usage(&usage).await;
                                                let _ = audit_logger
                                                    .log_request(
                                                        &key_id,
                                                        &model_name,
                                                        &provider_id,
                                                        Protocol::Openai,
                                                        &body,
                                                        "[stream]",
                                                        200,
                                                        latency_ms,
                                                        input_tokens,
                                                        output_tokens,
                                                    )
                                                    .await;
                                            });
                                            return None;
                                        }
                                        // Try to extract usage from this event
                                        if let Ok(v) = serde_json::from_str::<Value>(data) {
                                            if let Some(usage) = v.get("usage") {
                                                input_tokens = usage
                                                    .get("prompt_tokens")
                                                    .and_then(|t| t.as_i64());
                                                output_tokens = usage
                                                    .get("completion_tokens")
                                                    .and_then(|t| t.as_i64());
                                            }
                                        }
                                        // Forward to client
                                        let event = Event::default().data(data.to_string());
                                        return Some((Ok::<_, std::convert::Infallible>(event), (byte_stream, buffer, input_tokens, output_tokens)));
                                    }
                                }
                            }

                            // Read more bytes from upstream
                            match byte_stream.next().await {
                                Some(Ok(bytes)) => {
                                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                                }
                                Some(Err(_)) => {
                                    tokio::spawn(record_stream_usage(
                                        storage.clone(), audit_logger.clone(),
                                        key_id.clone(), body.clone(), model_name.clone(),
                                        provider_id.clone(), channel_id.clone(),
                                        Protocol::Openai, "[stream truncated]", 0,
                                        start.elapsed().as_millis() as i64,
                                        pricing_policy_id.clone(),
                                        model_input_price, model_output_price, model_request_price,
                                        input_tokens, output_tokens,
                                    ));
                                    return None;
                                }
                                None => {
                                    tokio::spawn(record_stream_usage(
                                        storage.clone(), audit_logger.clone(),
                                        key_id.clone(), body.clone(), model_name.clone(),
                                        provider_id.clone(), channel_id.clone(),
                                        Protocol::Openai, "[stream incomplete]", 0,
                                        start.elapsed().as_millis() as i64,
                                        pricing_policy_id.clone(),
                                        model_input_price, model_output_price, model_request_price,
                                        input_tokens, output_tokens,
                                    ));
                                    return None;
                                }
                            }
                        }
                    }
                },
            );

            let sse = Sse::new(event_stream).keep_alive(KeepAlive::default());
            return Ok(sse.into_response());
        }

        // All channels exhausted
        return Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)));
    }

    // 8. Non-streaming: failover loop over channels
    let mut last_error = String::new();
    for channel in &channels {
        // Get upstream_model_name for this channel
        let upstream_name = channel_models
            .iter()
            .find(|cm| cm.channel_id == channel.id)
            .map(|cm| cm.upstream_model_name.as_str())
            .unwrap_or(&model_name);

        // Create modified body with upstream model name
        let modified_body = if upstream_name != &model_name {
            let mut req_json_modified = req_json.clone();
            if let Some(model_obj) = req_json_modified.get_mut("model") {
                *model_obj = serde_json::Value::String(upstream_name.to_string());
            }
            serde_json::to_string(&req_json_modified).unwrap_or_else(|_| body.clone())
        } else {
            body.clone()
        };

        // Decrypt the channel's api_key for the request
        let api_key_value = decrypt(&channel.api_key, &state.encryption_key)
            .unwrap_or_else(|_| channel.api_key.clone());

        let base_url = match channel.base_url.as_deref() {
            Some(url) => url.to_string(),
            None => {
                // Try to get endpoint from JSON, fall back to base_url
                let endpoints: serde_json::Value = provider
                    .endpoints
                    .as_ref()
                    .and_then(|e| serde_json::from_str(e).ok())
                    .unwrap_or(serde_json::Value::Null);
                endpoints
                    .get("openai")
                    .and_then(|v| v.as_str())
                    .or(provider.base_url.as_deref())
                    .ok_or_else(|| ApiError::Internal(format!("Provider '{}' has no OpenAI endpoint", provider_id)))?
                    .to_string()
            }
        };

        let openai_provider = OpenAiProvider {
            name: provider.name.clone(),
            base_url,
            api_key: api_key_value.clone(),
        };

        let start = Instant::now();
        let proxy_result = match openai_provider
            .proxy_request(&client, "/v1/chat/completions", modified_body, vec![])
            .await
        {
            Ok(result) => result,
            Err(e) => {
                last_error = format!("Connection error on channel '{}': {}", channel.name, e);
                continue;
            }
        };

        let latency_ms = start.elapsed().as_millis() as i64;

        // 5xx or status 0: server error, try next channel
        if proxy_result.status_code >= 500 || proxy_result.status_code == 0 {
            last_error = format!(
                "Server error {} on channel '{}': {}",
                proxy_result.status_code, channel.name, proxy_result.response_body
            );
            continue;
        }

        // 2xx or 4xx: return to client (no failover on client errors)
        let channel_id = channel.id.clone();
        let storage = state.storage.clone();
        let audit_logger = state.audit_logger.clone();
        let key_id = api_key.id.clone();
        let provider_id = provider.id.clone();
        let model_name_clone = model_name.clone();
        let status_code = proxy_result.status_code;
        let response_body = proxy_result.response_body.clone();
        let input_tokens = proxy_result.input_tokens;
        let output_tokens = proxy_result.output_tokens;
        let pricing_policy_id = model_entry.model.pricing_policy_id.clone();
        let model_input_price = model_entry.model.input_price;
        let model_output_price = model_entry.model.output_price;
        let model_request_price = model_entry.model.request_price;

        tokio::spawn(async move {
            // Use PricingCalculator
            let usage = Usage::from_tokens(
                Some(input_tokens.unwrap_or(0)),
                Some(output_tokens.unwrap_or(0)),
                1,
            );
            let calculator = PricingCalculator;

            // Get policy from storage or fallback to legacy pricing
            let cost = if let Some(policy_id) = &pricing_policy_id {
                match storage.get_pricing_policy(policy_id).await {
                    Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
                    _ => 0.0,
                }
            } else {
                // Fallback to legacy pricing if no policy (backwards compat)
                llm_gateway_billing::calculate_cost(
                    &llm_gateway_storage::BillingType::Token, // default to token
                    input_tokens,
                    output_tokens,
                    model_input_price,
                    model_output_price,
                    model_request_price,
                ).cost
            };

            // Record usage
            let usage = UsageRecord {
                id: uuid::Uuid::new_v4().to_string(),
                key_id: key_id.clone(),
                model_name: model_name_clone.clone(),
                provider_id: provider_id.clone(),
                channel_id: Some(channel_id.clone()),
                protocol: Protocol::Openai,
                input_tokens,
                output_tokens,
                cost,
                created_at: chrono::Utc::now(),
            };
            let _ = storage.record_usage(&usage).await;

            // Write audit log
            let _ = audit_logger
                .log_request(
                    &key_id,
                    &model_name_clone,
                    &provider_id,
                    Protocol::Openai,
                    &body,
                    &response_body,
                    status_code as i32,
                    latency_ms,
                    input_tokens,
                    output_tokens,
                )
                .await;
        });

        // Return response to client
        let status = StatusCode::from_u16(proxy_result.status_code)
            .unwrap_or(StatusCode::BAD_GATEWAY);
        return Ok((status, proxy_result.response_body).into_response());
    }

    // All channels exhausted
    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
}

/// GET /v1/models — list models available via OpenAI-compatible providers.
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<axum::Json<Value>, ApiError> {
    // Auth check
    let raw_token = extract_bearer_token(&headers)?;
    let token_hash = hash_api_key(&raw_token);
    let _api_key = state
        .storage
        .get_key_by_hash(&token_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let openai_models: Vec<Value> = models
        .iter()
        .filter(|m| m.openai_compatible)
        .map(|m| {
            json!({
                "id": m.model.name,
                "object": "model",
                "created": m.model.created_at.timestamp(),
                "owned_by": m.provider_name,
            })
        })
        .collect();

    Ok(axum::Json(json!({
        "object": "list",
        "data": openai_models,
    })))
}

/// Helper to determine rate limits for a key/model pair.
/// Checks channel-level limits first, then falls back to key-level limits.
async fn get_rate_limits(
    state: &Arc<AppState>,
    api_key: &ApiKey,
    model_name: &str,
    channel: Option<&Channel>,
) -> (Option<i64>, Option<i64>) {
    // 1. Check channel-level limits first
    if let Some(ch) = channel {
        if ch.rpm_limit.is_some() || ch.tpm_limit.is_some() {
            return (ch.rpm_limit, ch.tpm_limit);
        }
    }
    // 2. Fall back to key-level limits
    if let Ok(Some(limit)) = state.storage.get_key_model_rate_limit(&api_key.id, model_name).await {
        (Some(limit.rpm), Some(limit.tpm))
    } else {
        (api_key.rate_limit, None)
    }
}
