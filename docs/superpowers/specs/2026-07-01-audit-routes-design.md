# Audit Log Routes Field Design

**Date:** 2026-07-01
**Status:** Approved (brainstorming complete)

## Goal

Record every upstream routing attempt inside a single audit log row, so a request that fans out across multiple channels via failover leaves one forensic record of the full route history — not just the winning attempt.

Today, the proxy (since v1.7.2) creates **one audit row per upstream attempt** — a side effect of the fix that made every attempt auditable. The user's example makes the problem with that shape obvious:

> a client llm api request glm-5.2, proxy match first channel, first route is glm-5.2, channel A, fail; match second route: glm-5.2, channel B, fail; match third route: minimax-3, channel C, success. all this routes log to audit_log.routes.

A per-attempt-row design makes the per-client-request story a join: `SELECT * FROM audit_logs WHERE request_id = $1`. A consolidated-routes design makes it a single row read. The new design is what the user described and what the audit pipeline should look like going forward.

## Decisions (from brainstorming)

1. **Replace** per-attempt rows with one row per client request + a `routes` JSONB array. Per-attempt rows are gone.
2. **All-failed case**: the top-level audit row reflects the **final** attempt. The full attempt history is preserved in `routes`.
3. **/admin/logs UI**: new "Routes" column showing attempt count + click-to-expand modal listing each route.
4. **Per-route fields** (full metadata): `model`, `channel_id`, `channel_name`, `status_code`, `error_message`, `latency_ms`, `started_at`.
5. **Token counts**: only on the top-level row, reflecting the **successful** attempt. Failed attempts do not bill.
6. **Top-level `request_body` / `response_body`**: client-original body + final response (or last error message on all-failed).
7. **Pipeline shape**: the proxy assembles `routes` in memory during the failover loop and dispatches **one** NATS `AuditTask` with the array. The audit worker stays a one-task-one-row transformer.

## Scope

**In scope**
- New `routes JSONB` column on `audit_logs` (nullable for legacy rows, no backfill)
- `RouteAttempt` Rust type, shared between storage, nats-publisher, api, audit, audit-worker
- Proxy `proxy_inner` rewrite: collect attempts in a `Vec<RouteAttempt>`, dispatch one event at loop exit
- Audit worker + audit library: forward `routes` through
- Storage: insert / select / list with the new column
- Frontend: Routes column + expand-in-modal in `/admin/logs`
- i18n: Routes column header + modal labels (en + zh)
- Tests: storage round-trip, null-byte sanitization, frontend rendering, manual smoke

**Out of scope**
- Backfill of `routes` for existing per-attempt rows (they stay with `routes = NULL`)
- Filtering / querying into `routes` from the API (no current access pattern needs it)
- Exposing routes in the streaming audit log (SSE)
- Index on `routes` (JSONB, ad-hoc query only)

## Files Changed

**Backend**
- `crates/storage/migrations/postgres/20260701000000_audit_routes.sql` (new)
- `crates/storage/src/types.rs` — `RouteAttempt` struct; add `routes: Option<Vec<RouteAttempt>>` to `AuditLog` and `AuditLogSummary`
- `crates/storage/src/postgres.rs` — `insert_log`, `get_audit_by_request_id`, list/filter queries
- `crates/nats-publisher/src/lib.rs` — add `routes: Option<Vec<RouteAttempt>>` to `AuditEvent`
- `crates/api/src/proxy.rs` — `proxy_inner` rewrite
- `crates/audit/src/lib.rs` — `routes` parameter on `log_request`; sanitize `routes[*].error_message`
- `crates/audit-worker/src/main.rs` — pass `routes` through to `log_request`

**Frontend**
- `web/src/types/index.ts` — `routes?: RouteAttempt[]` on `AuditLogSummary`
- `web/src/pages/Logs.tsx` — Routes column + expand modal
- `web/src/pages/Logs.test.tsx` (or existing test file) — new test cases
- `web/src/i18n/en.json` + `zh.json` — new keys

## Data Model

### `audit_logs.routes` column

JSONB array of `RouteAttempt` objects. Nullable for legacy rows. New rows always populated.

