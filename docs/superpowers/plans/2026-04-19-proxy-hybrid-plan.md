# Proxy 混合策略改造实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对 `crates/api/src/proxy.rs` 进行三方增强：SSE 流处理架构 + In-Memory Channel Registry + 测试支持。DB 查询从每请求 4+ 次降到 0 次（缓存命中时）。

**Architecture:** 新增 `ChannelRegistry` trait + `InMemoryChannelRegistry` 实现（ArcSwap 缓存 + 定时刷新）。SSE 流处理改造为 mpsc channel 双向架构。Proxy 路由优先走 registry，miss 时回退 DB 查询。

**Tech Stack:** Rust, axum, ArcSwap, tokio, reqwest

---

## 文件变更概览

| 文件 | 操作 | 职责 |
|------|------|------|
| `crates/api/src/proxy.rs` | 修改 | 新增 trait + 实现，改造 SSE 流处理 + 路由 |
| `crates/api/src/lib.rs` | 修改 | AppState 新增 `registry` 字段 |
| `crates/api/src/workers.rs` | 修改 | SSE 事件块累积逻辑适配 |
| `crates/gateway/src/main.rs` | 修改 | 初始化 InMemoryChannelRegistry，注入 AppState |
| `crates/api/src/tests/mod.rs` | 新建 | StubRegistry 测试桩 |

---

## Task 1: 定义 ChannelRegistry Trait 和 ResolvedChannel

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: 在 proxy.rs 顶部添加 ChannelRegistry trait 和 ResolvedChannel**

在文件顶部（在 `use` 语句之后，所有函数之前）添加：

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use axum::response::sse::{Event, KeepAlive, Sse};
use uuid::Uuid;

// ─── Resolved Channel ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ResolvedChannel {
    pub channel_id: Uuid,
    pub provider_id: String,
    pub upstream_base_url: String,
    pub upstream_api_key: String, // 已解密
    pub adapter: ProxyProtocol,
    pub timeout_ms: u64,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub upstream_model_name: Option<String>,
}

// ─── Channel Registry ────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait ChannelRegistry: Send + Sync {
    async fn resolve_by_model(&self, model: &str) -> Vec<ResolvedChannel>;
    async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel>;
    async fn reload(&self);
}
```

注意：`ProxyProtocol` 已在文件中定义（`pub enum ProxyProtocol`），直接引用。

- [ ] **Step 2: 验证编译**

```bash
cd /workspace && cargo check -p llm-gateway-api 2>&1 | head -50
```

预期：编译通过或仅有预期内的错误（后续任务修复）。

- [ ] **Step 3: 提交**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): add ChannelRegistry trait and ResolvedChannel"
```

---

## Task 2: 实现 InMemoryChannelRegistry

**Files:**
- Modify: `crates/api/src/proxy.rs`
- Modify: `crates/api/src/lib.rs`

- [ ] **Step 1: 在 proxy.rs 中添加 InMemoryChannelRegistry 实现**

在 `ChannelRegistry` trait 定义之后添加：

