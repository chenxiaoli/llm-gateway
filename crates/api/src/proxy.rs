use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use rand::Rng as _;
use axum::response::IntoResponse;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;

use async_trait::async_trait;

use llm_gateway_auth::hash_api_key;
use llm_gateway_storage::types::TimeSlot;
use llm_gateway_storage::Protocol;

use llm_gateway_nats_publisher::{AuditEvent, UsageEvent};

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
    pub markup_ratio: i64,
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
    pub weight: Option<i32>,
    /// Per-model overrides: keyed by lowercase model name.
    pub model_overrides: HashMap<String, ChannelModelEnriched>,
    pub proxy_url: Option<String>,
    pub group_id: Option<String>,
    pub available_hours: Option<Vec<TimeSlot>>,
}

/// Builds the upstream URL by combining a base endpoint with the request path,
/// stripping the `/v1` prefix for OpenAI protocol.
pub fn build_upstream_url(base_url: &str, request_path: &str, protocol: ProxyProtocol) -> String {
    let base = base_url.trim_end_matches('/');
    let suffix = match protocol {
        // Strip /v1 prefix for OpenAI protocol - endpoints already include /v1
        ProxyProtocol::OpenAI => request_path.strip_prefix("/v1").unwrap_or(request_path),
        ProxyProtocol::Anthropic => request_path,
    };
    format!("{}{}", base, suffix)
}

impl ResolvedChannel {
    /// Returns the upstream URL for the given protocol.
    pub fn upstream_url(&self, request_path: &str, protocol: ProxyProtocol) -> String {
        let base = match protocol {
            ProxyProtocol::OpenAI => self.endpoint_openai.as_deref(),
            ProxyProtocol::Anthropic => self.endpoint_anthropic.as_deref(),
        };
        build_upstream_url(base.unwrap_or_default(), request_path, protocol)
    }
}

// ─── Channel Registry ────────────────────────────────────────────────────────

#[async_trait]
pub trait ChannelRegistry: Send + Sync {
    async fn resolve_by_model(&self, model: &str) -> Vec<ResolvedChannel>;
    async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel>;
    async fn reload(&self);
    fn disable_channel_model(&self, channel_id: &str, model_name: &str, until: Instant);
    fn is_circuit_broken(&self, channel_id: &str, model_name: &str) -> bool;
}

/// Protocol for determining which adapter to use
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyProtocol {
    OpenAI,
    Anthropic,
}

// ─── InMemoryChannelRegistry ─────────────────────────────────────────────────

use arc_swap::ArcSwap;
use dashmap::DashMap;

pub struct InMemoryChannelRegistry {
    cache: Arc<ArcSwap<HashMap<String, ResolvedChannel>>>,
    model_index: Arc<ArcSwap<HashMap<String, Vec<String>>>>,
    storage: Arc<dyn llm_gateway_storage::Storage>,
    encryption_key: [u8; 32],
    refresh_interval: Duration,
    circuit_breaker: Arc<DashMap<(String, String), Instant>>,
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
            circuit_breaker: Arc::new(DashMap::new()),
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

            // Skip channels that are temporarily disabled (e.g., due to SSE errors)
            if let Some(disabled_until) = channel.disabled_until {
                if disabled_until > Utc::now() {
                    tracing::debug!("[PROXY] Channel '{}' is temporarily disabled until {}", channel.name, disabled_until);
                    continue;
                }
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
                weight: channel.weight,
                model_overrides,
                proxy_url,
                group_id: channel.group_id.clone(),
                available_hours: channel.available_hours.clone(),
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

        self.cache.store(Arc::new(cache));
        self.model_index.store(Arc::new(model_index));

        Ok(())
    }
}

fn is_available_now(slots: &Option<Vec<TimeSlot>>) -> bool {
    let slots = match slots {
        Some(s) if !s.is_empty() => s,
        _ => return true,
    };
    let now = chrono::Utc::now();
    let today = now.format("%a").to_string().to_lowercase();
    let now_minutes = now.format("%H").to_string().parse::<i32>().unwrap_or(0) * 60
        + now.format("%M").to_string().parse::<i32>().unwrap_or(0);

    slots.iter().any(|slot| {
        if !slot.days.contains(&today) {
            return false;
        }
        let parse_time = |t: &str| -> i32 {
            let parts: Vec<&str> = t.split(':').take(2).collect();
            let h: i32 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
            let m: i32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
            h * 60 + m
        };
        let start = parse_time(&slot.start);
        let end = parse_time(&slot.end);
        if start <= end {
            now_minutes >= start && now_minutes < end
        } else {
            now_minutes >= start || now_minutes < end
        }
    })
}

fn select_weighted_order<T>(items: &mut Vec<T>, weight_fn: impl Fn(&T) -> i32) {
    let mut rng = rand::rng();
    for i in 0..items.len().saturating_sub(1) {
        let total_weight: i32 = items[i..].iter().map(|item| weight_fn(item).max(1)).sum();
        if total_weight == 0 {
            continue;
        }
        let mut roll = rng.random_range(0..total_weight);
        let mut selected = i;
        for (j, item) in items[i..].iter().enumerate() {
            roll -= weight_fn(item).max(1);
            if roll < 0 {
                selected = i + j;
                break;
            }
        }
        items.swap(i, selected);
    }
}

fn apply_weighted_routing(candidates: &mut Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)>) {
    candidates.sort_by(|a, b| a.0.priority.cmp(&b.0.priority));
    let mut result = Vec::with_capacity(candidates.len());
    let mut i = 0;
    while i < candidates.len() {
        let prio = candidates[i].0.priority;
        let start = i;
        while i < candidates.len() && candidates[i].0.priority == prio {
            i += 1;
        }
        let mut tier: Vec<_> = candidates[start..i].to_vec();
        if tier.len() > 1 {
            select_weighted_order(&mut tier, |c| c.0.weight.unwrap_or(100));
        }
        result.extend(tier);
    }
    candidates.clear();
    candidates.extend(result);
}

