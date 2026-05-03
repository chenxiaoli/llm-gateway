# NATS JetStream Audit/Usage Decoupling — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the in-process `mpsc::channel(100)` for audit/usage with NATS JetStream, providing durable event persistence and enabling external consumers.

**Architecture:** Gateway publishes pre-computed `UsageEvent` and `AuditEvent` to two separate NATS JetStream streams. In-process push consumers write to DB. When `[nats]` config is absent, falls back to current mpsc behavior.

**Tech Stack:** Rust, async-nats (JetStream), tokio, serde_json

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/nats-publisher/Cargo.toml` | Create | Crate manifest |
| `crates/nats-publisher/src/lib.rs` | Create | Event types + NATS client (connect, create streams, publish) |
| `Cargo.toml` | Modify | Add `nats-publisher` member |
| `crates/storage/src/types.rs` | Modify | Add `NatsConfig` to `AppConfig` |
| `crates/api/src/lib.rs` | Modify | Add `nats_publisher: Option<Arc<NatsPublisher>>` to AppState |
| `crates/api/src/workers.rs` | Modify | Make `parse_usage` + cost calc public; add NATS consumer workers |
| `crates/api/src/proxy.rs` | Modify | NATS publish path alongside mpsc fallback |
| `crates/gateway/src/main.rs` | Modify | Init NATS client, start consumers |
| `crates/api/Cargo.toml` | Modify | Add `llm-gateway-nats-publisher` dependency |

---

### Task 1: Create `nats-publisher` crate with event types

**Files:**
- Create: `crates/nats-publisher/Cargo.toml`
- Create: `crates/nats-publisher/src/lib.rs`

- [ ] **Step 1: Create crate manifest**

Create `crates/nats-publisher/Cargo.toml`:

```toml
[package]
name = "llm-gateway-nats-publisher"
version = "0.14.1"
edition = "2021"

[dependencies]
async-nats = "0.39"
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }
```

- [ ] **Step 2: Create event types and NATS client**

Create `crates/nats-publisher/src/lib.rs`:

```rust
use async_nats::jetstream::{self, consumer::PushConsumer, stream::Config, AckKind, Context};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const STREAM_USAGE: &str = "GATEWAY_USAGE";
const STREAM_AUDIT: &str = "GATEWAY_AUDIT";
const SUBJECT_USAGE: &str = "gateway.usage";
const SUBJECT_AUDIT: &str = "gateway.audit";
const CONSUMER_USAGE: &str = "usage-worker";
const CONSUMER_AUDIT: &str = "audit-worker";
const DELIVERY_USAGE: &str = "usage-worker-delivery";
const DELIVERY_AUDIT: &str = "audit-worker-delivery";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: String,
    pub key_id: String,
    pub user_id: Option<String>,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cost: i64,
    pub latency_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub key_id: String,
    pub user_id: Option<String>,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: String,
    pub stream: bool,
    pub status_code: i32,
    pub latency_ms: i64,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: Option<String>,
    pub upstream_url: Option<String>,
    pub request_body: String,
    pub response_body: String,
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
    pub created_at: String,
}

pub struct NatsPublisher {
    js: Context,
}

impl NatsPublisher {
    pub async fn new(url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = async_nats::connect(url).await?;
        let js = jetstream::new(client);

        // Create streams idempotently
        js.get_or_create_stream(Config {
            name: STREAM_USAGE.into(),
            subjects: vec![SUBJECT_USAGE.into()],
            max_messages: 1_000_000,
            max_age: std::time::Duration::from_secs(7 * 24 * 3600),
            ..Default::default()
        })
        .await?;

        js.get_or_create_stream(Config {
            name: STREAM_AUDIT.into(),
            subjects: vec![SUBJECT_AUDIT.into()],
            max_messages: 5_000_000,
            max_age: std::time::Duration::from_secs(30 * 24 * 3600),
            ..Default::default()
        })
        .await?;

        Ok(Self { js })
    }