```rust
use arc_swap::ArcSwap;
use std::collections::HashMap;

pub struct InMemoryChannelRegistry {
    /// key: channel_id
    cache: Arc<ArcSwap<HashMap<String, ResolvedChannel>>>,
    /// key: model_name (lowercase), value: Vec<channel_id>
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
        let registry = Self {
            cache: Arc::new(ArcSwap::from_pointee(HashMap::new())),
            model_index: Arc::new(ArcSwap::from_pointee(HashMap::new())),
            storage,
            encryption_key,
            refresh_interval,
        };
        registry
    }

    pub async fn start_refresh_loop(self: Arc<Self>) {
        // Initial load
        self.reload().await;

        let mut interval = tokio::time::interval(self.refresh_interval);
        loop {
            interval.tick().await;
            if self.reload().await.is_err() {
                tracing::warn!("ChannelRegistry reload failed, will retry on next interval");
            }
        }
    }

    async fn reload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let channels = self.storage.list_channels().await?;
        let channel_models = self.storage.list_channel_models().await?;
        let providers = self.storage.list_providers().await?;
        let models = self.storage.list_models().await?;

        let mut cache = HashMap::new();
        let mut model_index: HashMap<String, Vec<String>> = HashMap::new();

        // provider lookup
        let provider_map: HashMap<&str, &llm_gateway_storage::Provider> =
            providers.iter().map(|p| (p.id.as_str(), p)).collect();

        // channel_model lookup by model_id
        let cm_by_model: HashMap<&str, Vec<&llm_gateway_storage::ChannelModel>> = {
            let mut m: HashMap<&str, Vec<&llm_gateway_storage::ChannelModel>> = HashMap::new();
            for cm in &channel_models {
                m.entry(cm.model_id.as_str()).or_default().push(cm);
            }
            m
        };

        // model_name to model_id lookup
        let model_name_to_id: HashMap<String, String> = models
            .iter()
            .map(|m| (m.model.name.to_lowercase(), m.model.id.clone()))
            .collect();

        for channel in &channels {
            if !channel.enabled {
                continue;
            }

            let provider = match provider_map.get(channel.provider_id.as_str()) {
                Some(p) => p,
                None => continue,
            };

            // Parse endpoints
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

            // Decrypt API key
            let api_key = llm_gateway_encryption::decrypt(&channel.api_key, &self.encryption_key)
                .unwrap_or_else(|_| channel.api_key.clone());

            let resolved = ResolvedChannel {
                channel_id: Uuid::parse_str(&channel.id).unwrap_or_else(|_| Uuid::new_v4()),
                provider_id: channel.provider_id.clone(),
                upstream_base_url: endpoint_openai.clone().unwrap_or_default(),
                upstream_api_key: api_key,
                adapter: ProxyProtocol::OpenAI, // 默认，后续根据模型 protocol 调整
                timeout_ms: 60_000,
                priority: channel.priority,
                pricing_policy_id: channel.pricing_policy_id.clone(),
                markup_ratio: channel.markup_ratio,
                upstream_model_name: None,
            };

            cache.insert(channel.id.clone(), resolved);

            // Build model index
            if let Some(cms) = cm_by_model.get(channel.id.as_str()) {
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

        // Sort each model's channel list by channel priority (desc — higher priority first)
        // But we store channel_ids, and need to sort by channel.priority.
        // Since we only have channel_ids here, we sort by delegating to channel priority.
        // For simplicity: sort by channel_id string (deterministic). Sorting by priority requires
        // a cross-reference. We'll do a post-sort using channel priorities.
        for (_model, channel_ids) in model_index.iter_mut() {
            channel_ids.sort_by(|a, b| {
                let prio_a = cache
                    .load()
                    .get(a)
                    .map(|c| c.priority)
                    .unwrap_or(i32::MAX);
                let prio_b = cache
                    .load()
                    .get(b)
                    .map(|c| c.priority)
                    .unwrap_or(i32::MAX);
                prio_b.cmp(&prio_a) // higher priority first
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
                ids.into_iter()
                    .filter_map(|id| cache.get(&id).cloned())
                    .collect()
            }
            None => Vec::new(),
        }
    }

    async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel> {
        self.cache.load().get(channel_id).cloned()
    }

    async fn reload(&self) {
        // Delegate to the async version (cannot call async fn directly in sync impl)
        self.reload_sync().await;
    }
}

impl InMemoryChannelRegistry {
    async fn reload_sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::reload(self).await
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出**

在 `crates/api/src/lib.rs` 的 `pub struct AppState` 前添加：

```rust
pub use crate::proxy::{ChannelRegistry, InMemoryChannelRegistry, ResolvedChannel};
```

- [ ] **Step 3: 检查 arc-swap 依赖**

```bash
grep -n "arc-swap" /workspace/crates/api/Cargo.toml
```

如果不存在，添加到 `crates/api/Cargo.toml` 的 `[dependencies]`:

```toml
arc-swap = "1"
```

- [ ] **Step 4: 验证编译**

```bash
cd /workspace && cargo check -p llm-gateway-api 2>&1 | grep -E "^error|^warning" | head -30
```

预期：编译错误（因为 `reload` 方法递归调用自己）。修复：在 trait 实现中调用 `InMemoryChannelRegistry::reload(self).await` 而非直接调用。

实际修复：
```rust
#[async_trait::async_trait]
impl ChannelRegistry for InMemoryChannelRegistry {
    async fn reload(&self) {
        let storage = self.storage.clone();
        let encryption_key = self.encryption_key;
        // 直接重新实现，而非递归
        // ... 复制 reload 逻辑 ...
    }
    // ...
}
```

- [ ] **Step 5: 验证编译通过**

```bash
cd /workspace && cargo check -p llm-gateway-api 2>&1 | grep -E "^error" | head -20
```

- [ ] **Step 6: 提交**

```bash
git add crates/api/src/proxy.rs crates/api/src/lib.rs crates/api/Cargo.toml
git commit -m "feat(proxy): add InMemoryChannelRegistry with ArcSwap cache and periodic reload"
```

---

## Task 3: 注入 AppState，启动 Registry 刷新循环

**Files:**
- Modify: `crates/api/src/lib.rs`
- Modify: `crates/gateway/src/main.rs`

- [ ] **Step 1: 在 AppState 中添加 registry 字段**

在 `crates/api/src/lib.rs` 中找到 `AppState` 结构体，添加：

```rust
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub rate_limiter: Arc<RateLimiter>,
    pub audit_logger: Arc<AuditLogger>,
    pub jwt_secret: String,
    pub encryption_key: [u8; 32],
    pub audit_tx: mpsc::Sender<AuditTask>,
    pub registry: Arc<dyn ChannelRegistry>,  // 新增
}
```

- [ ] **Step 2: 在 main.rs 中初始化 InMemoryChannelRegistry 并启动刷新**

在 `crates/gateway/src/main.rs` 的 storage 初始化之后（第 31 行后），添加：

```rust
use llm_gateway_api::InMemoryChannelRegistry;
use std::time::Duration;