### `RouteAttempt`

```rust
pub struct RouteAttempt {
    pub model: String,                // channel model actually used
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub status_code: i32,             // 0 = connection error
    pub error_message: Option<String>,// None on success
    pub latency_ms: i64,
    pub started_at: DateTime<Utc>,
}
```

### Top-level `AuditLog` semantics (after this change)

- `request_id` — the **client request's** id, shared across all attempts within this row (unchanged)
- `key_id`, `user_id`, `protocol`, `stream` — request-scoped (unchanged)
- `original_model` — the client's original request model (e.g. `glm-5.2`). **Top-level only**, not duplicated per route. (Unchanged meaning.)
- `model_name` — the **final** attempt's channel model (e.g. `minimax-3` on success, last-failed model otherwise)
- `provider_id`, `channel_id`, `channel_name` — the final attempt's channel/provider
- `status_code` — the final attempt's status (success code, or last error code including 0 for connection error)
- `latency_ms` — total wall-clock from request start to final response (sum of all attempt latencies plus the failover gap)
- `request_body` — the **client's** original request body (unchanged from current behavior)
- `response_body` — the successful attempt's response, or the last error message on all-failed
- `request_path`, `upstream_url` — final attempt's path/URL
- `request_headers`, `response_headers` — final attempt's headers
- `input_tokens`, `output_tokens` — successful attempt's tokens (`None` on all-failed)
- `routes` — `Vec<RouteAttempt>` with the full attempt history (in order)
- `created_at` — row insert time

## Proxy Flow

```
proxy_inner(...):
  ...
  let mut routes: Vec<RouteAttempt> = Vec::new();
  let mut final_attempt: Option<FinalAttempt> = None;  // success or last failure
  let mut last_error_message: Option<String> = None;
  let mut all_failed = true;
  let start_total = Instant::now();

  for (channel, channel_model) in &routing_candidates {
    let attempt_start = Instant::now();
    let upstream_name = channel_model.upstream_model_name.as_deref().unwrap_or(&model_name);
    let upstream_url = channel.upstream_url(&request_path, protocol);
    let modified_body = rewrite_model_in_body(&req_json, upstream_name, is_stream, protocol);

    // Build request, send...

    let route_entry = match send_result {
      Err(e) => {
        let err = format!("Connection error on channel '{}': {}", channel.name, e);
        last_error_message = Some(err.clone());
        RouteAttempt {
          model: upstream_name.into(),
          channel_id: channel.channel_id.to_string(),
          channel_name: Some(channel.name.clone()),
          status_code: 0,
          error_message: Some(err),
          latency_ms: attempt_start.elapsed().as_millis() as i64,
          started_at: chrono::Utc::now(),
        }
      }
      Ok(resp) if resp.status().is_success() => {
        let route = RouteAttempt { ... status_code: resp.status().as_u16() as i32, error_message: None, ... };
        all_failed = false;
        final_attempt = Some(FinalAttempt::from(resp));
        route
      }
      Ok(resp) if (400..500).contains(&resp.status().as_u16()) => {
        // 4xx is final — log and return
        let route = RouteAttempt { ... status_code: resp.status().as_u16() as i32, error_message: Some(body), ... };
        routes.push(route);
        routes.push(route);  // 4xx attempt is also recorded
        // ...build AuditTask, dispatch, return 4xx to client
      }
      Ok(resp) => {
        // 5xx — record and try next
        let route = RouteAttempt { ... status_code: resp.status().as_u16() as i32, error_message: Some(...), ... };
        last_error_message = Some(...);
        route
      }
    };
    routes.push(route_entry);
  }

  // Build final AuditTask from final_attempt (or last_error) and dispatch.
  // Stream response if successful; return error to client otherwise.
```

(The above is the structural shape — exact field assembly goes in the implementation plan.)

## NATS Event

`AuditEvent` gains one field:

```rust
pub routes: Option<Vec<RouteAttempt>>,
```

Serialized as `null` or a JSON array. Worker deserializes and forwards to `insert_log`.

## Storage

