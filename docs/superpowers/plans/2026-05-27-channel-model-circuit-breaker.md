# Channel-Model Circuit Breaker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace channel-level DB-based disabling with per-(channel, model) in-memory circuit breaker for immediate failover on 429/SSE errors.

**Architecture:** Add a `DashMap<(String, String), Instant>` to `InMemoryChannelRegistry`. Circuit breaker state is purely in-memory — no DB writes, no migrations. Filtering happens inside `resolve_by_model()` so callers are unaware of circuit breaker internals.

**Tech Stack:** Rust, DashMap 6, existing ArcSwap/reqwest infrastructure.

---

### Task 1: Add `dashmap` dependency

**Files:**
- Modify: `crates/api/Cargo.toml`

- [ ] **Step 1: Add dashmap to Cargo.toml**

In `crates/api/Cargo.toml`, add `dashmap = "6"` to the `[dependencies]` section (after the `rand = "0.9"` line):

```toml
dashmap = "6"
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /workspace/llm-gateway && cargo check -p llm-gateway-api`
Expected: compiles (dashmap downloaded, no code uses it yet)

- [ ] **Step 3: Commit**

```bash
git add crates/api/Cargo.toml
git commit -m "chore: add dashmap dependency for circuit breaker"
```

---

### Task 2: Add `circuit_breaker` field to `InMemoryChannelRegistry` and implement new trait methods

**Files:**
- Modify: `crates/api/src/proxy.rs` (lines 1–8 imports, lines 82–87 trait, lines 96–106 struct, lines 108–121 constructor, lines 330–354 trait impl)

- [ ] **Step 1: Add `dashmap` import**

At the top of `crates/api/src/proxy.rs`, after the existing `use arc_swap::ArcSwap;` line (line 98), add:

```rust
use dashmap::DashMap;
```

- [ ] **Step 2: Add new methods to `ChannelRegistry` trait**

In the trait definition (lines 82–87), add two new methods after `async fn reload(&self);`:

```rust
fn disable_channel_model(&self, channel_id: &str, model_name: &str, until: Instant);
fn is_circuit_broken(&self, channel_id: &str, model_name: &str) -> bool;
```

The full trait becomes:

```rust
#[async_trait]
pub trait ChannelRegistry: Send + Sync {
    async fn resolve_by_model(&self, model: &str) -> Vec<ResolvedChannel>;
    async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel>;
    async fn reload(&self);
    fn disable_channel_model(&self, channel_id: &str, model_name: &str, until: Instant);
    fn is_circuit_broken(&self, channel_id: &str, model_name: &str) -> bool;
}
```

- [ ] **Step 3: Add `circuit_breaker` field to `InMemoryChannelRegistry`**

In the struct definition (lines 100–106), add a new field after `refresh_interval`:

```rust
pub struct InMemoryChannelRegistry {
    cache: Arc<ArcSwap<HashMap<String, ResolvedChannel>>>,
    model_index: Arc<ArcSwap<HashMap<String, Vec<String>>>>,
    storage: Arc<dyn llm_gateway_storage::Storage>,
    encryption_key: [u8; 32],
    refresh_interval: Duration,
    circuit_breaker: Arc<DashMap<(String, String), Instant>>,
}
```

- [ ] **Step 4: Initialize `circuit_breaker` in constructor**

In the `new()` method (lines 108–121), add the field to the `Self` struct:

```rust
Self {
    cache: Arc::new(ArcSwap::from_pointee(HashMap::new())),
    model_index: Arc::new(ArcSwap::from_pointee(HashMap::new())),
    storage,
    encryption_key,
    refresh_interval,
    circuit_breaker: Arc::new(DashMap::new()),
}
```

- [ ] **Step 5: Implement the new trait methods**

In the `impl ChannelRegistry for InMemoryChannelRegistry` block (starting at line 330), add two new method implementations after the existing `reload()` method:

```rust
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
```

- [ ] **Step 6: Update `resolve_by_model` to filter circuit-broken entries**

Replace the existing `resolve_by_model` implementation (lines 332–345) with circuit-breaker-aware version:

```rust
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
                        Some(until) => now >= *until, // expired = healthy
                        None => true,                 // not in circuit breaker = healthy
                    }
                })
                .collect()
        }
        None => Vec::new(),
    }
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cd /workspace/llm-gateway && cargo check -p llm-gateway-api`
Expected: compiles

- [ ] **Step 8: Commit**

```bash
git add crates/api/Cargo.toml crates/api/src/proxy.rs
git commit -m "feat(proxy): add in-memory circuit breaker to ChannelRegistry"
```

---

### Task 3: Replace 429 handler — use `disable_channel_model` instead of `disable_channel_until`