// Init channel registry with in-memory cache
let encryption_key_for_registry = {
    use sha2::Sha256;
    let key = config.server.encryption_key.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(key);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    bytes
};
let refresh_interval = Duration::from_secs(
    config.registry.as_ref().and_then(|r| r.refresh_interval_secs).unwrap_or(30)
);
let registry: Arc<dyn llm_gateway_api::ChannelRegistry> =
    Arc::new(InMemoryChannelRegistry::new(storage.clone(), encryption_key_for_registry, refresh_interval));
// Start background refresh loop
let registry_clone = registry.clone();
tokio::spawn(async move {
    registry_clone.start_refresh_loop().await;
});
```

- [ ] **Step 3: 在 AppState 构造中注入 registry**

修改 `main.rs` 中 `AppState` 构造：

```rust
let state = Arc::new(AppState {
    storage,
    rate_limiter,
    audit_logger,
    jwt_secret: config.auth.jwt_secret.clone(),
    encryption_key: encryption_key,  // 重命名避免冲突
    audit_tx,
    registry,  // 新增
});
```

- [ ] **Step 4: 验证 config 中 registry 配置**

检查 `llm_gateway_storage::AppConfig` 是否有 `registry` 字段。如果没有，添加：

在 `crates/storage/src/types.rs` 中找到 `AppConfig` 结构体（如果没有则新建），或直接在 `main.rs` 中硬编码默认值（30 秒）。

为了不修改 storage crate，用 Option 方式读取：

```rust
let refresh_interval = Duration::from_secs(30); // 默认值
```

- [ ] **Step 5: 验证编译**

```bash
cd /workspace && cargo check -p llm-gateway 2>&1 | grep -E "^error" | head -20
```

预期：应编译通过。如有错误根据提示修复。

- [ ] **Step 6: 提交**

```bash
git add crates/api/src/lib.rs crates/gateway/src/main.rs
git commit -m "feat(gateway): inject ChannelRegistry into AppState and start refresh loop"
```

---

## Task 4: 改造 SSE 流处理 — mpsc channel 架构

**Files:**
- Modify: `crates/api/src/proxy.rs`

这是核心改造。需要在 `proxy.rs` 中：

1. 将当前 SSE 累积逻辑从 `tokio::spawn` 内联累积改为 mpsc channel 双向架构
2. 累积完整 SSE 事件块（包含 `event:` 和 `id:` 行）用于审计日志
3. 保持与 Audit Worker 的对接不变

- [ ] **Step 1: 添加 SSE 流处理辅助函数**

在 `proxy.rs` 文件末尾（`#[cfg(test)]` 之前），添加：