- `insert_log`: extend the `INSERT` statement. `routes` is `serde_json::to_value(&log.routes)` → `JSONB`, or `NULL` when `None`.
- `get_audit_by_request_id`: extend the `SELECT`. Deserialize via `serde_json::from_value`.
- `list_logs` + filter queries: extend the row struct and projection to include `routes`.

## Audit Worker

No logic change. Just pass `event.routes` through to `audit_logger.log_request`.

`AuditLogger::log_request` adds a `routes: Option<Vec<RouteAttempt>>` parameter. The same null-byte defense-in-depth applied to `response_body` (v1.7.2) is applied to every `routes[*].error_message` (upstream error bodies can also carry U+0000 / U+FFFD).

## Frontend

`AuditLogSummary` gains `routes?: RouteAttempt[]`. `/admin/logs` gains:

- A "Routes" column showing the attempt count, e.g. `3 routes` (small badge)
- Click → opens a modal/drawer listing each route in order, with:
  - `model` + `channel_name` (with link to channel detail)
  - `status_code` color-coded (green=2xx, red=4xx/5xx/0, amber=429)
  - `error_message` (truncated to one line, full on hover/click)
  - `latency_ms` and `started_at`

The existing single-row detail (`handleView`) keeps working: `request_body`, `response_body`, `original_model`, `model_name` are now the final-attempt values, which is what an operator investigating a failure most often wants.

## Testing

**Backend unit/integration**
- `crates/storage/src/postgres.rs`: round-trip test — insert a row with `routes: Some(vec![...])`, fetch by request_id, assert the array round-trips intact.
- `crates/audit/src/lib.rs`: unit test that `routes[*].error_message` containing U+0000 / U+FFFD gets sanitized (parity with the v1.7.2 `response_body` fix).
- `crates/api/src/proxy.rs`: integration test for the new behavior — trigger a 2-fail-then-1-success failover, assert exactly **one** audit row was created, `routes.len() == 3`, top-level `model_name` matches the successful channel, top-level `status_code` is 2xx, `original_model` is the client-original.

**Manual smoke (end-to-end)**
- Configure two channels pointing at a closed port (port 1) and a third pointing at a working model.
- Send a client request to `glm-5.2`.
- Confirm `audit_logs` has exactly one new row, `routes` is a 3-element JSONB array, top-level fields reflect the final attempt.

**Frontend**
- `web/src/pages/Logs.test.tsx` (or equivalent): test the new column renders attempt count and the click-to-expand modal lists each route with the right fields.
- Manual eyeball: open `/admin/logs`, click a row with multiple routes, confirm the modal is readable.

## Migration

```sql
-- 20260701000000_audit_routes.sql
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS routes JSONB;
```

- Idempotent (`IF NOT EXISTS`)
- Nullable — no backfill, no default
- Postgres treats this as a metadata-only change (no row rewrite)
- Old per-attempt rows get `routes = NULL`. New rows always populate it.

## Risk

- **Schema migration**: `ALTER TABLE ADD COLUMN` is metadata-only, fast even on large tables.
- **Behavior change**: `SELECT COUNT(*) FROM audit_logs` no longer equals "upstream attempts" — it now equals "client requests". Anyone with custom SQL hitting the audit table needs to know.
- **Audit worker deployment order**: deploy the new worker before the new gateway. The new gateway emits events with the new field; the old worker would deserialize-fail on the new field. (Mitigation: the worker uses `serde(deserialize_with = "...")`-style tolerant deserialization, or we add a default — implementation detail in the plan.)

## Rollback

- Revert the code commits, redeploy old gateway + old worker
- The `routes` column stays in the schema (additive, harmless); new rows simply won't populate it
- No data loss either direction
- The "old worker reads new event" deserialization issue goes away as soon as the old worker is back

## Commit Strategy

One feature branch `feature/audit-routes`, merged to develop, released as `1.8.0` (minor bump — the audit table semantics change is a backwards-incompatible query-result change for any custom SQL, even though the schema change itself is additive). Multiple commits per layer (types, migration, storage, nats, proxy, audit-worker, audit-lib, frontend, tests).

## Open Questions

None at design time.
