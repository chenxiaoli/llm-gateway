# Proxy Recursion Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lift auth/balance/role/body-parse out of `proxy_inner`'s recursion path so each runs exactly once per HTTP request, regardless of how many fallback models the request tries.

**Architecture:** Split `proxy_inner` (which today does both once-only work AND routing/upstream/audit AND is recursed into by `try_model_fallback`) into two functions: `proxy_inner` (entry point, not recursive, does Steps 1-3 + `request_id`) and `proxy_route_and_forward` (routing core, recursive, safe). `try_model_fallback` now recurses into `proxy_route_and_forward` instead of `proxy_inner`. The v1.8.3 `client_model: Option<String>` band-aid parameter goes away — `client_requested_model` becomes a `String` threaded from the entry point.

**Tech Stack:** Rust workspace (Axum, sqlx, async-nats, serde_json), PostgreSQL, NATS, git flow release process.

**Spec:** `docs/superpowers/specs/2026-07-04-proxy-recursion-refactor-design.md`

---

## File Structure

| File | Responsibility |
|------|----------------|
| `crates/api/src/proxy.rs` | The only code file touched. Rename `proxy_inner` → `proxy_route_and_forward` with new signature (params in: `req_json`, `request_id`, `client_requested_model: String`, `api_key`, `request_user_id`, `request_is_admin`, `fallback_depth`). Move Steps 1-3 + `request_id` generation OUT of `proxy_route_and_forward` and INTO a new `proxy_inner` entry function. Update `try_model_fallback` signature (drop `original_model`, add `client_requested_model`, `request_user_id`, `request_is_admin`, `request_id`); its recursive call now hits `proxy_route_and_forward`. Update 5 internal call sites of `try_model_fallback` and 4 HTTP handler call sites of `proxy_inner`. |
| `CHANGELOG.md` | Add `## [1.8.4] - 2026-07-04` entry. |
| `Cargo.lock` | Regenerate via `cargo update --workspace` after Cargo.toml bumps. |
| `crates/*/Cargo.toml` (12 files) | Bump `version = "1.8.3"` → `version = "1.8.4"`. |
| `web/package.json` | Bump `"version": "0.16.6"` → `"version": "0.16.7"`. |

---

## Task 1: Refactor proxy_inner into entry + routing core

**Files:**
- Modify: `crates/api/src/proxy.rs`

All steps in this task land in a single commit at the end. Compile errors between steps are expected — the compiler enforces completeness across the 5 internal `try_model_fallback` call sites and 4 HTTP handler call sites.

- [ ] **Step 1: Rename proxy_inner → proxy_route_and_forward and change signature**

Find this block at `crates/api/src/proxy.rs:840-854`:

```rust
/// Unified proxy: receives request → forwards to upstream → returns response
/// Usage/Cost/Audit are handled in spawned async tasks
async fn proxy_inner(
    state: Arc<AppState>,
    headers: HeaderMap,
    body: String,
    protocol: ProxyProtocol,
    request_path: String,
    fallback_depth: u32,
    // Original model the client requested, threaded through recursive
    // `try_model_fallback` calls. When set, overrides extraction from the
    // (possibly substituted) request body. `None` for the initial call from
    // the HTTP handler.
    client_model: Option<String>,
) -> Result<axum::response::Response, ApiError> {
```

Replace with:

```rust
/// Routing core: model lookup → channel resolution → failover → audit.
/// Safe to recurse into — no auth, balance, or role lookups run here.
/// Entry point is `proxy_inner`; recursion comes from `try_model_fallback`.
async fn proxy_route_and_forward(
    state: Arc<AppState>,
    headers: HeaderMap,
    body: String,
    req_json: serde_json::Value,
    request_id: String,
    client_requested_model: String,
    protocol: ProxyProtocol,
    request_path: String,
    api_key: llm_gateway_storage::ApiKey,
    request_user_id: Option<String>,
    request_is_admin: bool,
    fallback_depth: u32,
) -> Result<axum::response::Response, ApiError> {
```