    pub async fn publish_usage(&self, event: &UsageEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::to_vec(event)?;
        self.js.publish(SUBJECT_USAGE.into(), payload.into()).await?.await?;
        Ok(())
    }

    pub async fn publish_audit(&self, event: &AuditEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::to_vec(event)?;
        self.js.publish(SUBJECT_AUDIT.into(), payload.into()).await?.await?;
        Ok(())
    }

    pub async fn create_usage_consumer(&self) -> Result<PushConsumer, Box<dyn std::error::Error + Send + Sync>> {
        let stream = self.js.get_stream(STREAM_USAGE).await?;
        let consumer = stream.get_or_create_consumer(
            CONSUMER_USAGE,
            async_nats::jetstream::consumer::push::Config {
                durable_name: Some(CONSUMER_USAGE.into()),
                deliver_subject: DELIVERY_USAGE.into(),
                ..Default::default()
            },
        )
        .await?;
        Ok(consumer)
    }

    pub async fn create_audit_consumer(&self) -> Result<PushConsumer, Box<dyn std::error::Error + Send + Sync>> {
        let stream = self.js.get_stream(STREAM_AUDIT).await?;
        let consumer = stream.get_or_create_consumer(
            CONSUMER_AUDIT,
            async_nats::jetstream::consumer::push::Config {
                durable_name: Some(CONSUMER_AUDIT.into()),
                deliver_subject: DELIVERY_AUDIT.into(),
                ..Default::default()
            },
        )
        .await?;
        Ok(consumer)
    }

    pub fn js_context(&self) -> &Context {
        &self.js
    }
}
```

- [ ] **Step 3: Add crate to workspace**

In `Cargo.toml` (workspace root), add `"crates/nats-publisher"` to the `members` array after `"crates/encryption"`:

```toml
members = [
    "crates/gateway",
    "crates/api",
    "crates/provider",
    "crates/auth",
    "crates/ratelimit",
    "crates/billing",
    "crates/audit",
    "crates/storage",
    "crates/encryption",
    "crates/nats-publisher",
]
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p llm-gateway-nats-publisher`
Expected: compiles without errors

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nats-publisher/
git commit -m "feat: add nats-publisher crate with event types and JetStream client"
```

---

### Task 2: Add NATS config and wire into AppState

**Files:**
- Modify: `crates/storage/src/types.rs` (add NatsConfig)
- Modify: `crates/storage/src/lib.rs` (add to AppConfig if needed, or check where AppConfig lives)
- Modify: `crates/api/src/lib.rs` (add nats_publisher to AppState)
- Modify: `crates/api/Cargo.toml` (add nats-publisher dep)

- [ ] **Step 1: Find and update AppConfig**

Read `crates/storage/src/lib.rs` to find where `AppConfig` is defined. It likely has `DatabaseConfig`, `AuthConfig`, etc. Add a new `NatsConfig`:

In `crates/storage/src/lib.rs`, add after the existing config structs:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    pub url: String,
}
```

Add `pub nats: Option<NatsConfig>` to the main `AppConfig` struct.

- [ ] **Step 2: Add nats_publisher to AppState**

In `crates/api/src/lib.rs`, add import at the top:

```rust
use llm_gateway_nats_publisher::NatsPublisher;
```

Add to `AppState` struct (after `audit_tx`):

```rust
pub nats_publisher: Option<Arc<NatsPublisher>>,
```

- [ ] **Step 3: Add dependency to api crate**

In `crates/api/Cargo.toml`, add to `[dependencies]`:

```toml
llm-gateway-nats-publisher = { path = "../nats-publisher" }
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --workspace`
Expected: compiles (NatsPublisher not used yet, just Option field)

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/lib.rs crates/api/src/lib.rs crates/api/Cargo.toml
git commit -m "feat: add NatsConfig and wire NatsPublisher into AppState"
```

