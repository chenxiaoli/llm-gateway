use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;

use async_trait::async_trait;

use llm_gateway_auth::hash_api_key;
use llm_gateway_storage::Protocol;

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;
use crate::AuditTask;

// ─── Resolved Channel ────────────────────────────────────────────────────────

/// Per-model enrichment for a channel (upstream_model_name, pricing, markup).
#[derive(Clone, Debug)]
pub struct ChannelModelEnriched {
    pub upstream_model_name: Option<String>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
}

#[derive(Clone, Debug)]
pub struct ResolvedChannel {
    pub channel_id: Uuid,
    pub provider_id: String,
    pub name: String,
    /// Per-protocol endpoint base URLs (already includes provider-specific path segments).
    /// openai endpoint → append /chat/completions
    /// anthropic endpoint → append /messages
    pub endpoint_openai: Option<String>,
    pub endpoint_anthropic: Option<String>,
    pub upstream_api_key: String, // decrypted
    pub adapter: ProxyProtocol,
    pub timeout_ms: u64,
    pub priority: i32,
    /// Per-model overrides: keyed by lowercase model name.
    pub model_overrides: HashMap<String, ChannelModelEnriched>,
    pub proxy_url: Option<String>,
}

impl ResolvedChannel {
    /// Returns the upstream URL for the given protocol by appending
    /// the request path to the appropriate endpoint.
    pub fn upstream_url(&self, request_path: &str, protocol: ProxyProtocol) -> String {
        let base = match protocol {
            ProxyProtocol::OpenAI => self.endpoint_openai.as_deref(),
            ProxyProtocol::Anthropic => self.endpoint_anthropic.as_deref(),
        };
        let base = base.unwrap_or_default().trim_end_matches('/');
        format!("{}{}", base, request_path)
    }
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
        let all_storage_models = self.storage.list_models().await?;

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