- [ ] **Step 2: Remove the once-only work from the top of proxy_route_and_forward**

Find this block (now inside `proxy_route_and_forward`, was `proxy_inner:855-924`):

```rust
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

    // Capture the client's original request. In the fallback-recursion path
    // `try_model_fallback` rewrites the body's `model` field before recursing,
    // so `model_name` here may already be the substituted fallback — the
    // threaded `client_model` (when set) is authoritative. In the direct path
    // from the HTTP handler, the body is unmodified and `model_name` is the
    // client's request verbatim.
    let client_requested_model = client_model.unwrap_or_else(|| model_name.clone());
```

Replace with:

```rust
    // === Parse model (req_json is parsed by the caller and moved in) ===
    let model_name = req_json
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or(ApiError::BadRequest("Missing 'model' field".to_string()))?
        .to_string();

    tracing::debug!(
        "[PROXY] Incoming request, model: {}, protocol: {:?}, fallback_depth: {}",
        model_name, protocol, fallback_depth
    );
```

What changed: removed Steps 1, 2, 2.5, the body parse (now done by `proxy_inner`), `request_id` generation (now done by `proxy_inner`), and the `client_model.unwrap_or_else` line (`client_requested_model` is now a parameter). Kept `model_name` extraction because `proxy_route_and_forward` still needs it for routing.

- [ ] **Step 3: Update try_model_fallback signature**

Find at `crates/api/src/proxy.rs:741-751`:

```rust
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
```

Replace with:

```rust
/// Try fallback models when the initial model fails to route.
/// Returns Some(response) if a fallback succeeded, None if no fallback available.
/// Recurses into `proxy_route_and_forward` (not `proxy_inner`) so auth/balance/role
/// are not re-run on each attempt.
fn try_model_fallback<'a>(
    state: &'a Arc<AppState>,
    headers: &'a HeaderMap,
    body: &'a str,
    client_requested_model: &'a str,
    api_key: &'a llm_gateway_storage::ApiKey,
    request_user_id: &'a Option<String>,
    request_is_admin: bool,
    request_id: &'a str,
    protocol: ProxyProtocol,
    request_path: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<axum::response::Response>> + Send + 'a>> {
```

- [ ] **Step 4: Replace `original_model` references inside try_model_fallback**

Inside `try_model_fallback`, three lines reference `original_model` (the parameter we just renamed to `client_requested_model`). At `crates/api/src/proxy.rs:771`:

```rust
        let model_lower = original_model.to_lowercase();
```

Replace with:

```rust
        let model_lower = client_requested_model.to_lowercase();
```

At `crates/api/src/proxy.rs:778-781`:

```rust
                tracing::debug!(
                    "[PROXY] No fallback group contains model '{}' (config '{}' has {} groups) — fallback skipped",
                    original_model, fallback_config.name, fallback_config.config.len()
                );
```

Replace with:

```rust
                tracing::debug!(
                    "[PROXY] No fallback group contains model '{}' (config '{}' has {} groups) — fallback skipped",
                    client_requested_model, fallback_config.name, fallback_config.config.len()
                );
```

At `crates/api/src/proxy.rs:796`:

```rust
            tracing::info!("[PROXY] Trying fallback model '{}' for failed '{}'", fallback_model, original_model);
```

Replace with:

```rust
            tracing::info!("[PROXY] Trying fallback model '{}' for failed '{}'", fallback_model, client_requested_model);
```

- [ ] **Step 5: Lift req_json out of the inner block in try_model_fallback and update recursive call**

Find at `crates/api/src/proxy.rs:798-822`:

```rust
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
                Some(original_model.to_string()), // preserve the client's original request across recursion
            )).await;
```

Replace with:

```rust
            // Parse body, rewrite the model field, and keep both the serialized
            // fallback_body (for upstream forwarding) and the modified req_json
            // (to pass into the recursive call without re-parsing).
            let (fallback_body, fallback_req_json) = {
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
                let serialized = serde_json::to_string(&modified).unwrap_or_else(|_| body.to_string());
                (serialized, modified)
            };

            // Box the recursive proxy call to satisfy the compiler's sizing requirement.
            // Recurse into proxy_route_and_forward (not proxy_inner) so auth/balance/role
            // are not re-run on each fallback attempt.
            let result = Box::pin(proxy_route_and_forward(
                state.clone(),
                headers.clone(),
                fallback_body,
                fallback_req_json,
                request_id.to_string(),
                client_requested_model.to_string(),
                protocol,
                request_path.to_string(),
                api_key.clone(),
                request_user_id.clone(),
                request_is_admin,
                1, // fallback depth — prevents re-triggering fallback in recursive call
            )).await;
```

- [ ] **Step 6: Update the 5 try_model_fallback call sites inside proxy_route_and_forward**

All 5 call sites have identical argument shape. They reference `&model_name` for the `original_model` parameter (now `client_requested_model`). Use `replace_all` to update them in one edit.

Find (occurs 5 times — at lines 947, 988, 1062, 1128, 1659 in the original file):

```rust
                if let Some(resp) = try_model_fallback(
                    &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
                ).await {
```

Replace with (use `replace_all: true`):

```rust
                if let Some(resp) = try_model_fallback(
                    &state, &headers, &body, &client_requested_model, &api_key,
                    &request_user_id, request_is_admin, &request_id,
                    protocol, &request_path,
                ).await {
```

Note: the indentation may vary slightly between sites (some are nested deeper). The `replace_all` should match the common pattern. If it matches fewer than 5 sites, edit the remaining sites individually with more context.

- [ ] **Step 7: Add the new proxy_inner entry function**

Insert a new function immediately above the `/// Routing core:` doc-comment on `proxy_route_and_forward` (i.e., where the old `/// Unified proxy:` doc-comment used to start before Step 1 renamed it).

Add:

```rust
/// HTTP handler entry point. Runs once-per-request work (auth, balance check,
/// user role lookup, body parse, request_id) and delegates the rest to
/// `proxy_route_and_forward`. Not recursive — `try_model_fallback` recurses
/// into `proxy_route_and_forward` instead.
async fn proxy_inner(
    state: Arc<AppState>,
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

    // === Step 3: Parse body and capture client_requested_model ===
    let req_json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let client_requested_model = req_json
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or(ApiError::BadRequest("Missing 'model' field".to_string()))?
        .to_string();

    let request_id = uuid::Uuid::new_v4().to_string();

    proxy_route_and_forward(
        state,
        headers,
        body,
        req_json,
        request_id,
        client_requested_model,
        protocol,
        request_path,
        api_key,
        request_user_id,
        request_is_admin,
        0,  // fallback_depth — entry call; allows try_model_fallback to fire
    )
    .await
}

```

- [ ] **Step 8: Update the 4 HTTP handler call sites**

Find at `crates/api/src/proxy.rs:1717-1752` (the four handlers `proxy`, `proxy_with_protocol`, `messages`, `responses`):

```rust
    proxy_inner(state, headers, body, protocol, request_path, 0, None).await
}

/// Wrapper for /v1/chat/completions - uses OpenAI protocol
pub async fn proxy_with_protocol(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::OpenAI, "/v1/chat/completions".to_string(), 0, None).await
}

/// Wrapper for /v1/messages - uses Anthropic protocol
pub async fn messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::Anthropic, "/v1/messages".to_string(), 0, None).await
}

/// Wrapper for /v1/responses - uses OpenAI protocol, passthrough all fields
pub async fn responses(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::OpenAI, "/v1/responses".to_string(), 0, None).await
}
```

Replace with:

```rust
    proxy_inner(state, headers, body, protocol, request_path).await
}

/// Wrapper for /v1/chat/completions - uses OpenAI protocol
pub async fn proxy_with_protocol(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::OpenAI, "/v1/chat/completions".to_string()).await
}

/// Wrapper for /v1/messages - uses Anthropic protocol
pub async fn messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::Anthropic, "/v1/messages".to_string()).await
}

/// Wrapper for /v1/responses - uses OpenAI protocol, passthrough all fields
pub async fn responses(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::OpenAI, "/v1/responses".to_string()).await
}
```

- [ ] **Step 9: Build the release binary**

Run:

```bash
cargo build --release
```

Expected: `Finished \`release\` profile [optimized] target(s)` with no errors. The 2 pre-existing warnings in `crates/api/src/proxy.rs` (unused `next_is_error` assignment, etc.) are unrelated and tolerable.

If compile errors mention `proxy_inner` or `try_model_fallback`, re-check that:
- All 5 internal call sites of `try_model_fallback` were updated (Step 6)
- The recursive call inside `try_model_fallback` references `proxy_route_and_forward` not `proxy_inner` (Step 5)
- All 4 HTTP handler call sites dropped `, 0, None` (Step 8)
- The new `proxy_inner` was inserted above `proxy_route_and_forward` (Step 7)

- [ ] **Step 10: Run the full test suite**

Run:

```bash
DATABASE_URL="postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway" cargo test --workspace 2>&1 | tail -30
```

Expected: every `test result:` line ends with `0 failed`. The relevant suites are typically:
- `llm-gateway-api` (lib + tests): 17+ tests in test_auth, plus routing/storage tests
- `llm-gateway-storage`, `llm-gateway-billing`, etc.

If any test fails, DO NOT commit. Investigate the failure — most likely a missed call site or a moved variable.

- [ ] **Step 11: Restart the gateway and audit-worker**

Stop the currently running processes and start fresh ones with the new binary.

```bash
# Find current gateway PIDs (non-defunct)
ps -eo pid,etime,cmd | grep -E "target/release/llm-gateway" | grep -v grep | grep -v defunct

# Stop them gracefully (replace <pid> with the actual PIDs above)
kill <gateway-pid> <audit-worker-pid>

# Wait briefly, then start fresh in the background
./target/release/llm-gateway &
sleep 2
./target/release/llm-gateway-audit-worker &
```

Expected: both processes start without `Failed to connect to NATS` or other startup errors. Verify with:

```bash
curl -s http://localhost:8080/health 2>&1 | head -5
```

Should return HTML (the admin app shell), not a connection error.

- [ ] **Step 12: Live-verify scenario 1 — direct path**

Pick an existing test API key and send a request for a model that already maps to a channel.

```bash
# Get a test API key (create one if none exists)
PGPASSWORD=Xabc12345 psql -h 10.0.17.3 -U llm_gateway -d llm_gateway -c "SELECT id FROM api_keys WHERE name LIKE 'test%' OR name LIKE 'fb-verify%' LIMIT 1;"
```

If a key exists, capture its hash from the keys admin page or generate a fresh one:

```bash
# Generate a fresh test key (one-shot, no UI needed)
curl -s -X POST http://localhost:8080/api/v1/admin/keys \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{"name":"verify-184-direct","enabled":true}' | jq -r '.key'
```

Send a direct-path request:

```bash
KEY=<the key from above>
MODEL=<a model that exists in your DB, e.g. glm-5.1>

curl -s -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"$MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"max_tokens\":10}" \
  | jq '.choices[0].message.content'
```

Expected: a valid completion response from upstream.

Then check the audit log:

```bash
PGPASSWORD=Xabc12345 psql -h 10.0.17.3 -U llm_gateway -d llm_gateway -c "
  SELECT key_id, model_name, original_model, status_code, jsonb_array_length(routes) AS route_count, created_at
  FROM audit_logs
  WHERE key_id = (SELECT id FROM api_keys WHERE name = 'verify-184-direct')
  ORDER BY created_at DESC LIMIT 5;"
```