---

### Task 3: Make parse_usage and cost calculation callable from proxy.rs

**Files:**
- Modify: `crates/api/src/workers.rs` (make functions public)

The `parse_usage` function and cost calculation logic currently live inside the private worker loop. We need `parse_usage` public so proxy.rs can call it in NATS mode. The cost calculation should also be extracted into a standalone function.

- [ ] **Step 1: Make parse_usage public**

In `crates/api/src/workers.rs`, change line 13 from:

```rust
fn parse_usage(bytes: &[u8], stream: bool, proto: Protocol) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>)
```

to:

```rust
pub fn parse_usage(bytes: &[u8], stream: bool, proto: Protocol) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>)
```

- [ ] **Step 2: Extract cost calculation into a public function**

Add this public function before `start_audit_worker` in `crates/api/src/workers.rs`:

```rust
pub fn calculate_cost(
    pricing_policy_config: &Option<serde_json::Value>,
    pricing_policy_billing_type: &str,
    markup_ratio: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
) -> i64 {
    use llm_gateway_billing::PricingCalculator;
    use llm_gateway_storage::{PricingPolicy, Usage};

    if let Some(config) = pricing_policy_config {
        let policy = PricingPolicy {
            id: String::new(),
            name: String::new(),
            billing_type: pricing_policy_billing_type.to_string(),
            config: config.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let usage = Usage {
            input_tokens: input_tokens.unwrap_or(0),
            output_tokens: output_tokens.unwrap_or(0),
            input_chars: None,
            output_chars: None,
            request_count: 1,
            cache_read_tokens,
            cache_creation_tokens,
        };
        let raw_cost = PricingCalculator.calculate_cost(&policy, &usage);
        raw_cost * markup_ratio / 10_000
    } else {
        0
    }
}
```

- [ ] **Step 3: Update start_audit_worker to use the extracted function**

In `start_audit_worker`, replace the inline cost calculation block (the `let cost = ...` block around lines 101-123) with:

```rust
let cost = calculate_cost(
    &task.pricing_policy_config,
    &task.pricing_policy_billing_type,
    task.markup_ratio,
    input_tokens,
    output_tokens,
    cache_read_tokens,
    cache_creation_tokens,
);
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p llm-gateway-api`
Expected: compiles without errors

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/workers.rs
git commit -m "refactor: extract parse_usage and calculate_cost as public functions"
```

---

### Task 4: Add NATS publish path to proxy.rs

**Files:**
- Modify: `crates/api/src/proxy.rs`

This task adds a helper function that publishes to NATS when available, and replaces the three `audit_tx` send points with dual-path logic.

- [ ] **Step 1: Add publish helper function**

Add this function near the top of `crates/api/src/proxy.rs` (after the imports, around line 21):

```rust
use llm_gateway_nats_publisher::{AuditEvent, UsageEvent};

