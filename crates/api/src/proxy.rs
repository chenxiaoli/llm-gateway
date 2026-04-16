use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::StreamExt;
use std::sync::Arc;
use std::time::Instant;
use tracing::debug;

use llm_gateway_auth::hash_api_key;
use llm_gateway_encryption::decrypt;
use llm_gateway_storage::Protocol;

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;

/// Protocol for determining which adapter to use
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyProtocol {
    OpenAI,
    Anthropic,
}

/// Unified proxy: receives request → forwards to upstream → returns response
/// Usage/Cost/Audit are handled in spawned async tasks
pub async fn proxy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
    protocol: ProxyProtocol,
) -> Result<axum::response::Response, ApiError> {
    // === Step 1: Auth ===
    let raw_token = extract_bearer_token(&headers)?;
    let token_hash = hash_api_key(&raw_token);
    let api_key = state
        .storage
        .get_key_by_hash(&token_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    if !api_key.enabled {
        return Err(ApiError::Forbidden);
    }

    // === Step 2: Parse model ===
    let req_json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let model_name = req_json
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or(ApiError::BadRequest("Missing 'model' field".to_string()))?
        .to_string();

    // === Step 3: Find model → provider → channels ===
    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let is_stream = req_json
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    let model_entry = match protocol {
        ProxyProtocol::OpenAI => models
            .iter()
            .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase() && m.openai_compatible && m.model.enabled)
            .ok_or(ApiError::NotFound(format!("Model '{}' not found or not OpenAI compatible", model_name)))?,
        ProxyProtocol::Anthropic => models
            .iter()
            .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase() && m.anthropic_compatible && m.model.enabled)
            .ok_or(ApiError::NotFound(format!("Model '{}' not found or not Anthropic compatible", model_name)))?,
    };

    let provider_id = model_entry.model.provider_id.clone();
    let provider = state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    if !provider.enabled {
        return Err(ApiError::NotFound(format!("Provider '{}' is disabled", provider_id)));
    }

    let channels = match state.storage.get_channels_for_model(&model_name).await {
        Ok(channels) if !channels.is_empty() => channels,
        _ => state.storage.list_enabled_channels_by_provider(&provider_id).await.map_err(|e| ApiError::Internal(e.to_string()))?,
    };
    if channels.is_empty() {
        return Err(ApiError::Internal(format!("Provider '{}' has no enabled channels", provider_id)));
    }

    let channel_models = state.storage.get_channel_models_for_model(&model_name).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let client = reqwest::Client::new();
    let mut last_error = String::new();

    // === Route with failover ===
    for channel in &channels {
        let upstream_name = channel_models
            .iter()
            .find(|cm| cm.channel_id == channel.id)
            .map(|cm| cm.upstream_model_name.as_str())
            .unwrap_or(&model_name);

        let modified_body = if upstream_name != &model_name {
            let mut req_json_modified = req_json.clone();
            if let Some(model_obj) = req_json_modified.get_mut("model") {
                *model_obj = serde_json::Value::String(upstream_name.to_string());
            }
            serde_json::to_string(&req_json_modified).unwrap_or_else(|_| body.clone())
        } else {
            body.clone()
        };

        let api_key_value = decrypt(&channel.api_key, &state.encryption_key)
            .unwrap_or_else(|_| channel.api_key.clone());

        // Get base_url: channel → provider endpoints JSON → provider base_url
        let base_url = if let Some(url) = channel.base_url.as_deref() {
            url.to_string()
        } else {
            // Try provider endpoints JSON
            let endpoints: serde_json::Value = provider
                .endpoints
                .as_ref()
                .and_then(|e| serde_json::from_str(e).ok())
                .unwrap_or(serde_json::Value::Null);
            let key = match protocol {
                ProxyProtocol::OpenAI => "openai",
                ProxyProtocol::Anthropic => "anthropic",
            };
            endpoints
                .get(key)
                .and_then(|v| v.as_str())
                .or(provider.base_url.as_deref())
                .map(|s| s.to_string())
                .ok_or_else(|| ApiError::Internal(format!("No base_url for channel {} (check provider endpoints)", channel.name)))?
        };

        // endpoints is base URL - append path
        // /v1/messages -> endpoint.anthropic + /v1/messages
        // /v1/chat/completions -> endpoint.openai + /v1/chat/completions
        let path = match protocol {
            ProxyProtocol::OpenAI => "/v1/chat/completions",
            ProxyProtocol::Anthropic => "/v1/messages",
        };
        let url = format!("{}{}", base_url.trim_end_matches('/'), path);
        let mut req = client.post(&url);

        match protocol {
            ProxyProtocol::OpenAI => {
                req = req.header("Content-Type", "application/json");
            }
            ProxyProtocol::Anthropic => {
                req = req
                    .header("x-api-key", &api_key_value)
                    .header("anthropic-version", "2023-06-01")
                    .header("Content-Type", "application/json");
            }
        }

        req = req.header("Authorization", format!("Bearer {}", api_key_value));
        req = req.body(modified_body);

        let start = Instant::now();
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("Connection error on channel '{}': {}", channel.name, e);
                continue;
            }
        };
        let latency_ms = start.elapsed().as_millis() as i64;
        let status = resp.status().as_u16();

        if status >= 500 {
            last_error = format!("Server error {} on channel '{}'", status, channel.name);
            continue;
        }

        if status != 200 && status < 500 {
            let error_body = resp.text().await.unwrap_or_default();
            return Ok((StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY), error_body).into_response());
        }

        // === Handle streaming vs non-streaming ===
        let start = Instant::now();

        if is_stream {
            // === Streaming: forward SSE with token tracking ===
            let byte_stream = resp.bytes_stream();

            // Clone all needed data for the stream handler
            let storage = state.storage.clone();
            let audit_logger = state.audit_logger.clone();
            let key_id = api_key.id.clone();
            let provider_id = provider.id.clone();
            let model_name = model_name.clone();
            let channel_id = channel.id.clone();
            let pricing_policy_id = model_entry.model.pricing_policy_id.clone();
            let model_input_price = model_entry.model.input_price;
            let model_output_price = model_entry.model.output_price;
            let model_request_price = model_entry.model.request_price;
            let proto = match protocol {
                ProxyProtocol::OpenAI => Protocol::Openai,
                ProxyProtocol::Anthropic => Protocol::Anthropic,
            };
            let body = body.clone();

            let event_stream = futures::stream::unfold(
                (byte_stream, String::new(), None as Option<i64>, None as Option<i64>),
                move |(mut byte_stream, mut buffer, mut input_tokens, mut output_tokens)| {
                    let storage = storage.clone();
                    let audit_logger = audit_logger.clone();
                    let key_id = key_id.clone();
                    let provider_id = provider_id.clone();
                    let model_name = model_name.clone();
                    let channel_id = channel_id.clone();
                    let pricing_policy_id = pricing_policy_id.clone();
                    let model_input_price = model_input_price;
                    let model_output_price = model_output_price;
                    let model_request_price = model_request_price;
                    let proto = proto.clone();
                    let body = body.clone();

                    async move {
                        loop {
                            // Try to extract a complete SSE event from buffer
                            while let Some(pos) = buffer.find("\n\n") {
                                let event_text = buffer[..pos].to_string();
                                buffer = buffer[pos + 2..].to_string();

                                // Parse "data: ..." lines
                                let mut event_data = String::new();
                                for line in event_text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if data.trim() == "[DONE]" {
                                            // Stream finished — spawn audit + billing
                                            let latency_ms = start.elapsed().as_millis() as i64;
                                            debug!("Stream [DONE] detected for model {}", model_name);

                                            tokio::spawn(async move {
                                                use llm_gateway_billing::{PricingCalculator, Usage};

                                                let usage = Usage::from_tokens(
                                                    Some(input_tokens.unwrap_or(0)),
                                                    Some(output_tokens.unwrap_or(0)),
                                                    1,
                                                );
                                                let calculator = PricingCalculator;

                                                let cost = if let Some(pid) = &pricing_policy_id {
                                                    match storage.get_pricing_policy(pid).await {
                                                        Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
                                                        _ => 0.0,
                                                    }
                                                } else {
                                                    llm_gateway_billing::calculate_cost(
                                                        &llm_gateway_storage::BillingType::Token,
                                                        input_tokens,
                                                        output_tokens,
                                                        model_input_price,
                                                        model_output_price,
                                                        model_request_price,
                                                    ).cost
                                                };

                                                let record = llm_gateway_storage::UsageRecord {
                                                    id: uuid::Uuid::new_v4().to_string(),
                                                    key_id: key_id.clone(),
                                                    model_name: model_name.clone(),
                                                    provider_id: provider_id.clone(),
                                                    channel_id: Some(channel_id.clone()),
                                                    protocol: proto.clone(),
                                                    input_tokens,
                                                    output_tokens,
                                                    cost,
                                                    created_at: chrono::Utc::now(),
                                                };
                                                let _ = storage.record_usage(&record).await;
                                                let _ = audit_logger.log_request(
                                                    &key_id,
                                                    &model_name,
                                                    &provider_id,
                                                    proto,
                                                    true,
                                                    &body,
                                                    &buffer,
                                                    200,
                                                    latency_ms,
                                                    input_tokens,
                                                    output_tokens,
                                                ).await;
                                            });
                                            return None;
                                        }

                                        // Try to extract usage from this event
                                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                                            if let Some(usage) = v.get("usage") {
                                                // Try Anthropic style first (input_tokens/output_tokens)
                                                if let Some(in_tok) = usage.get("input_tokens").and_then(|t| t.as_i64()) {
                                                    input_tokens = Some(in_tok);
                                                }
                                                if let Some(out_tok) = usage.get("output_tokens").and_then(|t| t.as_i64()) {
                                                    output_tokens = Some(out_tok);
                                                }
                                                // Also check if we need to update (they might be None)
                                                if input_tokens.is_none() {
                                                    if let Some(in_tok) = usage.get("prompt_tokens").and_then(|t| t.as_i64()) {
                                                        input_tokens = Some(in_tok);
                                                    }
                                                }
                                                if output_tokens.is_none() {
                                                    if let Some(out_tok) = usage.get("completion_tokens").and_then(|t| t.as_i64()) {
                                                        output_tokens = Some(out_tok);
                                                    }
                                                }
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
                                    // Stream error, record what we have
                                    let latency_ms = start.elapsed().as_millis() as i64;
                                    tokio::spawn(async move {
                                        use llm_gateway_billing::{PricingCalculator, Usage};
                                        let usage = Usage::from_tokens(Some(input_tokens.unwrap_or(0)), Some(output_tokens.unwrap_or(0)), 1);
                                        let calculator = PricingCalculator;
                                        let cost = if let Some(pid) = &pricing_policy_id {
                                            match storage.get_pricing_policy(pid).await {
                                                Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
                                                _ => 0.0,
                                            }
                                        } else {
                                            llm_gateway_billing::calculate_cost(
                                                &llm_gateway_storage::BillingType::Token,
                                                input_tokens,
                                                output_tokens,
                                                model_input_price,
                                                model_output_price,
                                                model_request_price,
                                            ).cost
                                        };
                                        let record = llm_gateway_storage::UsageRecord {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            key_id: key_id.clone(),
                                            model_name: model_name.clone(),
                                            provider_id: provider_id.clone(),
                                            channel_id: Some(channel_id.clone()),
                                            protocol: proto.clone(),
                                            input_tokens,
                                            output_tokens,
                                            cost,
                                            created_at: chrono::Utc::now(),
                                        };
                                        let _ = storage.record_usage(&record).await;
                                        let _ = audit_logger.log_request(
                                            &key_id, &model_name, &provider_id, proto, true,
                                            &body, "[stream error]", 500, latency_ms, input_tokens, output_tokens,
                                        ).await;
                                    });
                                    return None;
                                }
                                None => {
                                    // Stream ended normally
                                    let latency_ms = start.elapsed().as_millis() as i64;
                                    tokio::spawn(async move {
                                        use llm_gateway_billing::{PricingCalculator, Usage};
                                        let usage = Usage::from_tokens(Some(input_tokens.unwrap_or(0)), Some(output_tokens.unwrap_or(0)), 1);
                                        let calculator = PricingCalculator;
                                        let cost = if let Some(pid) = &pricing_policy_id {
                                            match storage.get_pricing_policy(pid).await {
                                                Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
                                                _ => 0.0,
                                            }
                                        } else {
                                            llm_gateway_billing::calculate_cost(
                                                &llm_gateway_storage::BillingType::Token,
                                                input_tokens,
                                                output_tokens,
                                                model_input_price,
                                                model_output_price,
                                                model_request_price,
                                            ).cost
                                        };
                                        let record = llm_gateway_storage::UsageRecord {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            key_id: key_id.clone(),
                                            model_name: model_name.clone(),
                                            provider_id: provider_id.clone(),
                                            channel_id: Some(channel_id.clone()),
                                            protocol: proto.clone(),
                                            input_tokens,
                                            output_tokens,
                                            cost,
                                            created_at: chrono::Utc::now(),
                                        };
                                        let _ = storage.record_usage(&record).await;
                                        let _ = audit_logger.log_request(
                                            &key_id, &model_name, &provider_id, proto, true,
                                            &body, &buffer, 200, latency_ms, input_tokens, output_tokens,
                                        ).await;
                                    });
                                    return None;
                                }
                            }
                        }
                    }
                },
            );

            let sse = Sse::new(event_stream).keep_alive(KeepAlive::default());
            let mut response = sse.into_response();

            // Set proper content-type for SSE
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                "text/event-stream; charset=utf-8".parse().unwrap(),
            );
            return Ok(response);
        }

        // Non-streaming: original behavior
        let response_body = resp.text().await.unwrap_or_default();

        // === Spawn async workers ===
        let storage = state.storage.clone();
        let audit_logger = state.audit_logger.clone();
        let key_id = api_key.id.clone();
        let provider_id = provider.id.clone();
        let model_name = model_name.clone();
        let channel_id = channel.id.clone();
        let proto = match protocol {
            ProxyProtocol::OpenAI => Protocol::Openai,
            ProxyProtocol::Anthropic => Protocol::Anthropic,
        };
        let proto_for_audit = proto.clone();
        let proto_for_usage = proto.clone();
        let pricing_policy_id = model_entry.model.pricing_policy_id.clone();
        let input_price = model_entry.model.input_price;
        let output_price = model_entry.model.output_price;
        let request_price = model_entry.model.request_price;
        let (input_tokens, output_tokens) = extract_usage_from_response(&response_body, protocol);
        let body_for_worker = body.clone();
        let response_for_worker = response_body.clone();

        tokio::spawn(async move {
            use llm_gateway_billing::{PricingCalculator, Usage};

            // Note: audit_logger.log_request already checks settings internally
            let request_body = body_for_worker;
            let response_body = response_for_worker;

            let usage = Usage::from_tokens(input_tokens, output_tokens, 1);
            let calculator = PricingCalculator;

            let cost = if let Some(pid) = &pricing_policy_id {
                match storage.get_pricing_policy(pid).await {
                    Ok(Some(policy)) => calculator.calculate_cost(&policy, &usage),
                    _ => 0.0,
                }
            } else {
                llm_gateway_billing::calculate_cost(
                    &llm_gateway_storage::BillingType::Token,
                    input_tokens,
                    output_tokens,
                    input_price,
                    output_price,
                    request_price,
                ).cost
            };

            let record = llm_gateway_storage::UsageRecord {
                id: uuid::Uuid::new_v4().to_string(),
                key_id: key_id.clone(),
                model_name: model_name.clone(),
                provider_id: provider_id.clone(),
                channel_id: Some(channel_id.clone()),
                protocol: proto_for_usage,
                input_tokens,
                output_tokens,
                cost,
                created_at: chrono::Utc::now(),
            };

            let _ = storage.record_usage(&record).await;

            let _ = audit_logger.log_request(
                &key_id,
                &model_name,
                &provider_id,
                proto_for_audit,
                is_stream,
                &request_body,
                &response_body,
                200,
                latency_ms,
                input_tokens,
                output_tokens,
            ).await;
        });

        return Ok(response_body.into_response());
    }

    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
}

fn extract_usage_from_response(body: &str, protocol: ProxyProtocol) -> (Option<i64>, Option<i64>) {
    let v: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let usage = match v.get("usage") {
        Some(u) => u,
        None => return (None, None),
    };

    match protocol {
        ProxyProtocol::OpenAI => {
            let input = usage.get("prompt_tokens").and_then(|t| t.as_i64());
            let output = usage.get("completion_tokens").and_then(|t| t.as_i64());
            (input, output)
        }
        ProxyProtocol::Anthropic => {
            let input = usage.get("input_tokens").and_then(|t| t.as_i64());
            let output = usage.get("output_tokens").and_then(|t| t.as_i64());
            (input, output)
        }
    }
}

/// Wrapper for /v1/chat/completions - uses OpenAI protocol
pub async fn proxy_with_protocol(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy(State(state), headers, body, ProxyProtocol::OpenAI).await
}

/// Wrapper for /v1/messages - uses Anthropic protocol
pub async fn messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy(State(state), headers, body, ProxyProtocol::Anthropic).await
}