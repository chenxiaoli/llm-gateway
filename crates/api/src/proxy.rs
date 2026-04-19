use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use async_trait::async_trait;

use llm_gateway_auth::hash_api_key;
use llm_gateway_encryption::decrypt;
use llm_gateway_storage::Protocol;

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;
use crate::AuditTask;

// ─── Resolved Channel ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ResolvedChannel {
    pub channel_id: Uuid,
    pub provider_id: String,
    pub upstream_base_url: String,
    pub upstream_api_key: String, // decrypted
    pub adapter: ProxyProtocol,
    pub timeout_ms: u64,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub upstream_model_name: Option<String>,
}

// ─── Channel Registry ────────────────────────────────────────────────────────

#[async_trait]
pub trait ChannelRegistry: Send + Sync {
    async fn resolve_by_model(&self, model: &str) -> Vec<ResolvedChannel>;
    async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel>;
    async fn reload(&self);
}

/// Protocol for determining which adapter to use
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyProtocol {
    OpenAI,
    Anthropic,
}

// ─── InMemoryChannelRegistry ─────────────────────────────────────────────────

use arc_swap::ArcSwap;

pub struct InMemoryChannelRegistry {
    cache: Arc<ArcSwap<HashMap<String, ResolvedChannel>>>,
    model_index: Arc<ArcSwap<HashMap<String, Vec<String>>>>,
    storage: Arc<dyn llm_gateway_storage::Storage>,
    encryption_key: [u8; 32],
    refresh_interval: Duration,
}

impl InMemoryChannelRegistry {
    pub fn new(
        storage: Arc<dyn llm_gateway_storage::Storage>,
        encryption_key: [u8; 32],
        refresh_interval: Duration,
    ) -> Self {
        Self {
            cache: Arc::new(ArcSwap::from_pointee(HashMap::new())),
            model_index: Arc::new(ArcSwap::from_pointee(HashMap::new())),
            storage,
            encryption_key,
            refresh_interval,
        }
    }

    pub async fn start_refresh_loop(self: Arc<Self>) {
        self.reload().await;
        let mut interval = tokio::time::interval(self.refresh_interval);
        loop {
            interval.tick().await;
            if let Err(e) = self.do_reload().await {
                tracing::warn!("ChannelRegistry reload failed: {}", e);
            }
        }
    }

    async fn reload(&self) {
        if let Err(e) = self.do_reload().await {
            tracing::warn!("ChannelRegistry reload failed: {}", e);
        }
    }

    async fn do_reload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let channels = self.storage.list_channels().await?;
        let channel_models = self.storage.list_channel_models().await?;
        let providers = self.storage.list_providers().await?;
        let models = self.storage.list_models().await?;

        let mut cache = HashMap::new();
        let mut model_index: HashMap<String, Vec<String>> = HashMap::new();

        let provider_map: HashMap<&str, &llm_gateway_storage::Provider> =
            providers.iter().map(|p| (p.id.as_str(), p)).collect();

        let cm_by_channel: HashMap<&str, Vec<&llm_gateway_storage::ChannelModel>> = {
            let mut m: HashMap<&str, Vec<&llm_gateway_storage::ChannelModel>> = HashMap::new();
            for cm in &channel_models {
                m.entry(cm.channel_id.as_str()).or_default().push(cm);
            }
            m
        };

        for channel in &channels {
            if !channel.enabled {
                continue;
            }

            let provider = match provider_map.get(channel.provider_id.as_str()) {
                Some(p) => p,
                None => continue,
            };

            let endpoints: serde_json::Value = provider
                .endpoints
                .as_ref()
                .and_then(|e| serde_json::from_str(e).ok())
                .unwrap_or(serde_json::Value::Null);

            let endpoint_openai = endpoints
                .get("openai")
                .and_then(|v| v.as_str())
                .or_else(|| endpoints.get("default").and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            let api_key = llm_gateway_encryption::decrypt(&channel.api_key, &self.encryption_key)
                .unwrap_or_else(|_| channel.api_key.clone());

            let resolved = ResolvedChannel {
                channel_id: Uuid::parse_str(&channel.id).unwrap_or_else(|_| Uuid::new_v4()),
                provider_id: channel.provider_id.clone(),
                upstream_base_url: endpoint_openai.unwrap_or_default(),
                upstream_api_key: api_key,
                adapter: ProxyProtocol::OpenAI,
                timeout_ms: 60_000,
                priority: channel.priority,
                pricing_policy_id: channel.pricing_policy_id.clone(),
                markup_ratio: channel.markup_ratio,
                upstream_model_name: None,
            };

            cache.insert(channel.id.clone(), resolved);

            if let Some(cms) = cm_by_channel.get(channel.id.as_str()) {
                for cm in cms {
                    if !cm.enabled {
                        continue;
                    }
                    if let Some(model) = models.iter().find(|m| m.model.id == cm.model_id) {
                        let model_name = model.model.name.to_lowercase();
                        model_index
                            .entry(model_name)
                            .or_default()
                            .push(channel.id.clone());
                    }
                }
            }
        }

        // Sort channel_ids by priority (higher priority first)
        for (_model, channel_ids) in model_index.iter_mut() {
            channel_ids.sort_by(|a, b| {
                let prio_a = cache
                    .get(a)
                    .map(|c| c.priority)
                    .unwrap_or(i32::MIN);
                let prio_b = cache
                    .get(b)
                    .map(|c| c.priority)
                    .unwrap_or(i32::MIN);
                prio_b.cmp(&prio_a)
            });
        }

        self.cache.store(Arc::new(cache));
        self.model_index.store(Arc::new(model_index));

        Ok(())
    }
}