async fn publish_audit_events(
    state: &Arc<crate::AppState>,
    task: &crate::AuditTask,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
    cost: i64,
) {
    if let Some(nats) = &state.nats_publisher {
        let now = chrono::Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        let usage_event = UsageEvent {
            id: id.clone(),
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
            latency_ms: task.latency_ms,
            created_at: now.to_rfc3339(),
        };

        let audit_event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
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
            response_body: String::from_utf8_lossy(&task.response_bytes).into_owned(),
            request_headers: task.request_headers.clone(),
            response_headers: task.response_headers.clone(),
            created_at: now.to_rfc3339(),
        };

        if let Err(e) = nats.publish_usage(&usage_event).await {
            tracing::warn!("[NATS] Failed to publish usage event: {}", e);
        }
        if let Err(e) = nats.publish_audit(&audit_event).await {
            tracing::warn!("[NATS] Failed to publish audit event: {}", e);
        }
    }
}
```

- [ ] **Step 2: Create a dispatch helper that handles both paths**

Add this helper function after `publish_audit_events`:

```rust
async fn dispatch_audit_task(
    state: &Arc<crate::AppState>,
    task: crate::AuditTask,
) {
    if state.nats_publisher.is_some() {
        let proto = task.protocol;
        let stream = task.stream;
        let (input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens) =
            crate::workers::parse_usage(&task.response_bytes, stream, proto);
        let cost = crate::workers::calculate_cost(
            &task.pricing_policy_config,
            &task.pricing_policy_billing_type,
            task.markup_ratio,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
        );
        publish_audit_events(state, &task, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost).await;
    } else {
        let _ = state.audit_tx.try_send(task);
    }
}
```

- [ ] **Step 3: Replace the three audit_tx send points**

**Point 1: Non-streaming error (line ~953)**
Replace:
```rust
let _ = state.audit_tx.try_send(task);
```
With:
```rust
dispatch_audit_task(&state, task).await;
```

**Point 2: Non-streaming success (line ~1124)**
Replace:
```rust
let _ = state.audit_tx.try_send(task);
```
With:
```rust
dispatch_audit_task(&state, task).await;
```

**Point 3: SSE stream completion in `process_sse_stream` (line ~473)**

This one is inside a spawned task that takes `mpsc::Sender<AuditTask>`. Change the function signature to take `Arc<crate::AppState>` instead of `mpsc::Sender<AuditTask>`, and replace the `audit_tx.send(task).await` with:

```rust
dispatch_audit_task(&state, task).await;
```

Update the `process_sse_stream` function signature to accept `state: Arc<crate::AppState>` instead of `audit_tx: mpsc::Sender<AuditTask>`, and remove the `use crate::AuditTask;` import from the function body if needed. Pass `state.clone()` at the call site (line ~1041) instead of `audit_tx.clone()`.

Also remove the `let audit_tx = state.audit_tx.clone();` line (~1032) since `process_sse_stream` no longer needs it.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p llm-gateway-api`
Expected: compiles without errors

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat: add NATS publish path to proxy with mpsc fallback"
```

---

### Task 5: Add NATS consumer workers

**Files:**
- Modify: `crates/api/src/workers.rs`

Add two new worker functions that consume from NATS JetStream and write to DB.

- [ ] **Step 1: Add NATS usage consumer**

Add to `crates/api/src/workers.rs`:

```rust
pub async fn start_nats_usage_worker(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
) {
    use llm_gateway_storage::UsageRecord;

    tracing::info!("[NATS-USAGE-WORKER] Starting");

    let consumer = match nats.create_usage_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[NATS-USAGE-WORKER] Failed to create consumer: {}", e);
            return;
        }
    };

    let mut messages = consumer.messages().await.unwrap_or_else(|e| {
        tracing::error!("[NATS-USAGE-WORKER] Failed to subscribe: {}", e);
        panic!("NATS usage consumer subscribe failed");
    });

    while let Some(Ok(msg)) = messages.next().await {
        let event: llm_gateway_nats_publisher::UsageEvent = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("[NATS-USAGE-WORKER] Failed to deserialize: {}", e);
                let _ = msg.ack().await;
                continue;
            }
        };

        let record = UsageRecord {
            id: event.id,
            key_id: event.key_id,
            user_id: event.user_id,
            model_name: event.model_name,
            provider_id: event.provider_id,
            channel_id: event.channel_id,
            protocol: match event.protocol.as_str() {
                "anthropic" => llm_gateway_storage::Protocol::Anthropic,
                _ => llm_gateway_storage::Protocol::Openai,
            },
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
            cache_read_tokens: event.cache_read_tokens,
            cache_creation_tokens: event.cache_creation_tokens,
            cost: event.cost,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.created_at)
                .map(|dt| dt.to_utc())
                .unwrap_or_else(|_| chrono::Utc::now()),
        };

        if let Err(e) = storage.record_usage(&record).await {
            tracing::warn!("[NATS-USAGE-WORKER] Failed to record usage: {}", e);
            // Nack to retry
            let _ = msg.nack().await;
            continue;
        }

        let _ = msg.ack().await;
    }

    tracing::info!("[NATS-USAGE-WORKER] Exiting");
}
```

- [ ] **Step 2: Add NATS audit consumer**

Add to `crates/api/src/workers.rs`:

```rust
pub async fn start_nats_audit_worker(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
) {
    use futures::StreamExt;
    use llm_gateway_audit::AuditLogger;
    use llm_gateway_storage::Protocol;

    tracing::info!("[NATS-AUDIT-WORKER] Starting");

    let audit_logger = AuditLogger::new(storage);
    let consumer = match nats.create_audit_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[NATS-AUDIT-WORKER] Failed to create consumer: {}", e);
            return;
        }
    };

    let mut messages = consumer.messages().await.unwrap_or_else(|e| {
        tracing::error!("[NATS-AUDIT-WORKER] Failed to subscribe: {}", e);
        panic!("NATS audit consumer subscribe failed");
    });

    while let Some(Ok(msg)) = messages.next().await {
        let event: llm_gateway_nats_publisher::AuditEvent = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("[NATS-AUDIT-WORKER] Failed to deserialize: {}", e);
                let _ = msg.ack().await;
                continue;
            }
        };

        let proto = match event.protocol.as_str() {
            "anthropic" => Protocol::Anthropic,
            _ => Protocol::Openai,
        };

        if let Err(e) = audit_logger.log_request(
            &event.key_id,
            event.user_id.as_deref(),
            &event.model_name,
            &event.provider_id,
            event.channel_id.as_deref(),
            proto,
            event.stream,
            &event.request_body,
            &event.response_body,
            event.status_code,
            event.latency_ms,
            None, // input_tokens not in audit event
            None, // output_tokens not in audit event
            event.original_model.as_deref(),
            event.upstream_model.as_deref(),
            event.model_override_reason.as_deref(),
            event.request_path.as_deref(),
            event.upstream_url.as_deref(),
            event.request_headers.as_deref(),
            event.response_headers.as_deref(),
        ).await {
            tracing::warn!("[NATS-AUDIT-WORKER] Failed to log audit: {}", e);
            let _ = msg.nack().await;
            continue;
        }

        let _ = msg.ack().await;
    }

    tracing::info!("[NATS-AUDIT-WORKER] Exiting");
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p llm-gateway-api`
Expected: compiles without errors

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/workers.rs
git commit -m "feat: add NATS consumer workers for usage and audit"
```