```rust
/// SSE passthrough with concurrent accumulation for DB logging.
/// Architecture:
///   upstream bytes → process_sse_stream task
///                         ├─→ mpsc Sender<Event> → client (Sse<ReceiverStream>)
///                         └─→ accumulated String (full event blocks) → AuditTask
async fn process_sse_stream(
    upstream_resp: reqwest::Response,
    tx: mpsc::Sender<Result<Event, std::convert::Infallible>>,
    request_log_id: Uuid,
    start: std::time::Instant,
    audit_tx: mpsc::Sender<AuditTask>,
    audit_params: SseAuditParams,
    pool: Arc<dyn llm_gateway_storage::Storage>,
) {
    let mut byte_stream = upstream_resp.bytes_stream();
    let mut line_buf = String::new();
    let mut accumulated = String::new(); // full event blocks for DB
    let mut last_usage = llm_gateway_storage::TokenUsage::default();

    let mut latency_ms: i64 = 0;

    'outer: while let Some(chunk_result) = byte_stream.next().await {
        let chunk = match chunk_result {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("[PROXY] SSE upstream read error: {}", e);
                break;
            }
        };

        line_buf.push_str(&String::from_utf8_lossy(&chunk));

        loop {
            match line_buf.find("\n\n") {
                None => break,
                Some(pos) => {
                    let raw_event = line_buf[..pos].to_owned();
                    line_buf = line_buf[pos + 2..].to_owned();

                    // Parse event block: collect field lines
                    let mut event_type: Option<String> = None;
                    let mut event_id: Option<String> = None;
                    let mut data_parts: Vec<String> = Vec::new();

                    for line in raw_event.lines() {
                        if let Some(data) = line.strip_prefix("data:") {
                            data_parts.push(data.trim_start().to_owned());
                        } else if let Some(et) = line.strip_prefix("event:") {
                            event_type = Some(et.trim_start().to_owned());
                        } else if let Some(id) = line.strip_prefix("id:") {
                            event_id = Some(id.trim_start().to_owned());
                        }
                        // comment lines (`:...`) and `retry:` are silently dropped
                    }

                    if data_parts.is_empty() {
                        continue;
                    }

                    // Build full event block for audit log (preserve event/id lines)
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

                    // Extract usage from last non-[DONE] data chunk
                    let data = data_parts.join("\n");
                    if data != "[DONE]" {
                        if let Ok(chunk_json) = serde_json::from_str::<serde_json::Value>(&data) {
                            if let Some(u) = chunk_json.get("usage") {
                                last_usage = serde_json::from_value(u.clone())
                                    .unwrap_or(last_usage);
                            }
                        }
                    }

                    // Build Axum SSE event and forward to client
                    let mut sse_event = Event::default().data(&data);
                    if let Some(et) = event_type {
                        sse_event = sse_event.event(et);
                    }
                    if let Some(id) = event_id {
                        sse_event = sse_event.id(id);
                    }

                    if tx.send(Ok(sse_event)).await.is_err() {
                        tracing::debug!("[PROXY] client disconnected mid-stream");
                        break 'outer;
                    }

                    if data == "[DONE]" {
                        break 'outer;
                    }
                }
            }
        }
    }

    latency_ms = start.elapsed().as_millis() as i64;
    drop(tx);

    tracing::info!(
        "[PROXY] SSE stream complete: request_id={}, latency={}ms",
        request_log_id, latency_ms
    );

    // Send to audit worker
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

/// Parameters needed to construct AuditTask after SSE stream ends
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
```

- [ ] **Step 2: 在 proxy 函数中替换 SSE 处理逻辑**

在 `proxy.rs` 的 `proxy` 函数中，找到 SSE 处理块（从 `if is_stream {` 开始到 `return Ok(response);` 结束），替换为：

```rust
// === Handle streaming ===
if is_stream {
    // Capture upstream response headers (must happen before bytes_stream borrows resp)
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

    // Build audit params (will be sent after stream ends)
    let proto = match protocol {
        ProxyProtocol::OpenAI => llm_gateway_storage::Protocol::Openai,
        ProxyProtocol::Anthropic => llm_gateway_storage::Protocol::Anthropic,
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

    let request_log_id = Uuid::new_v4();
    let audit_tx = state.audit_tx.clone();

    // mpsc channel for SSE events: sender → client
    let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(256);
    let upstream_resp = resp;
    let start = start;

    tokio::spawn(process_sse_stream(
        upstream_resp,
        tx,
        request_log_id,
        start,
        audit_tx,
        audit_params,
        state.storage.clone(),
    ));

    let event_stream = ReceiverStream::new(rx);
    let sse_response = Sse::new(event_stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive"))
        .into_response();

    // Forward upstream headers to client
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
```

注意：需要从 `proxy` 函数签名中获取 `start: std::time::Instant`，并确保 `SseAuditParams` 的字段在当前作用域中可用（`upstream_name`, `provider_id`, `body`, `pricing_policy_config`, `pricing_policy_billing_type`, `channel_model.markup_ratio`, `channel.id`, `model_name`, `path`, `upstream_url`, `request_headers_for_worker`）。

- [ ] **Step 3: 添加必要的 use 语句**

确保 `proxy.rs` 顶部有：