#[async_trait::async_trait]
impl ChannelRegistry for InMemoryChannelRegistry {
    async fn resolve_by_model(&self, model: &str) -> Vec<ResolvedChannel> {
        let model_key = model.to_lowercase();
        let channel_ids = self.model_index.load().get(&model_key).cloned();
        match channel_ids {
            Some(ids) => {
                let cache = self.cache.load();
                ids.iter()
                    .filter_map(|id| cache.get(id).cloned())
                    .collect()
            }
            None => Vec::new(),
        }
    }

    async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel> {
        self.cache.load().get(channel_id).cloned()
    }

    async fn reload(&self) {
        Self::reload(self).await;
    }
}

/// Parameters for SSE AuditTask construction
pub struct SseAuditParams {
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: llm_gateway_storage::Protocol,
    pub request_body: String,
    pub pricing_policy_config: Option<serde_json::Value>,
    pub pricing_policy_billing_type: String,
    pub markup_ratio: f64,
    pub channel_id: String,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: String,
    pub upstream_url: String,
    pub request_headers: String,
    pub response_headers: String,
}

/// SSE passthrough with concurrent accumulation for DB logging.
async fn process_sse_stream(
    upstream_resp: reqwest::Response,
    tx: mpsc::Sender<Result<Event, std::convert::Infallible>>,
    audit_tx: mpsc::Sender<AuditTask>,
    audit_params: SseAuditParams,
    start: Instant,
) {
    let byte_stream = upstream_resp.bytes_stream();
    tokio::pin!(byte_stream);
    let mut line_buf = String::new();
    let mut accumulated = String::new();

    'outer: loop {
        match futures::TryStreamExt::try_next(&mut byte_stream).await {
            Ok(Some(chunk)) => {
                // Forward raw bytes to client immediately
                if tx.send(Ok(Event::default().data(String::from_utf8_lossy(&chunk).as_ref()))).await.is_err() {
                    tracing::debug!("[PROXY] client disconnected");
                    break;
                }

                // Accumulate for audit
                line_buf.push_str(&String::from_utf8_lossy(&chunk));

                // SSE events delimited by double newline
                loop {
                    match line_buf.find("\n\n") {
                        None => break,
                        Some(pos) => {
                            let raw_event = line_buf[..pos].to_owned();
                            line_buf = line_buf[pos + 2..].to_owned();

                            // Parse event block
                            let mut event_type: Option<String> = None;
                            let mut event_id: Option<String> = None;
                            let mut data_parts: Vec<String> = Vec::new();

                            for line in raw_event.lines() {
                                if let Some(part) = line.strip_prefix("data:") {
                                    data_parts.push(part.trim_start().to_owned());
                                } else if let Some(et) = line.strip_prefix("event:") {
                                    event_type = Some(et.trim_start().to_owned());
                                } else if let Some(id) = line.strip_prefix("id:") {
                                    event_id = Some(id.trim_start().to_owned());
                                }
                            }

                            if data_parts.is_empty() {
                                continue;
                            }

                            let data = data_parts.join("\n");

                            // Build full event block for audit (preserve event/id lines)
                            let mut full_block = String::new();
                            if let Some(ref et) = event_type {
                                full_block.push_str("event:");
                                full_block.push_str(et);
                                full_block.push('\n');
                            }
                            if let Some(ref id) = event_id {
                                full_block.push_str("id:");
                                full_block.push_str(id);
                                full_block.push('\n');
                            }
                            for dp in &data_parts {
                                full_block.push_str("data:");
                                full_block.push_str(dp);
                                full_block.push('\n');
                            }
                            accumulated.push_str(&full_block);

                            // Forward to client
                            let mut sse_event = Event::default().data(&data);
                            if let Some(et) = event_type {
                                sse_event = sse_event.event(et);
                            }
                            if let Some(id) = event_id {
                                sse_event = sse_event.id(id);
                            }
                            if tx.send(Ok(sse_event)).await.is_err() {
                                tracing::debug!("[PROXY] client disconnected");
                                break 'outer;
                            }

                            if data == "[DONE]" {
                                break 'outer;
                            }
                        }
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                tracing::warn!("[PROXY] SSE upstream read error: {}", e);
                break;
            }
        }
    }

    drop(tx);
    let latency_ms = start.elapsed().as_millis() as i64;

    tracing::info!("[PROXY] SSE stream complete, latency={}ms", latency_ms);

    let task = AuditTask {
        key_id: audit_params.key_id,
        model_name: audit_params.model_name,
        provider_id: audit_params.provider_id,
        protocol: audit_params.protocol,
        stream: true,
        request_body: audit_params.request_body,
        response_bytes: accumulated.into_bytes(),
        status_code: 200,
        latency_ms,
        pricing_policy_config: audit_params.pricing_policy_config,
        pricing_policy_billing_type: audit_params.pricing_policy_billing_type,
        markup_ratio: audit_params.markup_ratio,
        channel_id: Some(audit_params.channel_id),
        original_model: audit_params.original_model,
        upstream_model: audit_params.upstream_model,
        model_override_reason: audit_params.model_override_reason,
        request_path: Some(audit_params.request_path),
        upstream_url: Some(audit_params.upstream_url),
        request_headers: Some(audit_params.request_headers),
        response_headers: Some(audit_params.response_headers),
    };
    if let Err(e) = audit_tx.send(task).await {
        tracing::warn!("[PROXY] Failed to send SSE audit task: {}", e);
    }
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

        tracing::debug!("[PROXY] Upstream response: channel={}, status={}, latency={}ms, content-encoding={:?}", channel.name, status, latency_ms, resp.headers().get("content-encoding"));

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

        // Build request headers JSON (for audit log — only forwarded headers, exclude sensitive ones)
        let request_headers_for_worker: String = {
            let mut map = serde_json::Map::new();
            for (name, value) in headers.iter() {
                match name.as_str() {
                    "host" | "authorization" | "content-length" | "x-api-key" | "api-key" => continue,
                    _ => {
                        if let Ok(v) = value.to_str() {
                            map.insert(name.to_string(), serde_json::Value::String(v.to_string()));
                        }
                    }
                }
            }
            serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
        };

        // === Handle streaming: SSE passthrough with mpsc channel architecture ===
        if is_stream {
            // Capture upstream response headers (must happen before bytes_stream consumes resp)
            let response_headers_for_worker: String = {
                let mut map = serde_json::Map::new();
                for (name, value) in resp.headers().iter() {
                    if let Ok(v) = value.to_str() {
                        map.insert(name.to_string(), serde_json::Value::String(v.to_string()));
                    }
                }
                serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
            };
            let upstream_resp_headers: Vec<_> = resp.headers().iter().map(|(n, v)| (n.clone(), v.clone())).collect();

            let proto = match protocol {
                ProxyProtocol::OpenAI => llm_gateway_storage::Protocol::Openai,
                ProxyProtocol::Anthropic => llm_gateway_storage::Protocol::Anthropic,
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

            let audit_params = SseAuditParams {
                key_id: api_key.id.clone(),
                model_name: upstream_name.to_string(),
                provider_id: provider_id.clone(),
                protocol: proto,
                request_body: body.clone(),
                pricing_policy_config,
                pricing_policy_billing_type,
                markup_ratio: channel_model.markup_ratio,
                channel_id: channel.id.clone(),
                original_model: if upstream_name != &model_name { Some(model_name.clone()) } else { None },
                upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
                model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
                request_path: path.to_string(),
                upstream_url: upstream_url.clone(),
                request_headers: request_headers_for_worker.clone(),
                response_headers: response_headers_for_worker,
            };

            let audit_tx = state.audit_tx.clone();
            let upstream_resp = resp;
            let start = start;

            // mpsc channel for SSE events
            let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(256);

            tokio::spawn(process_sse_stream(
                upstream_resp,
                tx,
                audit_tx,
                audit_params,
                start,
            ));

            let event_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
            let sse_response = Sse::new(event_stream)
                .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive"))
                .into_response();

            // Forward upstream headers (except content-length which is dynamic for streams)
            let mut response = sse_response;
            for (name, value) in upstream_resp_headers {
                if name.as_str() == "content-length" {
                    continue;
                }
                response.headers_mut().insert(name.clone(), value.clone());
            }
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                "text/event-stream; charset=utf-8".parse().unwrap(),
            );

            return Ok(response);
        }

        // Non-streaming: send to audit worker (it parses JSON, calculates cost, writes DB)
        // Capture response headers before consuming resp
        let response_headers_for_worker: String = {
            let mut map = serde_json::Map::new();
            for (name, value) in resp.headers().iter() {
                if let Ok(v) = value.to_str() {
                    map.insert(name.to_string(), serde_json::Value::String(v.to_string()));
                }
            }
            serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
        };
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
            request_headers: Some(request_headers_for_worker.clone()),
            response_headers: Some(response_headers_for_worker),
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