---

### Task 6: Wire up main.rs — init NATS, start consumers

**Files:**
- Modify: `crates/gateway/src/main.rs`
- Modify: `crates/gateway/Cargo.toml` (add nats-publisher dep)

- [ ] **Step 1: Add dependency to gateway crate**

In `crates/gateway/Cargo.toml`, add to `[dependencies]`:

```toml
llm-gateway-nats-publisher = { path = "../nats-publisher" }
```

- [ ] **Step 2: Update main.rs to init NATS and start workers**

In `crates/gateway/src/main.rs`, after the storage init block (after line ~53), add NATS initialization:

```rust
// Init NATS publisher (optional)
let nats_publisher: Option<Arc<llm_gateway_nats_publisher::NatsPublisher>> =
    if let Some(nats_cfg) = &config.nats {
        match llm_gateway_nats_publisher::NatsPublisher::new(&nats_cfg.url).await {
            Ok(pub_) => {
                tracing::info!("Connected to NATS: {}", nats_cfg.url);
                Some(Arc::new(pub_))
            }
            Err(e) => {
                tracing::error!("Failed to connect to NATS: {}", e);
                return Err(format!("NATS connection failed: {}", e).into());
            }
        }
    } else {
        tracing::info!("NATS not configured, using in-process mpsc for audit");
        None
    };
```