#[async_trait::async_trait]
impl ChannelRegistry for InMemoryChannelRegistry {
    async fn resolve_by_model(&self, model: &str) -> Vec<ResolvedChannel> {
        let model_key = model.to_lowercase();
        let now = Instant::now();

        // Lazy cleanup of expired circuit breaker entries
        self.circuit_breaker.retain(|_, until| *until > now);

        let channel_ids = self.model_index.load().get(&model_key).cloned();
        match channel_ids {
            Some(ids) => {
                let cache = self.cache.load();
                ids.iter()
                    .filter_map(|id| cache.get(id).cloned())
                    .filter(|ch| is_available_now(&ch.available_hours))
                    .filter(|ch| {
                        let key = (ch.channel_id.to_string(), model_key.clone());
                        match self.circuit_breaker.get(&key) {
                            Some(until) => now >= *until,
                            None => true,
                        }
                    })
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

    fn disable_channel_model(&self, channel_id: &str, model_name: &str, until: Instant) {
        let model_lower = model_name.to_lowercase();
        tracing::info!(
            "[CIRCUIT-BREAKER] Disabling channel_id={}, model={} until {:?}",
            channel_id, model_lower, until
        );
        self.circuit_breaker.insert(
            (channel_id.to_string(), model_lower),
            until,
        );
    }

    fn is_circuit_broken(&self, channel_id: &str, model_name: &str) -> bool {
        let model_lower = model_name.to_lowercase();
        match self.circuit_breaker.get(&(channel_id.to_string(), model_lower.clone())) {
            Some(until) => Instant::now() < *until,
            None => false,
        }
    }
}

/// Spawn the background refresh loop for a registry.
/// Call this after constructing InMemoryChannelRegistry, before converting to Arc<dyn ChannelRegistry>.
pub fn spawn_registry_refresh(registry: Arc<InMemoryChannelRegistry>) {
    tokio::spawn(async move {
        registry.start_refresh_loop().await;
    });
}

async fn publish_audit_events(
    state: &Arc<crate::AppState>,
    task: &crate::AuditTask,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
    cost: i64,
) {
    let nats = match &state.nats_publisher {
        Some(n) => n,
        None => return,
    };
    let now = chrono::Utc::now();

    let pricing_policy_json = task.pricing_policy.as_ref()
        .map(|p| serde_json::to_value(p).unwrap_or_default());
    let (pp_config, pp_billing_type) = match &task.pricing_policy {
        Some(p) => (Some(p.config.clone()), p.billing_type.as_str()),
        None => (None, "per_token"),
    };

    let usage_event = UsageEvent {
        id: uuid::Uuid::new_v4().to_string(),
        request_id: task.request_id.clone(),
        key_id: task.key_id.clone(),
        user_id: task.user_id.clone(),
        model_name: task.model_name.clone(),
        provider_id: task.provider_id.clone(),
        channel_id: task.channel_id.clone(),
        protocol: format!("{:?}", task.protocol).to_lowercase(),
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_creation_tokens,
        cost,
        pricing_policy: pricing_policy_json,
        weighted_tokens: crate::workers::calculate_weighted_tokens(
            &pp_config,
            pp_billing_type,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
        ),
        latency_ms: task.latency_ms,
        created_at: now.to_rfc3339(),
    };

    let audit_event = AuditEvent {
        id: uuid::Uuid::new_v4().to_string(),
        request_id: task.request_id.clone(),
        key_id: task.key_id.clone(),
        user_id: task.user_id.clone(),
        model_name: task.model_name.clone(),
        provider_id: task.provider_id.clone(),
        channel_id: task.channel_id.clone(),
        protocol: format!("{:?}", task.protocol).to_lowercase(),
        stream: task.stream,
        status_code: task.status_code,
        latency_ms: task.latency_ms,
        original_model: task.original_model.clone(),
        upstream_model: task.upstream_model.clone(),
        model_override_reason: task.model_override_reason.clone(),
        request_path: task.request_path.clone(),
        upstream_url: task.upstream_url.clone(),
        request_body: task.request_body.clone(),
        response_body: String::from_utf8_lossy(&task.response_bytes)
            .chars()
            .map(|c| if c == '\0' || c == '\u{FFFD}' { ' ' } else { c })
            .collect::<String>(),
        request_headers: task.request_headers.clone(),
        response_headers: task.response_headers.clone(),
        created_at: now.to_rfc3339(),
        routes: Some(task.routes.clone()),
    };

    if let Err(e) = nats.publish_usage(&usage_event).await {
        tracing::warn!("[NATS] Failed to publish usage event: {}", e);
    }
    tracing::info!("[NATS] publishing audit event request_id={} status={} cost={}",
        audit_event.request_id, audit_event.status_code, cost);
    if let Err(e) = nats.publish_audit(&audit_event).await {
        tracing::warn!("[NATS] Failed to publish audit event: {}", e);
    }
}

async fn dispatch_audit_task(
    state: &Arc<crate::AppState>,
    task: crate::AuditTask,
) {
    let proto = task.protocol.clone();
    let stream = task.stream;
    let (input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens) =
        crate::workers::parse_usage(&task.response_bytes, stream, proto);
    let (pp_config, pp_billing_type) = match &task.pricing_policy {
        Some(p) => (Some(p.config.clone()), p.billing_type.clone()),
        None => (None, "per_token".to_string()),
    };
    let cost = crate::workers::calculate_cost(
        &pp_config,
        &pp_billing_type,
        task.markup_ratio,
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_creation_tokens,
    );
    publish_audit_events(state, &task, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost).await;
}

/// Parameters for SSE AuditTask construction
pub struct SseAuditParams {
    pub request_id: String,
    pub key_id: String,
    pub user_id: Option<String>,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: llm_gateway_storage::Protocol,
    pub request_body: String,
    pub pricing_policy: Option<llm_gateway_storage::PricingPolicy>,
    pub markup_ratio: i64,
    pub channel_id: String,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: String,
    pub upstream_url: String,
    pub request_headers: String,
    pub response_headers: String,
    /// Per-upstream-attempt history collected before this streaming attempt
    /// (failed attempts that came before this success). The success entry
    /// itself is appended at stream end.
    pub routes: Vec<llm_gateway_storage::RouteAttempt>,
    /// Channel name captured for the success RouteAttempt at stream end.
    pub channel_name: Option<String>,
    /// Wall-clock start of the first attempt (for total latency).
    pub start_total: Instant,
    /// The per-attempt `Instant` when this streaming attempt started.
    pub attempt_start: Instant,
    /// Body sent to upstream (the client-original body, possibly model-rewritten).
    pub modified_body: String,
}

/// Parse recovery timestamp from SSE error message.
/// Looks for patterns like "将在 2026-05-21 20:30:44 重置" (Chinese) or
/// "reset at YYYY-MM-DD HH:MM:SS", "will reset at ..." and returns the parsed datetime.
/// Returns None if no timestamp found (uses default disable duration instead).
fn parse_recovery_timestamp(error_message: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Try to find Chinese pattern: "将在 YYYY-MM-DD HH:MM:SS 重置"
    if let Some(caps) = regex_lite::Regex::new(r"将在\s*(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})")
        .ok()?
        .captures(error_message)
    {
        let ts_str = caps.get(1)?.as_str();
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S") {
            return Some(chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc));
        }
    }

    // Try ISO 8601 pattern
    if let Some(caps) = regex_lite::Regex::new(r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})")
        .ok()?
        .captures(error_message)
    {
        let ts_str = caps.get(1)?.as_str();
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts_str) {
            return Some(dt.with_timezone(&chrono::Utc));
        }
    }

    None
}