Expected: `model_name` and `original_model` both equal `$MODEL`. `route_count >= 1`. `status_code = 200`.

- [ ] **Step 13: Live-Verify scenario 2 — fallback path**

Set up a fallback config that maps an unregistered model to a real one. (If a `model_fallbacks` row already exists from prior debugging, reuse it; otherwise create one.)

```bash
# Create fallback config that maps "client-unknown-184" → real model
curl -s -X POST http://localhost:8080/api/v1/admin/model-fallbacks \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-fb-184",
    "name": "verify-184-fallback",
    "config": [{"models": ["client-unknown-184", "'"$MODEL"'"], "priorities": [1, 2]}]
  }' | jq .

# Attach to the test key
curl -s -X PATCH http://localhost:8080/api/v1/admin/keys/$(PGPASSWORD=Xabc12345 psql -h 10.0.17.3 -U llm_gateway -d llm_gateway -tAc "SELECT id FROM api_keys WHERE name='verify-184-direct'") \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{"model_fallback_id":"test-fb-184"}' | jq .
```

Send a request for the unregistered model:

```bash
curl -s -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"client-unknown-184\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"max_tokens\":10}" \
  | jq '.choices[0].message.content'
```

Expected: a valid completion response from upstream (served by fallback resolution).

Then check the audit log:

```bash
PGPASSWORD=Xabc12345 psql -h 10.0.17.3 -U llm_gateway -d llm_gateway -c "
  SELECT key_id, model_name, original_model, status_code, jsonb_array_length(routes) AS route_count, created_at
  FROM audit_logs
  WHERE key_id = (SELECT id FROM api_keys WHERE name = 'verify-184-direct')
  ORDER BY created_at DESC LIMIT 5;"
```