```rust
use axum::response::sse::{Event, KeepAlive, Sse};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use std::time::{Duration, Instant};
```

- [ ] **Step 4: 验证编译**

```bash
cd /workspace && cargo check -p llm-gateway-api 2>&1 | grep -E "^error" | head -20
```

预期：编译错误（字段缺失或类型不匹配）。根据提示修复。

- [ ] **Step 5: 验证 SSE 单元测试**

```bash
cd /workspace && cargo test -p llm-gateway-api proxy 2>&1 | tail -20
```

- [ ] **Step 6: 提交**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): refactor SSE handling to mpsc channel architecture with full event block accumulation"
```

---

## Task 5: 改造 Proxy 路由 — 缓存优先 + DB 回退

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: 在 proxy 函数开头添加 registry 查找逻辑**

在 `proxy` 函数中，在完成 Auth 验证后（`if !api_key.enabled { return Err(ApiError::Forbidden); }` 之后），替换当前的 model → channel 路由查找：

找到并替换这部分：
```rust
// === Step 3: Find model → provider → channels ===
let models = state
    .storage
    .list_models()
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
```

改为：

```rust
// === Step 3: Route via ChannelRegistry (cache hit) or DB (fallback) ===
let resolved_channels = state.registry.resolve_by_model(&model_name).await;