/// SSE passthrough with concurrent accumulation for DB logging.
async fn process_sse_stream(
    upstream_resp: reqwest::Response,
    tx: mpsc::Sender<Result<Event, std::convert::Infallible>>,
    state: Arc<crate::AppState>,
    audit_params: SseAuditParams,
) {
    let byte_stream = upstream_resp.bytes_stream();
    tokio::pin!(byte_stream);
    let mut line_buf = String::new();
    let mut accumulated = String::new();

    'outer: loop {
        let mut saw_error = false;
        let mut error_message = String::new();

        match futures::TryStreamExt::try_next(&mut byte_stream).await {
            Ok(Some(chunk)) => {
                // Accumulate for audit
                line_buf.push_str(&String::from_utf8_lossy(&chunk).replace('\0', " "));

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
                                sse_event = sse_event.event(et.clone());
                                // Detect SSE error events
                                if et == "error" {
                                    saw_error = true;
                                    error_message = data.clone();
                                }
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

        // Handle SSE error event: disable channel and break
        if saw_error {
            tracing::warn!(
                "[PROXY] SSE error event received on channel '{}' model '{}': {}",
                audit_params.channel_id,
                audit_params.model_name,
                error_message,
            );

            // Circuit-break the (channel, model) combination
            let recovery_instant = match parse_recovery_timestamp(&error_message) {
                Some(ts) => {
                    let dur = (ts - Utc::now()).max(chrono::Duration::seconds(5));
                    Instant::now() + dur.to_std().unwrap_or(Duration::from_secs(300))
                }
                None => Instant::now() + Duration::from_secs(300),
            };
            state.registry.disable_channel_model(
                &audit_params.channel_id,
                &audit_params.model_name,
                recovery_instant,
            );

            break 'outer;
        }
    }

    drop(tx);
    let latency_ms = audit_params.start_total.elapsed().as_millis() as i64;
    let attempt_latency_ms = audit_params.attempt_start.elapsed().as_millis() as i64;

    tracing::info!("[PROXY] SSE stream complete, latency={}ms", latency_ms);

    // Build the success RouteAttempt and append it to the history collected
    // before this successful attempt. This produces the full routes array
    // for the single AuditTask dispatched at stream end.
    let mut routes = audit_params.routes.clone();
    routes.push(llm_gateway_storage::RouteAttempt {
        model: audit_params.model_name.clone(),
        channel_id: audit_params.channel_id.clone(),
        channel_name: audit_params.channel_name.clone(),
        status_code: 200,
        error_message: None,
        latency_ms: attempt_latency_ms,
        started_at: chrono::Utc::now(),
    });

    let task = AuditTask {
        request_id: audit_params.request_id,
        key_id: audit_params.key_id,
        user_id: audit_params.user_id,
        model_name: audit_params.model_name,
        provider_id: audit_params.provider_id,
        protocol: audit_params.protocol,
        stream: true,
        request_body: audit_params.modified_body,
        response_bytes: accumulated.into_bytes(),
        status_code: 200,
        latency_ms,
        pricing_policy: audit_params.pricing_policy,
        markup_ratio: audit_params.markup_ratio,
        channel_id: Some(audit_params.channel_id),
        original_model: audit_params.original_model,
        upstream_model: audit_params.upstream_model,
        model_override_reason: audit_params.model_override_reason,
        request_path: Some(audit_params.request_path),
        upstream_url: Some(audit_params.upstream_url),
        request_headers: Some(audit_params.request_headers),
        response_headers: Some(audit_params.response_headers),
        routes,
    };
    dispatch_audit_task(&state, task).await;
}

/// Try fallback models when the initial model fails to route.
/// Returns Some(response) if a fallback succeeded, None if no fallback available.
fn try_model_fallback<'a>(
    state: &'a Arc<AppState>,
    headers: &'a HeaderMap,
    body: &'a str,
    original_model: &'a str,
    api_key: &'a llm_gateway_storage::ApiKey,
    protocol: ProxyProtocol,
    request_path: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<axum::response::Response>> + Send + 'a>> {
    Box::pin(async move {
        let fallback_id = match api_key.model_fallback_id.as_ref() {
            Some(id) => id,
            None => {
                tracing::debug!("[PROXY] No model_fallback_id on API key '{}' — fallback skipped", api_key.id);
                return None;
            }
        };
        let fallback_config = match state.storage.get_model_fallback(fallback_id).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                tracing::warn!("[PROXY] model_fallback_id '{}' not found in DB — fallback skipped", fallback_id);
                return None;
            }
            Err(e) => {
                tracing::warn!("[PROXY] Failed to load model_fallback_id '{}': {:?} — fallback skipped", fallback_id, e);
                return None;
            }
        };
        let model_lower = original_model.to_lowercase();

        let group = match fallback_config.config.iter().find(|g| {
            g.models.iter().any(|m| m.to_lowercase() == model_lower)
        }) {
            Some(g) => g,
            None => {
                tracing::debug!(
                    "[PROXY] No fallback group contains model '{}' (config '{}' has {} groups) — fallback skipped",
                    original_model, fallback_config.name, fallback_config.config.len()
                );
                return None;
            }
        };

        let mut fallbacks: Vec<(&String, i32)> = group
            .models
            .iter()
            .zip(group.priorities.iter())
            .filter(|(m, _)| m.to_lowercase() != model_lower)
            .map(|(m, p)| (m, *p))
            .collect();
        fallbacks.sort_by_key(|(_, p)| *p);

        for (fallback_model, _) in fallbacks {
            tracing::info!("[PROXY] Trying fallback model '{}' for failed '{}'", fallback_model, original_model);

            let fallback_body = {
                let req_json: serde_json::Value = match serde_json::from_str(body) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("[PROXY] Failed to parse request body for fallback: {} — fallback skipped", e);
                        return None;
                    }
                };
                let mut modified = req_json;
                if let Some(model_obj) = modified.get_mut("model") {
                    *model_obj = serde_json::Value::String(fallback_model.to_string());
                }
                serde_json::to_string(&modified).unwrap_or_else(|_| body.to_string())
            };

            // Box the recursive proxy call to satisfy the compiler's sizing requirement
            let result = Box::pin(proxy_inner(
                state.clone(),
                headers.clone(),
                fallback_body,
                protocol,
                request_path.to_string(),
                1, // fallback depth — prevents re-triggering fallback in recursive call
            )).await;

            match result {
                Ok(response) => {
                    tracing::info!("[PROXY] Fallback to '{}' succeeded", fallback_model);
                    return Some(response);
                }
                Err(e) => {
                    tracing::warn!("[PROXY] Fallback '{}' failed: {:?}", fallback_model, e);
                    continue;
                }
            }
        }

        None
    })
}

