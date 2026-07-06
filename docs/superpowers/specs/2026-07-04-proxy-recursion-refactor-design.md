# Proxy Recursion Refactor Design

**Date:** 2026-07-04
**Status:** Draft (pending user review)
**Targets release:** v1.8.4
**Supersedes:** the `client_model: Option<String>` band-aid shipped in v1.8.3 (commit 69c3c51)

## Problem

`proxy_inner` in `crates/api/src/proxy.rs` (1715 lines) mixes two kinds of work in one function:

1. **Once-per-HTTP-request work**: auth (`get_key_by_hash`), balance check (`get_account_by_user_id`), user role lookup (`get_user`), body parse for `client_requested_model`, `request_id` generation.
2. **Routing + upstream + audit**: model lookup, channel resolution, failover loop, fallback dispatch, AuditTask construction.

`try_model_fallback` (proxy.rs:743) recurses into `proxy_inner` for each fallback model. Every recursion re-runs (1). For a request that fans out across N fallback models, that is **(N+1)× auth/balance/role DB roundtrips** for one client request.

v1.8.3 papered over a *symptom* of this structure — `original_model` getting clobbered by the substituted fallback — by threading a `client_model: Option<String>` parameter through `proxy_inner`. That made the audit row correct but left the redundant DB calls in place.

## Goal

Lift the once-per-request work out of the recursion. Auth/balance/role/parse each run exactly **once per HTTP request**, regardless of how many fallback models the request eventually tries.

## Non-Goals

- No DB schema changes.
- No AuditTask / audit_logs field changes (v1.8.0 `routes`, v1.8.2 `original_model` stay as-is).
- No ChannelRegistry, storage, or audit-worker interface changes.
- No HTTP handler signature changes (the four entry points `proxy`, `proxy_with_protocol`, `messages`, `responses` keep their current Axum signatures).
- No frontend changes.

## Architecture

### Function split

```
HTTP request
  ↓
proxy / proxy_with_protocol / messages / responses   (Axum handlers — unchanged)
  ↓
proxy_inner                                          (NOT recursive)
  • Step 1: auth         (get_key_by_hash)           [DB ×1]
  • Step 2: balance      (get_account_by_user_id)    [DB ×1]
  • Step 2.5: user role  (get_user)                  [DB ×1]
  • Step 3: parse body → client_requested_model
  • gen request_id
  ↓
proxy_route_and_forward                              (recursive — safe)
  • list_models, find model_entry
  • resolve channels (cache-first or DB fallback)
  • failover loop: upstream call, AuditTask on 4xx / success / all-failed
  • on routing miss or all-channels-failed: try_model_fallback
  ↓
try_model_fallback                                    (recursive — safe)
  • load fallback_config
  • for each fallback_model:
        rewrite body
        → proxy_route_and_forward(fallback_depth = 1)
```

### Function signatures

**`proxy_inner`** — entry point. Not recursive.

```rust
async fn proxy_inner(
    state: Arc<AppState>,
    headers: HeaderMap,
    body: String,
    protocol: ProxyProtocol,
    request_path: String,
) -> Result<axum::response::Response, ApiError>
```

Removed parameters vs v1.8.3: `fallback_depth: u32` (entry call always passes 0), `client_model: Option<String>` (replaced by `client_requested_model: String` extracted in this function).

**`proxy_route_and_forward`** — routing core. Safe to recurse into.

```rust
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
) -> Result<axum::response::Response, ApiError>
```

`req_json` is moved in by value, parsed by the caller. For the entry call from `proxy_inner`, it's the body parsed once at step 3. For the recursive call from `try_model_fallback`, it's the body parsed inside the fallback loop after the model field is rewritten. `body` is kept as a String for the audit `request_body` field and as a serialization fallback.

**`try_model_fallback`** — fallback executor. Recurses into `proxy_route_and_forward`.

```rust
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
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<axum::response::Response>> + Send + 'a>>
```

Removed parameter vs v1.8.3: `original_model: &'a str`. The semantic equivalent (`client_requested_model`) is threaded from the entry point; `try_model_fallback` no longer has its own private name for "the original".

The recursive call inside `try_model_fallback`'s loop becomes:

```rust
let result = Box::pin(proxy_route_and_forward(
    state.clone(),
    headers.clone(),
    fallback_body,
    req_json,                                  // parsed inside the loop
    request_id.to_string(),
    client_requested_model.to_string(),
    protocol,
    request_path.to_string(),
    api_key.clone(),
    request_user_id.clone(),
    request_is_admin,
    1,                                         // fallback_depth — prevents re-trigger
)).await;
```

## Data Flow

### Direct path (client model hits a channel)

```
HTTP request
  → proxy_inner
      Step 1: get_key_by_hash          [DB ×1]
      Step 2: get_account_by_user_id   [DB ×1]
      Step 2.5: get_user               [DB ×1]
      Step 3: parse body, extract client_requested_model
      gen request_id
      → proxy_route_and_forward(depth=0)
          list_models                   [DB ×1]
          resolve channels
          for channel in candidates:
              upstream HTTP call        [HTTP ×1]
              on 200: dispatch AuditTask, return Ok
              on 4xx: dispatch AuditTask (terminal), return
              on 5xx / conn err / 429: push RouteAttempt, continue
          all-failed: dispatch AuditTask, return Err(502)
```

DB calls per request: 4 fixed + 0 per upstream attempt.

