use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::StreamExt;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

use llm_gateway_auth::hash_api_key;
use llm_gateway_billing::calculate_cost;
use llm_gateway_provider::anthropic::AnthropicProvider;
use llm_gateway_provider::Provider;
use llm_gateway_storage::{ApiKey, Protocol, UsageRecord};

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;

/// POST /v1/messages — proxy to upstream Anthropic-compatible provider.
pub async fn messages(
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

    // 4. Check rate limits
    let (rpm_limit, tpm_limit) = get_rate_limits(&state, &api_key, &model_name).await;

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
        .find(|m| m.model.name == model_name && m.anthropic_compatible && m.model.enabled)
        .ok_or(ApiError::NotFound(format!("Model '{}' not found or not available via Anthropic", model_name)))?;

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

    // Get first enabled channel for the provider
    let channels = state
        .storage
        .list_enabled_channels_by_provider(provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let channel = channels.into_iter().next()
        .ok_or(ApiError::Internal(format!("Provider '{}' has no enabled channels", provider_id)))?;

    // 6. Get provider's anthropic_base_url
    let base_url = provider
        .anthropic_base_url
        .ok_or(ApiError::Internal(format!("Provider '{}' has no anthropic_base_url", provider_id)))?;

    // 7. Check for streaming
    let is_stream = req_json
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    if is_stream {
        let start = Instant::now();
        let client = reqwest::Client::new();

        // Build upstream request — Anthropic uses x-api-key and anthropic-version headers
        let url = format!("{}/v1/messages", base_url);
        let upstream_resp = client
            .post(&url)
            .header("x-api-key", &channel.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| ApiError::UpstreamError(502, e.to_string()))?;

        let status = upstream_resp.status().as_u16();
        if status != 200 {
            let error_body = upstream_resp.text().await.unwrap_or_default();
            return Ok((
                StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
                error_body,
            ).into_response());
        }

        // Create SSE stream from upstream response
        let byte_stream = upstream_resp.bytes_stream();
        let storage_clone = state.storage.clone();
        let audit_logger_clone = state.audit_logger.clone();
        let key_id = api_key.id.clone();
        let provider_id = provider.id.clone();
        let channel_id = channel.id.clone();
        let model_name_clone = model_name.clone();
        let billing_type = model_entry.model.billing_type.clone();
        let model_input_price = model_entry.model.input_price;
        let model_output_price = model_entry.model.output_price;
        let model_request_price = model_entry.model.request_price;

        let event_stream = futures::stream::unfold(
            (byte_stream, String::new(), None as Option<i64>, None as Option<i64>),
            move |(mut byte_stream, mut buffer, mut input_tokens, mut output_tokens)| {
                let storage = storage_clone.clone();
                let audit_logger = audit_logger_clone.clone();
                let key_id = key_id.clone();
                let provider_id = provider_id.clone();
                let channel_id = channel_id.clone();
                let model_name = model_name_clone.clone();
                let billing_type = billing_type.clone();
                let start = start;
                async move {
                    loop {
                        // Try to extract a complete SSE event from buffer
                        while let Some(pos) = buffer.find("\n\n") {
                            let event_text = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            // Parse "data: ..." lines — Anthropic may have multiple per event block
                            let mut event_data = String::new();
                            for line in event_text.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data.trim() == "[DONE]" {
                                        // Stream finished — spawn audit + billing
                                        let latency_ms = start.elapsed().as_millis() as i64;
                                        tokio::spawn(async move {
                                            let cost = calculate_cost(
                                                &billing_type,
                                                input_tokens,
                                                output_tokens,
                                                model_input_price,
                                                model_output_price,
                                                model_request_price,
                                            );
                                            let usage = UsageRecord {
                                                id: uuid::Uuid::new_v4().to_string(),
                                                key_id: key_id.clone(),
                                                model_name: model_name.clone(),
                                                provider_id: provider_id.clone(),
                                                channel_id: Some(channel_id.clone()),
                                                protocol: Protocol::Anthropic,
                                                input_tokens,
                                                output_tokens,
                                                cost: cost.cost,
                                                created_at: chrono::Utc::now(),
                                            };
                                            let _ = storage.record_usage(&usage).await;
                                            let _ = audit_logger
                                                .log_request(
                                                    &key_id,
                                                    &model_name,
                                                    &provider_id,
                                                    Protocol::Anthropic,
                                                    "",
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
                                                .get("input_tokens")
                                                .and_then(|t| t.as_i64());
                                            output_tokens = usage
                                                .get("output_tokens")
                                                .and_then(|t| t.as_i64());
                                        }
                                    }
                                    event_data = data.to_string();
                                }
                            }

                            if !event_data.is_empty() {
                                let event = Event::default().data(event_data);
                                return Some((Ok::<_, std::convert::Infallible>(event), (byte_stream, buffer, input_tokens, output_tokens)));
                            }
                        }

                        // Read more bytes from upstream
                        match byte_stream.next().await {
                            Some(Ok(bytes)) => {
                                buffer.push_str(&String::from_utf8_lossy(&bytes));
                            }
                            Some(Err(_)) => {
                                // Upstream error — end the stream
                                return None;
                            }
                            None => {
                                // Upstream closed connection without [DONE]
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

    // 8. Non-streaming: create AnthropicProvider, call proxy_request
    let anthropic_provider = AnthropicProvider {
        name: provider.name.clone(),
        base_url,
        api_key: channel.api_key.clone(),
    };

    let start = Instant::now();
    let client = reqwest::Client::new();
    let proxy_result = anthropic_provider
        .proxy_request(&client, "/v1/messages", body, vec![])
        .await
        .map_err(|e: Box<dyn std::error::Error + Send + Sync>| ApiError::UpstreamError(502, e.to_string()))?;

    let latency_ms = start.elapsed().as_millis() as i64;

    // 9. Spawn async task for audit log + usage/billing
    let storage = state.storage.clone();
    let audit_logger = state.audit_logger.clone();
    let key_id = api_key.id.clone();
    let provider_id = provider.id.clone();
    let model_name_clone = model_name.clone();
    let status_code = proxy_result.status_code;
    let response_body = proxy_result.response_body.clone();
    let input_tokens = proxy_result.input_tokens;
    let output_tokens = proxy_result.output_tokens;
    let billing_type = model_entry.model.billing_type.clone();
    let model_input_price = model_entry.model.input_price;
    let model_output_price = model_entry.model.output_price;
    let model_request_price = model_entry.model.request_price;

    tokio::spawn(async move {
        // Calculate cost
        let cost = calculate_cost(
            &billing_type,
            input_tokens,
            output_tokens,
            model_input_price,
            model_output_price,
            model_request_price,
        );

        // Record usage
        let usage = UsageRecord {
            id: uuid::Uuid::new_v4().to_string(),
            key_id: key_id.clone(),
            model_name: model_name_clone.clone(),
            provider_id: provider_id.clone(),
            channel_id: Some(channel.id.clone()),
            protocol: Protocol::Anthropic,
            input_tokens,
            output_tokens,
            cost: cost.cost,
            created_at: chrono::Utc::now(),
        };
        let _ = storage.record_usage(&usage).await;

        // Write audit log
        let _ = audit_logger
            .log_request(
                &key_id,
                &model_name_clone,
                &provider_id,
                Protocol::Anthropic,
                "",
                &response_body,
                status_code as i32,
                latency_ms,
                input_tokens,
                output_tokens,
            )
            .await;
    });

    // 10. Return response to client
    let status = StatusCode::from_u16(proxy_result.status_code)
        .unwrap_or(StatusCode::BAD_GATEWAY);
    Ok((status, proxy_result.response_body).into_response())
}

/// GET /v1/models — list models available via Anthropic-compatible providers.
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

    let anthropic_models: Vec<Value> = models
        .iter()
        .filter(|m| m.anthropic_compatible)
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
        "data": anthropic_models,
        "object": "list",
    })))
}

/// Helper to determine rate limits for a key/model pair.
async fn get_rate_limits(
    state: &Arc<AppState>,
    api_key: &ApiKey,
    model_name: &str,
) -> (Option<i64>, Option<i64>) {
    if let Ok(Some(limit)) = state.storage.get_key_model_rate_limit(&api_key.id, model_name).await {
        (Some(limit.rpm), Some(limit.tpm))
    } else {
        (api_key.rate_limit, None)
    }
}