/// Unified proxy: receives request → forwards to upstream → returns response
/// Usage/Cost/Audit are handled in spawned async tasks
async fn proxy_inner(
    state: Arc<AppState>,
    headers: HeaderMap,
    body: String,
    protocol: ProxyProtocol,
    request_path: String,
    fallback_depth: u32,
) -> Result<axum::response::Response, ApiError> {
    let request_id = uuid::Uuid::new_v4().to_string();

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
            if account.threshold > 0 && account.balance < account.threshold {
                tracing::warn!(
                    "[PROXY] Balance check failed: user={}, balance={}, threshold={}",
                    created_by, account.balance, account.threshold
                );
                return Err(ApiError::PaymentRequired);
            }
        }
    }

    // === Step 2.5: Determine request user_id and is_admin for routing filter ===
    let request_user_id = api_key.created_by.clone();
    let request_is_admin = if let Some(ref uid) = request_user_id {
        match state.storage.get_user(uid).await {
            Ok(Some(u)) => u.role == "admin",
            Ok(None) => false,
            Err(e) => {
                tracing::warn!("[PROXY] Failed to look up user role for {}: {}", uid, e);
                false  // Fail-safe: treat as non-admin if lookup fails
            }
        }
    } else {
        false  // No user_id (legacy admin-created keys) — no filter applied
    };

    // === Step 3: Parse model ===
    let req_json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let model_name = req_json
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or(ApiError::BadRequest("Missing 'model' field".to_string()))?
        .to_string();

    tracing::debug!("[PROXY] Incoming request, model: {}, protocol: {:?}", model_name, protocol);

    // Capture the client's request BEFORE model_name is reassigned to the channel
    // canonical form below. audit_log.original_model must reflect what the client
    // asked for, not what the gateway mapped it to.
    let client_requested_model = model_name.clone();

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

    let model_entry = match models
        .iter()
        .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
    {
        Some(m) => m,
        None => {
            if fallback_depth == 0 {
                if let Some(resp) = try_model_fallback(
                    &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
                ).await {
                    return Ok(resp);
                }
            }
            return Err(ApiError::NotFound(format!("Model '{}' not found", model_name)));
        }
    };

    // Normalize model_name to database canonical form for consistent usage/audit records
    let model_name = model_entry.model.name.clone();

    tracing::debug!("[PROXY] Found model: {} (id: {})", model_entry.model.name, model_entry.model.id);

    // === Step 3: Route via ChannelRegistry (cache-first) ===
    let resolved_channels = state.registry.resolve_by_model(&model_name).await;

    let mut routing_candidates: Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)> = if !resolved_channels.is_empty() {
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
            if fallback_depth == 0 {
                if let Some(resp) = try_model_fallback(
                    &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
                ).await {
                    return Ok(resp);
                }
            }
            return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
        }
        // Apply user-group routing filter
        if let Some(ref user_id) = request_user_id {
            if !request_is_admin {
                match state.storage.get_user_group_id(user_id).await {
                    Ok(Some(allowed_group_id)) => {
                        candidates.retain(|(rc, _)| {
                            rc.group_id.is_none() || rc.group_id.as_deref() == Some(&allowed_group_id)
                        });
                    }
                    Ok(None) => { /* User has no group — unrestricted */ }
                    Err(e) => {
                        tracing::warn!("[PROXY] Failed to look up group for user {}: {}", user_id, e);
                        /* Fail-open: don't filter */
                    }
                }
            }
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

        // Collect enabled + available channels first
        let mut available_channels: Vec<(&llm_gateway_storage::ChannelModel, &llm_gateway_storage::Channel)> =
            channel_models.iter()
                .filter(|cm| cm.enabled)
                .filter_map(|cm| {
                    channel_map.get(cm.channel_id.as_str()).and_then(|ch| {
                        if ch.enabled && is_available_now(&ch.available_hours) {
                            Some((cm, *ch))
                        } else {
                            None
                        }
                    })
                })
                .collect();

        // Apply user-group routing filter
        if let Some(ref user_id) = request_user_id {
            if !request_is_admin {
                if let Ok(Some(allowed_group_id)) = state.storage.get_user_group_id(user_id).await {
                    available_channels.retain(|(_, ch)| {
                        ch.group_id.is_none() || ch.group_id.as_deref() == Some(&allowed_group_id)
                    });
                }
            }
        }

        if available_channels.is_empty() {
            if fallback_depth == 0 {
                if let Some(resp) = try_model_fallback(
                    &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
                ).await {
                    return Ok(resp);
                }
            }
            return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
        }

        let provider_id = available_channels[0].1.provider_id.as_str();
        let provider = state
            .storage
            .get_provider(provider_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

        let mut candidates: Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)> = Vec::new();
        for (cm, channel) in &available_channels {
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
                weight: channel.weight,
                model_overrides: HashMap::new(), // not used in cache-miss path
                proxy_url,
                group_id: channel.group_id.clone(),
                available_hours: channel.available_hours.clone(),
            };
            candidates.push((resolved, (*cm).clone()));
        }
        candidates.sort_by(|a, b| a.0.priority.cmp(&b.0.priority));
        // Filter out circuit-broken (channel, model) combinations
        let model_lower = model_name.to_lowercase();
        candidates.retain(|(ch, _)| {
            !state.registry.is_circuit_broken(&ch.channel_id.to_string(), &model_lower)
        });
        if candidates.is_empty() {
            if fallback_depth == 0 {
                if let Some(resp) = try_model_fallback(
                    &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
                ).await {
                    return Ok(resp);
                }
            }
            return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
        }
        candidates
    };

    apply_weighted_routing(&mut routing_candidates);

    // Extract provider_id from resolved channel for audit/billing
    let provider_id = routing_candidates.first()
        .map(|(c, _)| c.provider_id.clone())
        .ok_or_else(|| ApiError::Internal("No channels available".to_string()))?;

    let default_client = reqwest::Client::new();
    let mut last_error = String::new();

    // === Route with failover ===
    // Collect per-attempt history for the audit row. One AuditTask is dispatched
    // at loop exit (per-branch) carrying the full routes array; today's per-attempt
    // dispatches are removed.
    let mut routes: Vec<llm_gateway_storage::RouteAttempt> = Vec::new();
    let start_total = Instant::now();

    for (channel, channel_model) in &routing_candidates {
        // Use upstream_model_name if provided, otherwise use the model name from request
        let upstream_name = channel_model.upstream_model_name.as_deref().unwrap_or(&model_name);

        let modified_body = {
            let mut req_json_modified = req_json.clone();
            if upstream_name != &model_name {
                if let Some(model_obj) = req_json_modified.get_mut("model") {
                    *model_obj = serde_json::Value::String(upstream_name.to_string());
                }
            }
            if is_stream && matches!(protocol, ProxyProtocol::OpenAI) && req_json_modified.get("stream_options").is_none() {
                if let Some(obj) = req_json_modified.as_object_mut() {
                    obj.insert(
                        "stream_options".to_string(),
                        serde_json::json!({"include_usage": true}),
                    );
                }
            }
            serde_json::to_string(&req_json_modified).unwrap_or_else(|_| body.clone())
        };

        let api_key_value = channel.upstream_api_key.clone();

        let upstream_url = channel.upstream_url(&request_path, protocol);

        tracing::debug!("[PROXY] Upstream URL: {} -> {}", request_path, upstream_url);
        let proxy_build_start = Instant::now();
        let client: reqwest::Client = match &channel.proxy_url {
            Some(proxy_url) => {
                match reqwest::Proxy::all(proxy_url) {
                    Ok(proxy) => {
                        match reqwest::Client::builder().proxy(proxy).build() {
                            Ok(c) => c,
                            Err(e) => {
                                let proxy_build_latency_ms = proxy_build_start.elapsed().as_millis() as i64;
                                tracing::warn!("[PROXY] Failed to build proxied client for channel '{}': {}", channel.name, e);
                                let error_msg = format!("Proxy client error on channel '{}': {}", channel.name, e);
                                last_error = error_msg.clone();
                                // Record this failed attempt in the routes array so the audit
                                // row reflects the proxy-build failure (not silently dropped).
                                routes.push(llm_gateway_storage::RouteAttempt {
                                    model: upstream_name.to_string(),
                                    channel_id: channel.channel_id.to_string(),
                                    channel_name: Some(channel.name.clone()),
                                    status_code: 0,
                                    error_message: Some(error_msg),
                                    latency_ms: proxy_build_latency_ms,
                                    started_at: chrono::Utc::now(),
                                });
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        let proxy_build_latency_ms = proxy_build_start.elapsed().as_millis() as i64;
                        tracing::warn!("[PROXY] Invalid proxy URL '{}' for channel '{}': {}", proxy_url, channel.name, e);
                        let error_msg = format!("Invalid proxy URL on channel '{}': {}", channel.name, e);
                        last_error = error_msg.clone();
                        // Record this failed attempt in the routes array so the audit
                        // row reflects the proxy-build failure (not silently dropped).
                        routes.push(llm_gateway_storage::RouteAttempt {
                            model: upstream_name.to_string(),
                            channel_id: channel.channel_id.to_string(),
                            channel_name: Some(channel.name.clone()),
                            status_code: 0,
                            error_message: Some(error_msg),
                            latency_ms: proxy_build_latency_ms,
                            started_at: chrono::Utc::now(),
                        });
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

        // Forward client headers to upstream with a blacklist — skip hop-by-hop,
        // auth, content-length, and browser-specific headers that can confuse upstream APIs.
        for (name, value) in headers.iter() {
            let lower = name.as_str().to_lowercase();
            match lower.as_str() {
                // Auth / identity
                "host" | "authorization" | "content-length" | "x-api-key" | "api-key" => continue,
                // Content negotiation already set by proxy
                "content-type" | "accept" | "accept-encoding" | "accept-language" => continue,
                // Connection / transport
                "connection" | "upgrade" | "keep-alive" | "transfer-encoding" | "te" | "trailer"
                | "proxy-connection" => continue,
                // x-forwarded-*
                _ if lower.starts_with("x-forwarded-") => continue,
                // Browser / CORS / Sec-*
                _ if lower.starts_with("sec-") || lower == "origin" || lower == "referer"
                    || lower == "priority" => continue,
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
                let latency_ms = start.elapsed().as_millis() as i64;
                let error_msg = format!("Connection error on channel '{}': {}", channel.name, e);
                last_error = error_msg.clone();

                // Record this failed attempt for the audit row. The single AuditTask
                // is dispatched at loop exit (or earlier on 4xx).
                routes.push(llm_gateway_storage::RouteAttempt {
                    model: upstream_name.to_string(),
                    channel_id: channel.channel_id.to_string(),
                    channel_name: Some(channel.name.clone()),
                    status_code: 0,
                    error_message: Some(error_msg),
                    latency_ms,
                    started_at: chrono::Utc::now(),
                });

                continue;
            }
        };
        let latency_ms = start.elapsed().as_millis() as i64;
        let status = resp.status().as_u16();

        tracing::debug!("[PROXY] Upstream response: channel={}, status={}, latency={}ms, content-encoding={:?}", channel.name, status, latency_ms, resp.headers().get("content-encoding"));

        if status >= 500 {
            last_error = format!("Server error {} on channel '{}'", status, channel.name);

            let error_body = resp.bytes().await.unwrap_or_default();
            let error_body_str = if error_body.len() >= 2 && error_body[0] == 0x1F && error_body[1] == 0x8B {
                use std::io::Read;
                let mut decoder = flate2::read::GzDecoder::new(&error_body[..]);
                let mut decompressed = String::new();
                match decoder.read_to_string(&mut decompressed) {
                    Ok(_) => decompressed,
                    Err(_) => String::from_utf8_lossy(&error_body).into_owned(),
                }
            } else {
                String::from_utf8_lossy(&error_body).into_owned()
            };
            let error_body_str = error_body_str.chars().map(|c| if c == '\0' || c == '\u{FFFD}' { ' ' } else { c }).collect::<String>();

            // Record this failed attempt for the audit row.
            routes.push(llm_gateway_storage::RouteAttempt {
                model: upstream_name.to_string(),
                channel_id: channel.channel_id.to_string(),
                channel_name: Some(channel.name.clone()),
                status_code: status as i32,
                error_message: Some(error_body_str),
                latency_ms,
                started_at: chrono::Utc::now(),
            });

            continue;
        }

        if status == 429 {
            tracing::warn!("[PROXY] Rate limited (429) on channel '{}', trying next", channel.name);
            last_error = format!("Rate limited (429) on channel '{}'", channel.name);

            let error_body = resp.bytes().await.unwrap_or_default();
            let error_body_str = String::from_utf8_lossy(&error_body).into_owned();

            // Circuit-break the (channel, model) combination on 429
            let recovery_instant = match parse_recovery_timestamp(&error_body_str) {
                Some(ts) => {
                    let dur = (ts - Utc::now()).max(chrono::Duration::seconds(5));
                    Instant::now() + dur.to_std().unwrap_or(Duration::from_secs(30))
                }
                None => Instant::now() + Duration::from_secs(30),
            };
            state.registry.disable_channel_model(
                &channel.channel_id.to_string(),
                &model_name,
                recovery_instant,
            );

            // Record this failed attempt for the audit row.
            routes.push(llm_gateway_storage::RouteAttempt {
                model: upstream_name.to_string(),
                channel_id: channel.channel_id.to_string(),
                channel_name: Some(channel.name.clone()),
                status_code: 429,
                error_message: Some(error_body_str),
                latency_ms,
                started_at: chrono::Utc::now(),
            });

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

            // Build request headers JSON (for audit log — only forwarded headers, exclude sensitive ones and x-forwarded-*)
            let request_headers_for_worker: String = {
                let mut map = serde_json::Map::new();
                for (name, value) in headers.iter() {
                    match name.as_str() {
                        "host" | "authorization" | "content-length" | "x-api-key" | "api-key" => continue,
                        _ if name.as_str().to_lowercase().starts_with("x-forwarded-") => continue,
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

            let pricing_policy = {
                let policy_id = channel_model.pricing_policy_id.as_deref()
                    .or(model_entry.model.pricing_policy_id.as_deref());
                match policy_id {
                    Some(id) => {
                        state.storage.get_pricing_policy(id).await
                            .map_err(|e| ApiError::Internal(e.to_string()))?
                    }
                    None => None,
                }
            };

            // Get raw bytes and try to decompress gzip if needed
            let error_body = resp.bytes().await.unwrap_or_default();
            let error_body_str = if error_body.len() >= 2 && error_body[0] == 0x1F && error_body[1] == 0x8B {
                // Gzip magic bytes - decompress
                use std::io::Read;
                let mut decoder = flate2::read::GzDecoder::new(&error_body[..]);
                let mut decompressed = String::new();
                match decoder.read_to_string(&mut decompressed) {
                    Ok(_) => decompressed,
                    Err(_) => String::from_utf8_lossy(&error_body).into_owned(),
                }
            } else {
                String::from_utf8_lossy(&error_body).into_owned()
            };
            // Clean null bytes and replacement characters
            let error_body_str = error_body_str.chars().map(|c| if c == '\0' || c == '\u{FFFD}' { ' ' } else { c }).collect::<String>();
            tracing::debug!("[PROXY] Upstream error response: status={}, body_len={}", status, error_body_str.len());

            // Record this 4xx attempt in the routes array.
            routes.push(llm_gateway_storage::RouteAttempt {
                model: upstream_name.to_string(),
                channel_id: channel.channel_id.to_string(),
                channel_name: Some(channel.name.clone()),
                status_code: status as i32,
                error_message: Some(error_body_str.clone()),
                latency_ms,
                started_at: chrono::Utc::now(),
            });

            // Dispatch a single AuditTask carrying the full failover history.
            // 4xx is terminal — return immediately, no further candidates attempted.
            let task = AuditTask {
                request_id: request_id.clone(),
                key_id: api_key.id.clone(),
                user_id: api_key.created_by.clone(),
                model_name: upstream_name.to_string(),
                provider_id: provider_id.clone(),
                protocol: proto,
                stream: is_stream,
                request_body: body.clone(),
                response_bytes: error_body_str.into_bytes(),
                status_code: status as i32,
                latency_ms: start_total.elapsed().as_millis() as i64,
                pricing_policy,
                markup_ratio: channel_model.markup_ratio,
                channel_id: Some(channel.channel_id.to_string()),
                original_model: Some(client_requested_model.clone()),
                upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
                model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
                request_path: Some(request_path.clone()),
                upstream_url: Some(upstream_url.clone()),
                request_headers: Some(request_headers_for_worker),
                response_headers: Some(response_headers_for_worker),
                routes,
            };
            dispatch_audit_task(&state, task).await;

            return Ok((StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY), error_body).into_response());
        }

        tracing::debug!("[PROXY] Upstream success response: status={}, is_stream={}", status, is_stream);

        // Build request headers JSON (for audit log — only forwarded headers, exclude sensitive ones and x-forwarded-*)
        let request_headers_for_worker: String = {
            let mut map = serde_json::Map::new();
            for (name, value) in headers.iter() {
                match name.as_str() {
                    "host" | "authorization" | "content-length" | "x-api-key" | "api-key" => continue,
                    _ if name.as_str().to_lowercase().starts_with("x-forwarded-") => continue,
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
            let pricing_policy = {
                let policy_id = channel_model.pricing_policy_id.as_deref()
                    .or(model_entry.model.pricing_policy_id.as_deref());
                match policy_id {
                    Some(id) => {
                        state.storage.get_pricing_policy(id).await
                            .map_err(|e| ApiError::Internal(e.to_string()))?
                    }
                    None => None,
                }
            };

            let audit_params = SseAuditParams {
                request_id: request_id.clone(),
                key_id: api_key.id.clone(),
                user_id: api_key.created_by.clone(),
                model_name: upstream_name.to_string(),
                provider_id: provider_id.clone(),
                protocol: proto,
                request_body: body.clone(),
                pricing_policy,
                markup_ratio: channel_model.markup_ratio,
                channel_id: channel.channel_id.to_string(),
                original_model: Some(client_requested_model.clone()),
                upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
                model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
                request_path: request_path.clone(),
                upstream_url: upstream_url.clone(),
                request_headers: request_headers_for_worker.clone(),
                response_headers: response_headers_for_worker,
                routes: routes.clone(),
                channel_name: Some(channel.name.clone()),
                start_total,
                attempt_start: start,
                modified_body: body.clone(),
            };

            let upstream_resp = resp;

            // mpsc channel for SSE events
            let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(256);

            tokio::spawn(process_sse_stream(
                upstream_resp,
                tx,
                state.clone(),
                audit_params,
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
        let pricing_policy = {
            let policy_id = channel_model.pricing_policy_id.as_deref()
                .or(model_entry.model.pricing_policy_id.as_deref());
            match policy_id {
                Some(id) => {
                    state.storage.get_pricing_policy(id).await
                        .map_err(|e| ApiError::Internal(e.to_string()))?
                }
                None => None,
            }
        };

        let proto = match protocol {
            ProxyProtocol::OpenAI => Protocol::Openai,
            ProxyProtocol::Anthropic => Protocol::Anthropic,
        };

        // Build the success RouteAttempt and append it to the routes array.
        // This produces the full failover history for the single AuditTask.
        routes.push(llm_gateway_storage::RouteAttempt {
            model: upstream_name.to_string(),
            channel_id: channel.channel_id.to_string(),
            channel_name: Some(channel.name.clone()),
            status_code: 200,
            error_message: None,
            latency_ms,
            started_at: chrono::Utc::now(),
        });

        let task = AuditTask {
            request_id: request_id.clone(),
            key_id: api_key.id.clone(),
            user_id: api_key.created_by.clone(),
            model_name: upstream_name.to_string(),
            provider_id: provider_id.clone(),
            protocol: proto,
            stream: false,
            request_body: body.clone(),
            response_bytes: response_bytes.clone(),
            status_code: 200,
            latency_ms: start_total.elapsed().as_millis() as i64,
            pricing_policy,
            markup_ratio: channel_model.markup_ratio,
            channel_id: Some(channel.channel_id.to_string()),
            original_model: Some(client_requested_model.clone()),
            upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
            model_override_reason: if upstream_name != &model_name { Some("channel_mapping".to_string()) } else { None },
            request_path: Some(request_path.clone()),
            upstream_url: Some(upstream_url.clone()),
            request_headers: Some(request_headers_for_worker.clone()),
            response_headers: Some(response_headers_for_worker),
            routes,
        };
        dispatch_audit_task(&state, task).await;

        return Ok(response_bytes.into_response());
    }

    // All channels failed — try model fallback
    if fallback_depth == 0 {
        if let Some(resp) = try_model_fallback(
            &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
        ).await {
            return Ok(resp);
        }
    }

    // All channels failed — dispatch a single audit row carrying the full
    // failover history (the `routes` Vec accumulated above). The top-level
    // fields reflect the last attempted channel.
    if let Some(last) = routes.last() {
        let proto = match protocol {
            ProxyProtocol::OpenAI => Protocol::Openai,
            ProxyProtocol::Anthropic => Protocol::Anthropic,
        };
        let last_channel_id = last.channel_id.clone();
        let last_model_name = last.model.clone();
        let last_error_msg = last.error_message.clone().unwrap_or_default();
        let last_status = last.status_code;

        // Resolve pricing policy from the last-attempted channel (best effort;
        // we don't have channel_model here, fall back to the model's policy).
        let pricing_policy = match model_entry.model.pricing_policy_id.as_deref() {
            Some(id) => state.storage.get_pricing_policy(id).await
                .map_err(|e| ApiError::Internal(e.to_string()))?,
            None => None,
        };

        let task = AuditTask {
            request_id: request_id.clone(),
            key_id: api_key.id.clone(),
            user_id: api_key.created_by.clone(),
            model_name: last_model_name,
            provider_id: provider_id.clone(),
            protocol: proto,
            stream: is_stream,
            request_body: body.clone(),
            response_bytes: last_error_msg.into_bytes(),
            status_code: last_status,
            latency_ms: start_total.elapsed().as_millis() as i64,
            pricing_policy,
            markup_ratio: 0,
            channel_id: Some(last_channel_id),
            original_model: Some(client_requested_model.clone()),
            upstream_model: None,
            model_override_reason: None,
            request_path: Some(request_path.clone()),
            upstream_url: None,
            request_headers: None,
            response_headers: None,
            routes,
        };
        dispatch_audit_task(&state, task).await;
    }

    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
}
/// Axum handler for proxy requests.
pub async fn proxy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
    protocol: ProxyProtocol,
    request_path: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, protocol, request_path, 0).await
}

/// Wrapper for /v1/chat/completions - uses OpenAI protocol
pub async fn proxy_with_protocol(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::OpenAI, "/v1/chat/completions".to_string(), 0).await
}

/// Wrapper for /v1/messages - uses Anthropic protocol
pub async fn messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::Anthropic, "/v1/messages".to_string(), 0).await
}

/// Wrapper for /v1/responses - uses OpenAI protocol, passthrough all fields
pub async fn responses(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::OpenAI, "/v1/responses".to_string(), 0).await
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
        fn disable_channel_model(&self, _channel_id: &str, _model_name: &str, _until: Instant) {}
        fn is_circuit_broken(&self, _channel_id: &str, _model_name: &str) -> bool {
            false
        }
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
            weight: None,
            model_overrides: HashMap::from([(
                model_key.clone(),
                ChannelModelEnriched {
                    upstream_model_name: Some("gpt-4o".into()),
                    pricing_policy_id: Some("policy-1".into()),
                    markup_ratio: 15_000,
                },
            )]),
            proxy_url: None,
            group_id: None,
            available_hours: None,
        };
        let enriched = rc.model_overrides.get(&model_key).expect("should have model override");
        assert_eq!(enriched.markup_ratio, 15_000);
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
            weight: None,
            model_overrides: HashMap::new(),
            proxy_url: None,
            group_id: None,
            available_hours: None,
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
            weight: None,
            model_overrides: HashMap::from([(
                "claude-3-5-sonnet".to_lowercase(),
                ChannelModelEnriched {
                    upstream_model_name: Some("claude-3-5-sonnet".into()),
                    pricing_policy_id: Some("policy-2".into()),
                    markup_ratio: 20_000,
                },
            )]),
            proxy_url: None,
            group_id: None,
            available_hours: None,
        };
        let registry = StubRegistry(vec![rc.clone()]);
        let result = registry.resolve(&channel_id.to_string()).await;
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert_eq!(resolved.provider_id, "prov-2");
        assert_eq!(resolved.adapter, ProxyProtocol::Anthropic);
        let enriched = resolved.model_overrides.get("claude-3-5-sonnet").expect("should have model override");
        assert_eq!(enriched.markup_ratio, 20_000);
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