**Files:**
- Modify: `crates/api/src/proxy.rs` (lines 1163–1173, the 429 disable block)

- [ ] **Step 1: Replace the 429 disable block**

In proxy.rs, find the block starting with the comment `// Auto-disable channel on 429` (around line 1163). Replace the entire block (through line 1173) with:

Old code to replace:
```rust
            // Auto-disable channel on 429 — parse recovery time or use 30s default
            let disable_until = if let Some(recovery_ts) = parse_recovery_timestamp(&error_body_str) {
                recovery_ts
            } else {
                Utc::now() + chrono::Duration::seconds(30)
            };
            if let Err(e) = state.storage.disable_channel_until(&channel.channel_id.to_string(), disable_until).await {
                tracing::error!("[PROXY] Failed to disable channel '{}' on 429: {:?}", channel.name, e);
            } else {
                tracing::info!("[PROXY] Channel '{}' disabled until {} (429 rate limit)", channel.name, disable_until);
            }
```

New code:
```rust
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /workspace/llm-gateway && cargo check -p llm-gateway-api`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): use circuit breaker for 429 rate limit handling"
```

---

### Task 4: Replace SSE error handler — use `disable_channel_model` and remove `disable_duration_secs`

**Files:**
- Modify: `crates/api/src/proxy.rs` (lines 475–495 SseAuditParams struct, lines 614–638 SSE error handler, line 1379 construction site)

- [ ] **Step 1: Remove `disable_duration_secs` from `SseAuditParams`**

In the `SseAuditParams` struct (lines 475–495), remove the last field:

Remove:
```rust
    /// Duration to disable channel on SSE error (default 5 minutes)
    pub disable_duration_secs: i64,
```

- [ ] **Step 2: Replace SSE error handler in `process_sse_stream`**

Find the block starting with `if saw_error {` (around line 615). Replace the entire `if saw_error { ... }` block (through line 638) with:

Old code to replace:
```rust
        if saw_error {
            tracing::warn!(
                "[PROXY] SSE error event received on channel '{}': {} — disabling for {}s",
                audit_params.channel_id,
                error_message,
                audit_params.disable_duration_secs
            );

            // Parse recovery time from error message if available
            let disable_until = if let Some(recovery_ts) = parse_recovery_timestamp(&error_message) {
                recovery_ts
            } else {
                Utc::now() + chrono::Duration::seconds(audit_params.disable_duration_secs)
            };

            // Disable channel in database
            if let Err(e) = state.storage.disable_channel_until(&audit_params.channel_id, disable_until).await {
                tracing::error!("[PROXY] Failed to disable channel '{}': {:?}", audit_params.channel_id, e);
            } else {
                tracing::info!("[PROXY] Channel '{}' disabled until {}", audit_params.channel_id, disable_until);
            }

            break 'outer;
        }
```

New code:
```rust
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
```

- [ ] **Step 3: Remove `disable_duration_secs` from `SseAuditParams` construction**

At the `SseAuditParams` construction site (around line 1361–1380), remove the line:

```rust
                disable_duration_secs: 300, // 5 minutes default
```

- [ ] **Step 4: Verify it compiles**

Run: `cd /workspace/llm-gateway && cargo check -p llm-gateway-api`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): use circuit breaker for SSE error handling"
```

---

### Task 5: Add circuit breaker filter to cache-miss fallback path

**Files:**
- Modify: `crates/api/src/proxy.rs` (after line 995, before the cache-miss path returns candidates)

- [ ] **Step 1: Add circuit breaker filter after cache-miss candidate collection**

In the cache-miss fallback path, find the line `candidates.sort_by(|a, b| a.0.priority.cmp(&b.0.priority));` (around line 995). Add a filter immediately after it:

```rust
        candidates.sort_by(|a, b| a.0.priority.cmp(&b.0.priority));
        // Filter out circuit-broken (channel, model) combinations
        let model_lower = model_name.to_lowercase();
        candidates.retain(|(ch, _)| {
            !state.registry.is_circuit_broken(&ch.channel_id.to_string(), &model_lower)
        });
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /workspace/llm-gateway && cargo check -p llm-gateway-api`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): filter circuit-broken entries in cache-miss fallback path"
```

---

### Task 6: Build and test

**Files:** None (verification only)

- [ ] **Step 1: Full workspace build**

Run: `cd /workspace/llm-gateway && cargo build --release`
Expected: compiles without errors

- [ ] **Step 2: Run unit tests**

Run: `cd /workspace/llm-gateway && cargo test --lib --workspace`
Expected: all tests pass

- [ ] **Step 3: Final commit if any fixups needed**

Only if something needed fixing in steps 1–2.