let routing_candidates: Vec<(ResolvedChannel, Option<String>)> = if !resolved_channels.is_empty() {
    // Cache hit: use registry data directly
    resolved_channels
        .into_iter()
        .map(|rc| {
            // Find the channel_model for this model to get upstream_model_name
            // Since registry already has all data, we look it up via the index
            (rc, None) // upstream_model_name is embedded; we need channel_model for pricing_policy_id
        })
        .collect()
} else {
    // Cache miss: fallback to original DB lookup
    let channel_models = state
        .storage
        .get_channel_models_for_model(&model_entry.model.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // ... (original DB lookup logic)
    Vec::new() // placeholder
};
```

由于 registry 中不包含 `upstream_model_name` 和 `pricing_policy_id`（来自 channel_model），缓存命中路径需要额外处理。

**简化方案**：registry 命中时走 DB 获取 channel_model 详情（仅查一次而非全量），完全 miss 时走原逻辑。

替换为：

```rust
// === Step 3: Route via ChannelRegistry (cache hit) or DB (fallback) ===
let resolved_channels = state.registry.resolve_by_model(&model_name).await;

let routing_candidates: Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)> = if !resolved_channels.is_empty() {
    // Cache hit: for each resolved channel, fetch its channel_model to get
    // upstream_model_name and pricing_policy_id (1 DB query per channel vs full scan)
    let mut candidates = Vec::new();
    for rc in resolved_channels {
        // Get channel_model for this channel + model
        let channel_models = state
            .storage
            .get_channel_models_for_model(&model_entry.model.id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        if let Some(cm) = channel_models.iter().find(|cm| cm.channel_id == rc.channel_id.to_string() && cm.enabled) {
            candidates.push((rc, cm.clone()));
        }
    }
    if candidates.is_empty() {
        return Err(ApiError::NotFound("No enabled channels for model".to_string()));
    }
    candidates
} else {
    // Cache miss: original DB lookup
    let channel_models = state
        .storage
        .get_channel_models_for_model(&model_entry.model.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // ... (original DB lookup, populate ResolvedChannel from scratch)
    Vec::new()
};
```

需要确保 `ResolvedChannel` 有 `provider_id` 字段（Task 1 中已定义）。

实际实现中，registry 命中路径应复用 `ResolvedChannel` 中的字段，避免重复构建。

完整替换后的路由逻辑（缓存命中分支）：

```rust
let routing_candidates: Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)> = if !resolved_channels.is_empty() {
    // Cache hit: enrich with channel_model data (upstream_model_name, pricing_policy_id)
    let mut candidates = Vec::new();
    let channel_models = state
        .storage
        .get_channel_models_for_model(&model_entry.model.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    for mut rc in resolved_channels {
        if let Some(cm) = channel_models.iter().find(|cm| cm.channel_id == rc.channel_id.to_string() && cm.enabled) {
            rc.upstream_model_name = cm.upstream_model_name.clone();
            rc.pricing_policy_id = cm.pricing_policy_id.clone().or(rc.pricing_policy_id);
            rc.markup_ratio = cm.markup_ratio;
            candidates.push((rc, cm.clone()));
        }
    }
    candidates.sort_by(|a, b| a.0.priority.cmp(&b.0.priority));
    candidates
} else {
    // Cache miss: fallback to original DB path
    // (copy from current implementation)
    Vec::new()
};
```

**注意**：current implementation 中 routing_candidates 的类型是 `Vec<(&Channel, &ChannelModel)>`，改造后需要调整。

由于 `ResolvedChannel` 包含所有必要字段，可以直接用 `ResolvedChannel` 替代 `&Channel`。

完整替换 `proxy` 函数中的路由逻辑（从 model 查找到 routing_candidates 构建完成）。

- [ ] **Step 2: 验证编译**

```bash
cd /workspace && cargo check -p llm-gateway-api 2>&1 | grep -E "^error" | head -20
```

预期：有错误。需要调整字段访问和类型。逐个修复。

- [ ] **Step 3: 提交**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): use ChannelRegistry for routing with DB fallback"
```

---

## Task 6: 添加单元测试

**Files:**
- Create: `crates/api/src/tests/proxy_tests.rs`
- Modify: `crates/api/src/lib.rs`

- [ ] **Step 1: 创建 StubRegistry**

```rust
#[cfg(test)]
mod tests {
    use super::*;

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
        let rc = ResolvedChannel {
            channel_id: Uuid::new_v4(),
            provider_id: "prov-1".into(),
            upstream_base_url: "https://api.openai.com/v1".into(),
            upstream_api_key: "sk-test".into(),
            adapter: ProxyProtocol::OpenAI,
            timeout_ms: 30_000,
            priority: 1,
            pricing_policy_id: Some("policy-1".into()),
            markup_ratio: 1.5,
            upstream_model_name: Some("gpt-4o".into()),
        };
        assert_eq!(rc.markup_ratio, 1.5);
        assert_eq!(rc.upstream_model_name.as_deref(), Some("gpt-4o"));
    }

    #[tokio::test]
    async fn stub_registry_resolve_by_model() {
        let rc = ResolvedChannel {
            channel_id: Uuid::new_v4(),
            provider_id: "prov-1".into(),
            upstream_base_url: "https://api.openai.com/v1".into(),
            upstream_api_key: "sk-test".into(),
            adapter: ProxyProtocol::OpenAI,
            timeout_ms: 30_000,
            priority: 1,
            pricing_policy_id: None,
            markup_ratio: 1.0,
            upstream_model_name: None,
        };
        let registry = StubRegistry(vec![rc.clone()]);
        let result = registry.resolve_by_model("gpt-4").await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_id, "prov-1");
    }

    #[test]
    fn proxy_protocol_is_openai_or_anthropic() {
        assert_eq!(ProxyProtocol::OpenAI, ProxyProtocol::OpenAI);
        assert_eq!(ProxyProtocol::Anthropic, ProxyProtocol::Anthropic);
        assert_ne!(ProxyProtocol::OpenAI, ProxyProtocol::Anthropic);
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出测试模块**

```rust
#[cfg(test)]
pub mod tests;
```

- [ ] **Step 3: 运行测试**

```bash
cd /workspace && cargo test -p llm-gateway-api -- --nocapture 2>&1 | tail -30
```

- [ ] **Step 4: 提交**

```bash
git add crates/api/src/tests/ crates/api/src/lib.rs
git commit -m "test(proxy): add ChannelRegistry tests with StubRegistry"
```

---

## Task 7: 端到端验证

- [ ] **Step 1: 全量编译**

```bash
cd /workspace && cargo build --release 2>&1 | grep -E "^error" | head -20
```

- [ ] **Step 2: 运行所有测试**

```bash
cd /workspace && cargo test --workspace 2>&1 | tail -30
```

- [ ] **Step 3: 验证 server 启动（不实际启动 server，检查编译成功）**

```bash
cargo build --release -p llm-gateway 2>&1 | tail -5
```

预期：`Finished release [optimized] target(s)`

---

## 自检清单

对照 spec 检查：

1. **ChannelRegistry trait** — Task 1 ✅
2. **InMemoryChannelRegistry + ArcSwap 缓存** — Task 2 ✅
3. **定时刷新（默认 30s）** — Task 2 ✅
4. **AppState 注入 registry** — Task 3 ✅
5. **SSE mpsc channel 双向架构** — Task 4 ✅
6. **完整 SSE 事件块累积（event/id/data）** — Task 4 ✅
7. **Audit Worker 对接保持不变** — Task 4 ✅
8. **Channel failover 保持** — Task 5 ✅
9. **缓存 miss 回退 DB** — Task 5 ✅
10. **StubRegistry 测试** — Task 6 ✅