### Fallback path (client model not in DB)

```
HTTP request
  → proxy_inner
      Steps 1-3 + request_id           [DB ×3 — once]
      → proxy_route_and_forward(depth=0)
          list_models                   [DB ×1]
          model not found
          → try_model_fallback
              get_model_fallback        [DB ×1]
              for fallback_model in group:
                  rewrite body
                  → proxy_route_and_forward(depth=1)
                      list_models       [DB ×1]
                      resolve channels
                      for channel: upstream call
                      on success: dispatch AuditTask, return Ok
          (no fallback succeeds) → return None → Err(404)
```

DB calls per N-fallback request: 3 (one-time) + 1 (initial list_models) + 1 (get_model_fallback) + N × 1 (list_models per attempt) + N × upstream HTTP.

Auth/balance/role run **exactly once**, regardless of N.

## Invariants

- `request_id` is generated once per HTTP request and shared across every fallback attempt. All AuditTasks for one client request share the same `request_id`. This preserves the v1.8.0 "one audit row per client request, multiple entries in `routes`" invariant.
- `client_requested_model` is extracted once at the entry point from the unmodified body and threaded through. It is never substituted, even when `try_model_fallback` rewrites the body's `model` field for the recursive call. This preserves the v1.8.2/v1.8.3 `original_model` correctness invariant.
- `fallback_depth` only takes values 0 (entry call) and 1 (recursive call from `try_model_fallback`). `proxy_route_and_forward` gates every `try_model_fallback` call site behind `if fallback_depth == 0`, preventing infinite recursion.

## Audit Behavior

Unchanged from v1.8.3. The four AuditTask dispatch sites (4xx terminal, streaming success, non-streaming success, all-channels-failed) all carry:

- `original_model: Some(client_requested_model.clone())` — the client's verbatim request
- `model_name: upstream_name.to_string()` — the DB-canonical model that was actually called
- `upstream_model: Some(...)` when the channel maps the model to a different upstream name
- `routes: Vec<RouteAttempt>` — every channel attempt, including fallback attempts, in order

The frontend `/admin/logs` page (Logs.tsx) requires no changes.

## Testing

- `cargo build --release` — borrow checker validates that `try_model_fallback` no longer references `proxy_inner` and that all five call sites pass the new signature.
- `cargo test --workspace` (with `DATABASE_URL` set) — existing 17+ test suites pass unchanged.
- Live verification (required — unit tests do not cover the full channel-routing + fallback chain):
  1. **Direct path**: client sends a registered model that resolves to a channel. Audit log shows `original_model == model_name`, both equal to the client's request.
  2. **Fallback path**: client sends an unregistered model that has a fallback group. Audit log shows `original_model = <client's model>`, `model_name = <fallback-resolved model>`, and `routes` contains entries for each fallback attempt.
  3. **No N+1 on auth/role**: gateway log or Postgres `pg_stat_statements` confirms `get_user` / `get_key_by_hash` / `get_account_by_user_id` each run exactly once per HTTP request, not (N+1)× for N fallback attempts.

## Risks

- **Five `try_model_fallback` call sites** (proxy.rs:947/988/1062/1128/1659) must all be updated to the new signature. Missed sites fail to compile — compiler enforces completeness.
- **Four HTTP handler call sites** (proxy.rs:1724/1733/1742/1751) must drop the `0, None` arguments. Same compiler enforcement.
- **`req_json` ownership**: proxy_inner moves `req_json` into `proxy_route_and_forward`. The upstream-body-modification block (currently `let mut req_json_modified = req_json.clone();` at proxy.rs:1161) keeps working — it clones the moved-in parameter. Logic-equivalent.
- **Behavioral parity**: audit log fields, routes array, status codes, original_model, model_name all match v1.8.3 byte-for-byte. The frontend Logs page is unaffected.

## Release Plan

1. Implement on `feature/refactor-proxy-recursion` (already branched from develop).
2. `cargo build --release` + `cargo test --workspace`.
3. Restart `gateway` and `audit-worker`; run the three live verification scenarios.
4. Merge `feature/refactor-proxy-recursion` → `develop`.
5. Cut `release/v1.8.4` from develop.
6. Bump versions: backend 1.8.3 → 1.8.4 (12 Cargo.toml + Cargo.lock); frontend 0.16.6 → 0.16.7 (web/package.json).
7. Add CHANGELOG entry for 1.8.4.
8. Merge `release/v1.8.4` → main + develop; tag `v1.8.4`; push.
9. Delete the release branch.

## CHANGELOG Draft

```markdown
## [1.8.4] - 2026-07-04

### Changed
- `proxy_inner` split: once-only work (auth, balance check, user role lookup, body parse, request_id generation) now runs in `proxy_inner` proper, which is not recursive. Routing, failover, fallback, and audit dispatch live in a new `proxy_route_and_forward` which is safe to recurse into. `try_model_fallback` now recurses into `proxy_route_and_forward` instead of `proxy_inner`.

### Fixed
- Each fallback attempt no longer re-runs `get_key_by_hash`, `get_account_by_user_id`, and `get_user`. A request that fans out across N fallback models used to make (N+1)× auth/balance/role DB calls; now it is 1× each per HTTP request.

### Removed
- The `client_model: Option<String>` parameter on `proxy_inner` (added in 1.8.3 as a band-aid) is gone. `client_requested_model` is now a `String` threaded from `proxy_inner`.
```
