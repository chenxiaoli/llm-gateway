use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use std::sync::Arc;
use std::time::Instant;

use llm_gateway_auth::hash_api_key;
use llm_gateway_encryption::decrypt;
use llm_gateway_storage::Protocol;

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;
use crate::AuditTask;

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

    let _original_model = model_name.clone();

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

    // === Step 3: Route via ChannelModel (sole routing source) ===
    // Get channel_models for this model, filter enabled, get channels, sort by priority
    let channel_models = state
        .storage
        .get_channel_models_for_model(&model_entry.model.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Filter enabled channel_models and get their channel_ids
    let enabled_channel_model_ids: Vec<&str> = channel_models
        .iter()
        .filter(|cm| cm.enabled)
        .map(|cm| cm.channel_id.as_str())
        .collect();

    if enabled_channel_model_ids.is_empty() {
        return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
    }

    // Get all channels and filter to those in our channel_models
    let all_channels = state
        .storage
        .list_channels()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build a map for quick channel lookup by id
    let channel_map: std::collections::HashMap<&str, &llm_gateway_storage::Channel> = all_channels
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect();

    // Create routing candidates: (channel, channel_model) pairs
    let mut routing_candidates: Vec<(&llm_gateway_storage::Channel, &llm_gateway_storage::ChannelModel)> = Vec::new();

    for cm in channel_models.iter().filter(|cm| cm.enabled) {
        if let Some(channel) = channel_map.get(cm.channel_id.as_str()) {
            if channel.enabled {
                routing_candidates.push((channel, cm));
            }
        }
    }

    if routing_candidates.is_empty() {
        return Err(ApiError::NotFound("No enabled channels available for routing".to_string()));
    }

    // Sort by channel priority (lower number = higher priority)
    routing_candidates.sort_by(|a, b| a.0.priority.cmp(&b.0.priority));

    // Get provider from first channel (for base_url fallback)
    let provider_id = routing_candidates.first().map(|(c, _)| c.provider_id.clone())
        .ok_or_else(|| ApiError::Internal("No channels available".to_string()))?;

    let provider = state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let client = reqwest::Client::new();
    let mut last_error = String::new();

    // === Route with failover ===
    for (channel, channel_model) in &routing_candidates {
        // Use upstream_model_name if provided, otherwise use the model name from request
        let upstream_name = channel_model.upstream_model_name.as_deref().unwrap_or(&model_name);

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

        // Forward non-auth client headers to upstream (exclude host, authorization, content-length, api keys)
        for (name, value) in headers.iter() {
            match name.as_str() {
                "host" | "authorization" | "content-length" | "x-api-key" | "api-key" => continue,
                _ => {
                    req = req.header(name.clone(), value);
                }
            }
        }

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
        if is_stream {
            // === Streaming: forward raw SSE stream as-is ===
            use futures::TryStreamExt;
            let byte_stream = resp.bytes_stream();

            let proto = match protocol {
                ProxyProtocol::OpenAI => Protocol::Openai,
                ProxyProtocol::Anthropic => Protocol::Anthropic,
            };

            // Send audit task via MPSC channel (non-blocking)
            let model_override_reason = if upstream_name != &model_name {
                Some("channel_mapping".to_string())
            } else {
                None
            };
            let audit_task = AuditTask {
                key_id: api_key.id.clone(),
                model_name: upstream_name.to_string(),
                provider_id: provider.id.clone(),
                protocol: proto,
                stream: true,
                request_body: body.clone(),
                response_body: "[streaming]".to_string(),
                status_code: 200,
                latency_ms,
                input_tokens: None,
                output_tokens: None,
                original_model: if upstream_name != &model_name { Some(model_name.clone()) } else { None },
                upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
                model_override_reason,
            };
            match state.audit_tx.send(audit_task).await {
                Ok(()) => tracing::debug!("[PROXY] Sent audit task to channel"),
                Err(e) => tracing::warn!("[PROXY] Failed to send audit task: channel full or closed"),
            }

            // Simply forward the stream without processing
            let stream = byte_stream.map_ok(|bytes| bytes);

            let mut response = axum::response::Response::new(axum::body::Body::from_stream(stream));

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
        let provider_id = provider_id.clone(); // Already captured from first channel above
        let model_name = model_name.clone();
        let upstream_name = upstream_name.to_string();
        let channel_id = channel.id.clone();
        let proto = match protocol {
            ProxyProtocol::OpenAI => Protocol::Openai,
            ProxyProtocol::Anthropic => Protocol::Anthropic,
        };
        let proto_for_audit = proto.clone();
        let proto_for_usage = proto.clone();

        // Use ChannelModel override > Model default for pricing
        let pricing_policy_id = channel_model.cost_policy_id.clone().or_else(|| model_entry.model.pricing_policy_id.clone());
        let billing_type = channel_model.billing_type.clone().or_else(|| Some("token".to_string()));
        let input_price = channel_model.input_price.unwrap_or(model_entry.model.input_price);
        let output_price = channel_model.output_price.unwrap_or(model_entry.model.output_price);
        let request_price = channel_model.request_price.unwrap_or(model_entry.model.request_price);

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
                // Use billing_type from ChannelModel or default to Token
                let billing = match billing_type.as_deref() {
                    Some("request") => llm_gateway_storage::BillingType::Request,
                    _ => llm_gateway_storage::BillingType::Token,
                };
                llm_gateway_billing::calculate_cost(
                    &billing,
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

            let model_override_reason = if upstream_name != model_name {
                Some("channel_mapping".to_string())
            } else {
                None
            };

            let _ = audit_logger.log_request(
                &key_id,
                &upstream_name,
                &provider_id,
                proto_for_audit,
                is_stream,
                &request_body,
                &response_body,
                200,
                latency_ms,
                input_tokens,
                output_tokens,
                if upstream_name != model_name { Some(&model_name) } else { None },
                if upstream_name != model_name { Some(upstream_name.as_str()) } else { None },
                model_override_reason.as_deref(),
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