Update the `AppState` construction (around line 105-115) to include:

```rust
nats_publisher,
```

Update the worker spawn section. Replace the existing unconditional worker spawn (line ~85):

```rust
let (audit_tx, audit_rx) = tokio::sync::mpsc::channel::<llm_gateway_api::AuditTask>(100);
```

With conditional logic:

```rust
let (audit_tx, audit_rx) = tokio::sync::mpsc::channel::<llm_gateway_api::AuditTask>(100);

if let Some(nats) = &nats_publisher {
    let nats_usage = nats.clone();
    let nats_audit = nats.clone();
    let storage_usage = storage.clone();
    let storage_audit = storage.clone();
    tokio::spawn(async move {
        llm_gateway_api::workers::start_nats_usage_worker(storage_usage, nats_usage).await;
    });
    tokio::spawn(async move {
        llm_gateway_api::workers::start_nats_audit_worker(storage_audit, nats_audit).await;
    });
} else {
    let storage_clone = storage.clone();
    tokio::spawn(async move {
        llm_gateway_api::workers::start_audit_worker(storage_clone, audit_rx).await;
    });
}
```

Note: The `audit_tx` channel is still created even when NATS is active, because the fallback path in `dispatch_audit_task` (in proxy.rs) uses `state.audit_tx.try_send(task)` when `nats_publisher` is None. When NATS is active, the mpsc receiver is simply dropped (no consumer), which means try_send will fail gracefully — but this path is never taken when NATS is configured.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --workspace`
Expected: compiles without errors

- [ ] **Step 4: Commit**

```bash
git add crates/gateway/Cargo.toml crates/gateway/src/main.rs
git commit -m "feat: wire NATS publisher and consumers into gateway startup"
```

---

### Task 7: Update tests

**Files:**
- Modify: All test files in `crates/api/tests/` that construct AppState

The test files create `AppState` with `mpsc::channel(100)`. They need the new `nats_publisher: None` field.

- [ ] **Step 1: Find all test files that construct AppState**

Run: `grep -rn "audit_tx" crates/api/tests/`

For each file found, add `nats_publisher: None,` after the `audit_tx` field in the AppState construction.

Files to update:
- `crates/api/tests/test_management_providers.rs`
- `crates/api/tests/test_management_keys.rs`
- `crates/api/tests/test_users.rs`
- `crates/api/tests/test_settings.rs`
- `crates/api/tests/test_auth.rs`

- [ ] **Step 2: Verify tests compile**

Run: `cargo test --workspace --no-run`
Expected: compiles without errors

- [ ] **Step 3: Run tests**

Run: `cargo test --workspace`
Expected: all existing tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/api/tests/
git commit -m "fix: update test AppState with nats_publisher field"
```

---

### Task 8: Build verification

**Files:** None (verification only)

- [ ] **Step 1: Full workspace build**

Run: `cargo build --workspace`
Expected: compiles without errors

- [ ] **Step 2: Frontend build (should be unaffected)**

Run: `cd web && npm run build`
Expected: builds without errors (no frontend changes)

- [ ] **Step 3: Verify mpsc fallback still works**

1. Start backend without `[nats]` config: `cargo run`
2. Expected log: `NATS not configured, using in-process mpsc for audit`
3. Make a request and verify usage/audit records are written to DB

- [ ] **Step 4: Verify NATS mode**

1. Add `[nats]` section to `config.toml`:
   ```toml
   [nats]
   url = "nats://localhost:4222"
   ```
2. Start a NATS server: `docker run -p 4222:4222 nats:latest -js`
3. Start backend: `cargo run`
4. Expected log: `Connected to NATS: nats://localhost:4222`
5. Make a request and verify usage/audit records are written to DB
6. Verify events are in JetStream via `nats stream info GATEWAY_USAGE`
