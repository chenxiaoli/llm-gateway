use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

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

    tracing::debug!("[PROXY] Incoming request, model: {}, protocol: {:?}", model_name, protocol);

    let _original_model = model_name.clone();

    // === Step 3: Find model → provider → channels ===
    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::debug!("[PROXY] Found {} models in database", models.len());

    let is_stream = req_json
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    let model_entry = match protocol {
        ProxyProtocol::OpenAI => models
            .iter()
            .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
            .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?,
        ProxyProtocol::Anthropic => models
            .iter()
            .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
            .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?,
    };

    tracing::debug!("[PROXY] Found model: {} (id: {})", model_entry.model.name, model_entry.model.id);

    // === Step 3: Route via ChannelModel (sole routing source) ===
    // Get channel_models for this model, filter enabled, get channels, sort by priority
    let channel_models = state
        .storage
        .get_channel_models_for_model(&model_entry.model.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::debug!("[PROXY] Found {} channel_models for model", channel_models.len());

    // Filter enabled channel_models and get their channel_ids
    let enabled_channel_model_ids: Vec<&str> = channel_models
        .iter()
        .filter(|cm| cm.enabled)
        .map(|cm| cm.channel_id.as_str())
        .collect();

    if enabled_channel_model_ids.is_empty() {
        return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
    }

    tracing::debug!("[PROXY] Enabled channel_model ids: {:?}", enabled_channel_model_ids);

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

    tracing::debug!("[PROXY] Routing candidates (sorted by priority): {}", routing_candidates.len());

    // Get provider from first channel (for endpoints lookup)
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

        // Get base_url: provider.endpoints[protocol] → provider.endpoints["default"]
        let endpoints_map: serde_json::Value = provider
            .endpoints
            .as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::Value::Null);
        let proto_key = match protocol {
            ProxyProtocol::OpenAI => "openai",
            ProxyProtocol::Anthropic => "anthropic",
        };
        let base_url = endpoints_map
            .get(proto_key)
            .and_then(|v| v.as_str())
            .or_else(|| endpoints_map.get("default").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .ok_or_else(|| ApiError::Internal(format!("No endpoint for channel '{}' (check provider endpoints, need '{}' or 'default')", channel.name, proto_key)))?;

        // endpoints is base URL - append path
        // /v1/messages -> endpoint.anthropic + /v1/messages
        // /v1/chat/completions -> endpoint.openai + /v1/chat/completions
        let path = match protocol {
            ProxyProtocol::OpenAI => "/v1/chat/completions",
            ProxyProtocol::Anthropic => "/v1/messages",
        };
        let upstream_url = format!("{}{}", base_url.trim_end_matches('/'), path);

        tracing::debug!("[PROXY] Upstream URL: {} -> {}", path, upstream_url);
        let mut req = client.post(&upstream_url);

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

        tracing::debug!("[PROXY] Upstream response: channel={}, status={}, latency={}ms", channel.name, status, latency_ms);

        if status >= 500 {
            last_error = format!("Server error {} on channel '{}'", status, channel.name);
            continue;
        }

        if status != 200 && status < 500 {
            let error_body = resp.text().await.unwrap_or_default();
            tracing::debug!("[PROXY] Upstream error response: status={}, body_len={}", status, error_body.len());
            return Ok((StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY), error_body).into_response());
        }

        tracing::debug!("[PROXY] Upstream success response: status={}, is_stream={}", status, is_stream);

        // === Handle streaming: accumulate SSE → forward to client → audit worker parses & calculates cost ===
        if is_stream {
            let audit_tx = state.audit_tx.clone();
            let (tx, rx) = mpsc::channel::<bytes::Bytes>(100);

            let upstream_name_for_worker = upstream_name.to_string();
            let model_name_for_worker = model_name.clone();
            let provider_id_for_worker = provider_id.clone();
            let api_key_id_for_worker = api_key.id.clone();
            let body_for_worker = body.clone();
            let proto = match protocol {
                ProxyProtocol::OpenAI => Protocol::Openai,
                ProxyProtocol::Anthropic => Protocol::Anthropic,
            };
            let channel_id_for_worker = channel.id.clone();
            let model_override_reason = if upstream_name != &model_name {
                Some("channel_mapping".to_string())
            } else {
                None
            };
            // Resolve pricing policy for cost calculation
            let (pricing_policy_config, pricing_policy_billing_type) = {
                let policy_id = channel_model.pricing_policy_id.as_deref()
                    .or(model_entry.model.pricing_policy_id.as_deref());
                match policy_id {
                    Some(id) => {
                        let opt = state.storage.get_pricing_policy(id).await
                            .map_err(|e| ApiError::Internal(e.to_string()))?;
                        match opt {
                            Some(p) => (Some(p.config), p.billing_type),
                            None => (None, "per_token".to_string()),
                        }
                    }
                    None => (None, "per_token".to_string()),
                }
            };
            let markup_ratio_for_worker = channel_model.markup_ratio;
            let request_path_for_worker = path.to_string();
            let upstream_url_for_worker = upstream_url.clone();

            // Spawn: read upstream SSE → forward to client → accumulate → send to audit worker
            tokio::spawn(async move {
                let byte_stream = resp.bytes_stream();
                let mut acc = Vec::new();

                tokio::pin!(byte_stream);
                loop {
                    match futures::TryStreamExt::try_next(&mut byte_stream).await {
                        Ok(Some(bytes)) => {
                            acc.extend_from_slice(&bytes);
                            let _ = tx.send(bytes).await;
                        }
                        Ok(None) => break,
                        Err(e) => {
                            tracing::warn!("[PROXY] Stream read error: {}", e);
                            let _ = tx.send(bytes::Bytes::new()).await;
                            break;
                        }
                    }
                }
                drop(tx);

                // Send accumulated bytes to audit worker (it parses SSE, calculates cost, writes DB)
                let task = AuditTask {
                    key_id: api_key_id_for_worker,
                    model_name: upstream_name_for_worker.clone(),
                    provider_id: provider_id_for_worker.clone(),
                    protocol: proto,
                    stream: true,
                    request_body: body_for_worker,
                    response_bytes: acc,
                    status_code: 200,
                    latency_ms,
                    pricing_policy_config,
                    pricing_policy_billing_type,
                    markup_ratio: markup_ratio_for_worker,
                    channel_id: Some(channel_id_for_worker),
                    original_model: if upstream_name_for_worker != model_name_for_worker {
                        Some(model_name_for_worker.clone())
                    } else {
                        None
                    },
                    upstream_model: if upstream_name_for_worker != model_name_for_worker {
                        Some(upstream_name_for_worker.clone())
                    } else {
                        None
                    },
                    model_override_reason,
                    request_path: Some(request_path_for_worker),
                    upstream_url: Some(upstream_url_for_worker),
                };
                if let Err(e) = audit_tx.send(task).await {
                    tracing::warn!("[PROXY] Failed to send audit task: {}", e);
                }
            });

            let stream_body = axum::body::Body::from_stream(
                tokio_stream::wrappers::ReceiverStream::new(rx)
                    .map(|b: bytes::Bytes| Ok::<_, std::convert::Infallible>(b))
            );

            let mut response = axum::response::Response::new(stream_body);
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                "text/event-stream; charset=utf-8".parse().unwrap(),
            );

            return Ok(response);
        }

        // Non-streaming: send to audit worker (it parses JSON, calculates cost, writes DB)
        let response_bytes = resp.bytes().await.unwrap_or_default().to_vec();

        // Resolve pricing policy for cost calculation
        let (pricing_policy_config, pricing_policy_billing_type) = {
            let policy_id = channel_model.pricing_policy_id.as_deref()
                .or(model_entry.model.pricing_policy_id.as_deref());
            match policy_id {
                Some(id) => {
                    let opt = state.storage.get_pricing_policy(id).await
                        .map_err(|e| ApiError::Internal(e.to_string()))?;
                    match opt {
                        Some(p) => (Some(p.config), p.billing_type),
                        None => (None, "per_token".to_string()),
                    }
                }
                None => (None, "per_token".to_string()),
            }
        };

        let proto = match protocol {
            ProxyProtocol::OpenAI => Protocol::Openai,
            ProxyProtocol::Anthropic => Protocol::Anthropic,
        };
        let task = AuditTask {
            key_id: api_key.id.clone(),
            model_name: upstream_name.to_string(),
            provider_id: provider_id.clone(),
            protocol: proto,
            stream: false,
            request_body: body.clone(),
            response_bytes: response_bytes.clone(),
            status_code: 200,
            latency_ms,
            pricing_policy_config,
            pricing_policy_billing_type,
            markup_ratio: channel_model.markup_ratio,
            channel_id: Some(channel.id.clone()),
            original_model: if upstream_name != &model_name { Some(model_name.clone()) } else { None },
            upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
            model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
            request_path: Some(path.to_string()),
            upstream_url: Some(upstream_url.clone()),
        };
        let _ = state.audit_tx.try_send(task);

        return Ok(response_bytes.into_response());
    }

    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
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