            let endpoint_anthropic = endpoints
                .get("anthropic")
                .and_then(|v| v.as_str())
                .or_else(|| endpoints.get("default").and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            let proxy_url = provider.proxy_url.clone();

            let api_key = llm_gateway_encryption::decrypt(&channel.api_key, &self.encryption_key)
                .unwrap_or_else(|_| channel.api_key.clone());

            // Pre-enrich with per-model overrides (upstream_model_name, pricing, markup)
            let mut model_overrides: HashMap<String, ChannelModelEnriched> = HashMap::new();
            if let Some(cms) = cm_by_channel.get(channel.id.as_str()) {
                for cm in cms {
                    if !cm.enabled {
                        continue;
                    }
                    if let Some(model) = all_storage_models.iter().find(|m| m.model.id == cm.model_id) {
                        let model_name_lower = model.model.name.to_lowercase();
                        model_overrides.insert(
                            model_name_lower,
                            ChannelModelEnriched {
                                upstream_model_name: cm.upstream_model_name.clone(),
                                pricing_policy_id: cm.pricing_policy_id.clone().or_else(|| channel.pricing_policy_id.clone()),
                                markup_ratio: cm.markup_ratio,
                            },
                        );
                    }
                }
            }

            let resolved = ResolvedChannel {
                channel_id: Uuid::parse_str(&channel.id).unwrap_or_else(|_| Uuid::new_v4()),
                provider_id: channel.provider_id.clone(),
                name: channel.name.clone(),
                endpoint_openai,
                endpoint_anthropic,
                upstream_api_key: api_key,
                adapter: ProxyProtocol::OpenAI,
                timeout_ms: 60_000,
                priority: channel.priority,
                model_overrides,
                proxy_url,
            };

            cache.insert(channel.id.clone(), resolved);

            // Build model_index: model_name -> channel_ids
            if let Some(cms) = cm_by_channel.get(channel.id.as_str()) {
                for cm in cms {
                    if !cm.enabled {
                        continue;
                    }
                    if let Some(model) = all_storage_models.iter().find(|m| m.model.id == cm.model_id) {
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

/// Spawn the background refresh loop for a registry.
/// Call this after constructing InMemoryChannelRegistry, before converting to Arc<dyn ChannelRegistry>.
pub fn spawn_registry_refresh(registry: Arc<InMemoryChannelRegistry>) {
    tokio::spawn(async move {
        registry.start_refresh_loop().await;
    });
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

                            // Accumulate raw event block as-is for audit (exact upstream format)
                            accumulated.push_str(&raw_event);
                            accumulated.push_str("\n\n");

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
    request_path: String,
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

    // === Step 2: Balance check ===
    // Keys with created_by = None (e.g. admin-created test keys) skip balance checks.
    // A threshold of 0 means "no limit" — skip the check in that case.
    if let Some(ref created_by) = api_key.created_by {
        if let Some(account) = state
            .storage
            .get_account_by_user_id(created_by)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        {
            if account.threshold > 0.0 && account.balance < account.threshold {
                tracing::warn!(
                    "[PROXY] Balance check failed: user={}, balance={}, threshold={}",
                    created_by, account.balance, account.threshold
                );
                return Err(ApiError::PaymentRequired);
            }
        }
    }

    // === Step 3: Parse model ===
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

    // === Step 3: Route via ChannelRegistry (cache-first) ===
    let resolved_channels = state.registry.resolve_by_model(&model_name).await;

    let routing_candidates: Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)> = if !resolved_channels.is_empty() {
        // Cache hit: use pre-enriched per-model data (no DB call needed)
        let model_key = model_name.to_lowercase();
        let mut candidates = Vec::new();
        for rc in resolved_channels {
            if let Some(enriched) = rc.model_overrides.get(&model_key) {
                let cm = llm_gateway_storage::ChannelModel {
                    id: Uuid::new_v4().to_string(), // not used in routing path
                    channel_id: rc.channel_id.to_string(),
                    model_id: model_entry.model.id.clone(),
                    enabled: true,
                    upstream_model_name: enriched.upstream_model_name.clone(),
                    pricing_policy_id: enriched.pricing_policy_id.clone(),
                    markup_ratio: enriched.markup_ratio,
                    priority_override: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                candidates.push((rc, cm));
            }
        }
        if candidates.is_empty() {
            return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
        }
        // Already sorted by priority in do_reload()
        candidates
    } else {
        // Cache miss: use original DB routing logic
        let channel_models = state
            .storage
            .get_channel_models_for_model(&model_entry.model.id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let all_channels = state
            .storage
            .list_channels()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let channel_map: std::collections::HashMap<&str, &llm_gateway_storage::Channel> = all_channels
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect();

        // Find provider_id from first enabled channel_model + channel
        let provider_id = {
            let mut pid: Option<&str> = None;
            for cm in channel_models.iter().filter(|cm| cm.enabled) {
                if let Some(ch) = channel_map.get(cm.channel_id.as_str()) {
                    if ch.enabled {
                        pid = Some(ch.provider_id.as_str());
                        break;
                    }
                }
            }
            pid.ok_or_else(|| ApiError::Internal("No provider ID available".to_string()))?
        };

        let provider = state
            .storage
            .get_provider(provider_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

        let mut candidates: Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)> = Vec::new();
        for cm in channel_models.iter().filter(|cm| cm.enabled) {
            if let Some(channel) = channel_map.get(cm.channel_id.as_str()) {
                if channel.enabled {
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
                    let endpoint_anthropic = endpoints
                        .get("anthropic")
                        .and_then(|v| v.as_str())
                        .or_else(|| endpoints.get("default").and_then(|v| v.as_str()))
                        .map(|s| s.to_string());

                    let proxy_url = provider.proxy_url.clone();

                    let api_key = llm_gateway_encryption::decrypt(&channel.api_key, &state.encryption_key)
                        .unwrap_or_else(|_| channel.api_key.clone());

                    let resolved = ResolvedChannel {
                        channel_id: Uuid::parse_str(&channel.id).unwrap_or_else(|_| Uuid::new_v4()),
                        provider_id: channel.provider_id.clone(),
                        name: channel.name.clone(),
                        endpoint_openai,
                        endpoint_anthropic,
                        upstream_api_key: api_key,
                        adapter: protocol,
                        timeout_ms: 60_000,
                        priority: channel.priority,
                        model_overrides: HashMap::new(), // not used in cache-miss path
                        proxy_url,
                    };
                    candidates.push((resolved, cm.clone()));
                }
            }
        }
        candidates.sort_by(|a, b| b.0.priority.cmp(&a.0.priority));
        if candidates.is_empty() {
            return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
        }
        candidates
    };

    // Extract provider_id from resolved channel for audit/billing
    let provider_id = routing_candidates.first()
        .map(|(c, _)| c.provider_id.clone())
        .ok_or_else(|| ApiError::Internal("No channels available".to_string()))?;

    let default_client = reqwest::Client::new();
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

        let api_key_value = channel.upstream_api_key.clone();

        let upstream_url = channel.upstream_url(&request_path, protocol);

        tracing::debug!("[PROXY] Upstream URL: {} -> {}", request_path, upstream_url);
        let client: reqwest::Client = match &channel.proxy_url {
            Some(proxy_url) => {
                match reqwest::Proxy::all(proxy_url) {
                    Ok(proxy) => {
                        match reqwest::Client::builder().proxy(proxy).build() {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::warn!("[PROXY] Failed to build proxied client for channel '{}': {}", channel.name, e);
                                last_error = format!("Proxy client error on channel '{}': {}", channel.name, e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[PROXY] Invalid proxy URL '{}' for channel '{}': {}", proxy_url, channel.name, e);
                        last_error = format!("Invalid proxy URL on channel '{}': {}", channel.name, e);
                        continue;
                    }
                }
            }
            None => default_client.clone(),
        };
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
        // Disable upstream compression — we forward raw bytes directly so downstream
        // clients receive the exact stream format without double-decompression.
        req = req.header("Accept-Encoding", "identity");

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
            // Capture response headers before consuming body
            let response_headers_for_worker: String = {
                let mut map = serde_json::Map::new();
                for (name, value) in resp.headers().iter() {
                    if let Ok(v) = value.to_str() {
                        map.insert(name.to_string(), serde_json::Value::String(v.to_string()));
                    }
                }
                serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
            };

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

            let proto = match protocol {
                ProxyProtocol::OpenAI => Protocol::Openai,
                ProxyProtocol::Anthropic => Protocol::Anthropic,
            };

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

            let error_body = resp.text().await.unwrap_or_default();
            tracing::debug!("[PROXY] Upstream error response: status={}, body_len={}", status, error_body.len());

            let task = AuditTask {
                key_id: api_key.id.clone(),
                model_name: upstream_name.to_string(),
                provider_id: provider_id.clone(),
                protocol: proto,
                stream: is_stream,
                request_body: body.clone(),
                response_bytes: error_body.clone().into_bytes(),
                status_code: status as i32,
                latency_ms,
                pricing_policy_config,
                pricing_policy_billing_type,
                markup_ratio: channel_model.markup_ratio,
                channel_id: Some(channel.channel_id.to_string()),
                original_model: if upstream_name != &model_name { Some(model_name.clone()) } else { None },
                upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
                model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
                request_path: Some(request_path.clone()),
                upstream_url: Some(upstream_url.clone()),
                request_headers: Some(request_headers_for_worker),
                response_headers: Some(response_headers_for_worker),
            };
            let _ = state.audit_tx.try_send(task);

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
                channel_id: channel.channel_id.to_string(),
                original_model: if upstream_name != &model_name { Some(model_name.clone()) } else { None },
                upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
                model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
                request_path: request_path.clone(),
                upstream_url: upstream_url.clone(),
                request_headers: request_headers_for_worker.clone(),
                response_headers: response_headers_for_worker,
            };

            let audit_tx = state.audit_tx.clone();
            let upstream_resp = resp;

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
            channel_id: Some(channel.channel_id.to_string()),
            original_model: if upstream_name != &model_name { Some(model_name.clone()) } else { None },
            upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
            model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
            request_path: Some(request_path.clone()),
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
    proxy(State(state), headers, body, ProxyProtocol::OpenAI, "/v1/chat/completions".to_string()).await
}

/// Wrapper for /v1/messages - uses Anthropic protocol
pub async fn messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy(State(state), headers, body, ProxyProtocol::Anthropic, "/v1/messages".to_string()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stub registry that always returns fixed channels
    pub struct StubRegistry(pub Vec<ResolvedChannel>);

    #[async_trait::async_trait]
    impl ChannelRegistry for StubRegistry {
        async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel> {
            self.0.iter().find(|c| c.channel_id.to_string() == channel_id).cloned()
        }
        async fn resolve_by_model(&self, _model: &str) -> Vec<ResolvedChannel> {
            self.0.clone()
        }
        async fn reload(&self) {}
    }

    #[test]
    fn resolved_channel_carries_all_fields() {
        let model_key = "gpt-4o".to_lowercase();
        let rc = ResolvedChannel {
            channel_id: Uuid::new_v4(),
            provider_id: "prov-1".into(),
            name: "test-channel".into(),
            endpoint_openai: Some("https://api.openai.com/v1".into()),
            endpoint_anthropic: Some("https://api.anthropic.com".into()),
            upstream_api_key: "sk-test".into(),
            adapter: ProxyProtocol::OpenAI,
            timeout_ms: 30_000,
            priority: 1,
            model_overrides: HashMap::from([(
                model_key.clone(),
                ChannelModelEnriched {
                    upstream_model_name: Some("gpt-4o".into()),
                    pricing_policy_id: Some("policy-1".into()),
                    markup_ratio: 1.5,
                },
            )]),
            proxy_url: None,
        };
        let enriched = rc.model_overrides.get(&model_key).expect("should have model override");
        assert_eq!(enriched.markup_ratio, 1.5);
        assert_eq!(enriched.upstream_model_name.as_deref(), Some("gpt-4o"));
        assert_eq!(enriched.pricing_policy_id.as_deref(), Some("policy-1"));
        assert_eq!(rc.adapter, ProxyProtocol::OpenAI);
        assert_eq!(rc.provider_id, "prov-1");
        assert_eq!(rc.name, "test-channel");
    }

    #[tokio::test]
    async fn stub_registry_resolve_by_model() {
        let rc = ResolvedChannel {
            channel_id: Uuid::new_v4(),
            provider_id: "prov-1".into(),
            name: "test-channel".into(),
            endpoint_openai: Some("https://api.openai.com/v1".into()),
            endpoint_anthropic: None,
            upstream_api_key: "sk-test".into(),
            adapter: ProxyProtocol::OpenAI,
            timeout_ms: 30_000,
            priority: 1,
            model_overrides: HashMap::new(),
            proxy_url: None,
        };
        let registry = StubRegistry(vec![rc.clone()]);
        let result = registry.resolve_by_model("any-model").await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_id, "prov-1");
    }

    #[tokio::test]
    async fn stub_registry_resolve_by_id() {
        let channel_id = Uuid::new_v4();
        let rc = ResolvedChannel {
            channel_id,
            provider_id: "prov-2".into(),
            name: "channel-by-id".into(),
            endpoint_openai: None,
            endpoint_anthropic: Some("https://api.anthropic.com".into()),
            upstream_api_key: "sk-ant".into(),
            adapter: ProxyProtocol::Anthropic,
            timeout_ms: 60_000,
            priority: 5,
            model_overrides: HashMap::from([(
                "claude-3-5-sonnet".to_lowercase(),
                ChannelModelEnriched {
                    upstream_model_name: Some("claude-3-5-sonnet".into()),
                    pricing_policy_id: Some("policy-2".into()),
                    markup_ratio: 2.0,
                },
            )]),
            proxy_url: None,
        };
        let registry = StubRegistry(vec![rc.clone()]);
        let result = registry.resolve(&channel_id.to_string()).await;
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert_eq!(resolved.provider_id, "prov-2");
        assert_eq!(resolved.adapter, ProxyProtocol::Anthropic);
        let enriched = resolved.model_overrides.get("claude-3-5-sonnet").expect("should have model override");
        assert_eq!(enriched.markup_ratio, 2.0);
    }

    #[test]
    fn proxy_protocol_variants() {
        assert_ne!(ProxyProtocol::OpenAI, ProxyProtocol::Anthropic);
        let openai = ProxyProtocol::OpenAI;
        let anthropic = ProxyProtocol::Anthropic;
        assert!(matches!(openai, ProxyProtocol::OpenAI));
        assert!(matches!(anthropic, ProxyProtocol::Anthropic));
    }
}