Expected: the most recent row has `model_name = $MODEL` (the fallback-resolved channel model) but `original_model = 'client-unknown-184'` (the client's actual request). `status_code = 200`. `route_count >= 1`.

- [ ] **Step 14: Live-Verify scenario 3 — no N+1 on auth/role**

Static verification (cheap, fast, definitive). Confirm that `state.storage.get_user`, `state.storage.get_key_by_hash`, and `state.storage.get_account_by_user_id` only appear in `proxy_inner` and NOT in `proxy_route_and_forward` or `try_model_fallback`.

Run:

```bash
# Should print exactly one match: inside proxy_inner
grep -n "state\.storage\.get_user\b\|state\.storage\.get_key_by_hash\b\|state\.storage\.get_account_by_user_id\b" crates/api/src/proxy.rs
```

Expected output (the exact line numbers may shift slightly due to the refactor):

```
<line>:            .get_key_by_hash(&token_hash)
<line>:            .get_account_by_user_id(created_by)
<line>:        match state.storage.get_user(uid).await {
```

Three matches total, all inside the new `proxy_inner` entry function. If any match falls inside `proxy_route_and_forward` or `try_model_fallback`, the refactor is incomplete — fix and re-build.

- [ ] **Step 15: Commit the refactor**

Stage only `proxy.rs`. The plan docs and spec are already committed.

```bash
git status --short
git add crates/api/src/proxy.rs
git commit -m "$(cat <<'EOF'
refactor(proxy): split proxy_inner into entry + routing core

proxy_inner is no longer recursive. It runs once-per-HTTP-request work
(auth via get_key_by_hash, balance check via get_account_by_user_id,
user role via get_user, body parse, request_id generation) and delegates
the rest to proxy_route_and_forward.

proxy_route_and_forward is the routing core: model lookup, channel
resolution, failover loop, audit dispatch. It is safe to recurse into
because it does not re-run any of the once-per-request work.

try_model_fallback now recurses into proxy_route_and_forward (not
proxy_inner) and threads client_requested_model, request_user_id,
request_is_admin, and request_id through the recursion.

Removes the client_model: Option<String> band-aid added in 1.8.3
(commit 69c3c51). client_requested_model is now a String threaded
from the entry point — no more unwrap_or_else.

A request that fans out across N fallback models previously made
(N+1)× auth/balance/role DB calls; now it is 1× each per HTTP request.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

Expected: `[feature/refactor-proxy-recursion <sha>] refactor(proxy): split proxy_inner into entry + routing core`.

---

## Task 2: Release v1.8.4

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `Cargo.lock`
- Modify: `crates/api/Cargo.toml`, `crates/audit-worker/Cargo.toml`, `crates/audit/Cargo.toml`, `crates/auth/Cargo.toml`, `crates/billing/Cargo.toml`, `crates/encryption/Cargo.toml`, `crates/gateway/Cargo.toml`, `crates/nats-publisher/Cargo.toml`, `crates/provider/Cargo.toml`, `crates/ratelimit/Cargo.toml`, `crates/storage/Cargo.toml`, `crates/usage-worker/Cargo.toml`
- Modify: `web/package.json`

- [ ] **Step 1: Merge feature/refactor-proxy-recursion → develop**

```bash
git checkout develop
git merge --no-ff feature/refactor-proxy-recursion -m "Merge branch 'feature/refactor-proxy-recursion' into develop

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

Expected: `Merge made by the 'recursive' strategy.` with `crates/api/src/proxy.rs` and the spec doc as the changed files.

- [ ] **Step 2: Cut release/v1.8.4 from develop**

```bash
git checkout -b release/v1.8.4
```

- [ ] **Step 3: Bump backend crate versions 1.8.3 → 1.8.4**

Run:

```bash
for f in crates/api crates/audit-worker crates/audit crates/auth crates/billing crates/encryption crates/gateway crates/nats-publisher crates/provider crates/ratelimit crates/storage crates/usage-worker; do
  sed -i 's/^version = "1.8.3"$/version = "1.8.4"/' "$f/Cargo.toml"
done
grep -H '^version' crates/*/Cargo.toml
```

Expected: all 12 lines print `version = "1.8.4"`.

- [ ] **Step 4: Bump frontend version 0.16.6 → 0.16.7**

Edit `web/package.json` line 4: change `"version": "0.16.6",` to `"version": "0.16.7",`.

Verify:

```bash
grep '"version"' web/package.json
```

Expected: `  "version": "0.16.7",`.

- [ ] **Step 5: Regenerate Cargo.lock**

```bash
cargo update --workspace 2>&1 | tail -15
```

Expected: 12 lines like `Updating llm-gateway-X v1.8.3 (/workspace/llm-gateway/crates/X) -> v1.8.4`. The remaining "unchanged dependencies" note is fine.

- [ ] **Step 6: Add CHANGELOG entry**

Edit `CHANGELOG.md`. Insert this block above the `## [1.8.3] - 2026-07-04` line:

```markdown
## [1.8.4] - 2026-07-04

### Changed
- `proxy_inner` split: once-only work (auth via `get_key_by_hash`, balance check via `get_account_by_user_id`, user role via `get_user`, body parse, `request_id` generation) now runs in `proxy_inner` proper, which is not recursive. Routing, failover, fallback, and audit dispatch live in a new `proxy_route_and_forward` which is safe to recurse into. `try_model_fallback` now recurses into `proxy_route_and_forward` instead of `proxy_inner`.

### Fixed
- Each fallback attempt no longer re-runs `get_key_by_hash`, `get_account_by_user_id`, and `get_user`. A request that fans out across N fallback models used to make (N+1)× auth/balance/role DB calls; now it is 1× each per HTTP request.

### Removed
- The `client_model: Option<String>` parameter on `proxy_inner` (added in 1.8.3 as a band-aid) is gone. `client_requested_model` is now a `String` threaded from `proxy_inner`.

```

- [ ] **Step 7: Verify build with new versions**

```bash
cargo build --release 2>&1 | tail -5
```

Expected: `Finished \`release\` profile [optimized]` with no errors.

- [ ] **Step 8: Commit version bump**

```bash
git add CHANGELOG.md Cargo.lock crates/*/Cargo.toml web/package.json
git status --short
```

Verify the staged file list contains exactly 15 files (CHANGELOG, Cargo.lock, 12 Cargo.toml, package.json). Untracked `docs/superpowers/plans/*.md` files from prior sessions should NOT be staged.

```bash
git commit -m "$(cat <<'EOF'
chore: bump to 1.8.4 / 0.16.7 for proxy recursion refactor

Backend (Rust crates): 1.8.3 → 1.8.4
Frontend (web/package.json): 0.16.6 → 0.16.7

Splits proxy_inner into an entry function (once-only auth/balance/role/
parse/request_id) and proxy_route_and_forward (routing core, recursive).
try_model_fallback recurses into proxy_route_and_forward, not proxy_inner.
Removes the client_model band-aid from 1.8.3.

A request that fans out across N fallback models used to make
(N+1)× auth/balance/role DB calls; now it is 1× each per HTTP request.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 9: Merge release/v1.8.4 into main, tag, merge into develop**

```bash
git checkout main
git merge --no-ff release/v1.8.4 -m "Merge release/v1.8.4 into main

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"

git tag -a v1.8.4 -m "v1.8.4 — proxy recursion refactor (no N+1 auth/balance/role)"

git checkout develop
git merge --no-ff release/v1.8.4 -m "Merge release/v1.8.4 into develop

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

- [ ] **Step 10: Delete local release branch**

```bash
git branch -d release/v1.8.4
```

Expected: `Deleted branch release/v1.8.4 (was <sha>).`

- [ ] **Step 11: Push everything**

```bash
GIT_SSL_NO_VERIFY=1 git push origin main develop --follow-tags
GIT_SSL_NO_VERIFY=1 git push origin --delete feature/refactor-proxy-recursion
```

Expected:
- `main -> main` updated
- `develop -> develop` updated
- `[new tag] v1.8.4 -> v1.8.4` (annotated)
- `[deleted] feature/refactor-proxy-recursion`

(Use `GIT_SSL_NO_VERIFY=1` to work around the GnuTLS pull-function error seen on previous pushes to this remote.)

- [ ] **Step 12: Delete the local feature branch**

```bash
git branch -d feature/refactor-proxy-recursion
```

- [ ] **Step 13: Verify final state**

```bash
git log --oneline -8
git tag --list "v1.8*"
git branch -a | grep -E "(develop|main|release/v1.8|feature/refactor)"
```

Expected:
- `git log` shows the merge commits and the bump + refactor commits on top of develop and main
- Tags include `v1.8.4`
- No `release/v1.8.4` or `feature/refactor-proxy-recursion` branches remain (local or remote)

---

## Self-Review Notes

- **Spec coverage**: Every section of `2026-07-04-proxy-recursion-refactor-design.md` is implemented.
  - Function split (architecture diagram) → Task 1 Steps 1, 2, 7
  - `proxy_inner` signature (no `fallback_depth`, no `client_model`) → Task 1 Step 7
  - `proxy_route_and_forward` signature → Task 1 Step 1
  - `try_model_fallback` new signature → Task 1 Step 3
  - Recursive call → Task 1 Step 5
  - Direct path data flow → Task 1 Step 12
  - Fallback path data flow → Task 1 Step 13
  - N+1 invariant → Task 1 Step 14
  - Audit behavior unchanged → covered by Task 1 Step 10 (existing tests pass) and Steps 12-13 (audit log fields match v1.8.3)
  - Release plan → Task 2
- **No placeholders**: every step contains either the exact code to insert or the exact command to run.
- **Type consistency**: `proxy_route_and_forward` parameter names match across signature (Step 1), recursive call (Step 5), and `proxy_inner` call site (Step 7). `client_requested_model: String` is consistent across all sites.
