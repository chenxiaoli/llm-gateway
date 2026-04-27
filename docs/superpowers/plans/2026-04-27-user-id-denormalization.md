# user_id Denormalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `user_id` column to `usage_records` and `audit_logs` tables, populate it at insert time, and use it for direct user-scoped queries instead of joining through `api_keys.created_by`.

**Architecture:** Denormalize `user_id` from `api_keys.created_by` into the two high-traffic tables. Add it to the `AuditTask` write path so every new record gets `user_id` at insert time. Replace the `key_ids IN (...)` workaround with a simple `WHERE user_id = ?` filter. Replace the settlement worker's N+1 loop with a grouped query.

**Tech Stack:** Rust (Axum, sqlx), SQLite + PostgreSQL, React/TypeScript frontend (no changes needed)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/storage/migrations/sqlite/20260428000000_add_user_id.sql` | SQLite migration: add column + backfill |
| `crates/storage/migrations/postgres/20260428000000_add_user_id.sql` | PostgreSQL migration: add column + backfill |
| `crates/storage/src/types.rs` | Add `user_id` to `UsageRecord`, `AuditLog`, `AuditLogSummary`, `UsageFilter`, `LogFilter`; remove `key_ids` from `UsageFilter` |
| `crates/storage/src/sqlite.rs` | Update row structs, INSERT/WHERE for `user_id`, remove `key_ids` IN logic |
| `crates/storage/src/postgres.rs` | Same as sqlite.rs for PostgreSQL |
| `crates/storage/src/lib.rs` | Add `query_usage_cost_by_user` to Storage trait |
| `crates/api/src/lib.rs` | Add `user_id` to `AuditTask` |
| `crates/api/src/proxy.rs` | Set `user_id` in `AuditTask` + `SseAuditParams` construction |
| `crates/api/src/workers.rs` | Pass `user_id` to `UsageRecord` and `AuditLogger` |
| `crates/audit/src/lib.rs` | Add `user_id` param to `log_request`, set in `AuditLog` |
| `crates/api/src/management/usage.rs` | Use `filter.user_id` instead of key lookup |
| `crates/api/src/management/logs.rs` | Add `user_id` filter for non-admin log queries |
| `crates/api/src/settlement.rs` | Replace N+1 with grouped query |

---

### Task 1: Migration files

**Files:**
- Create: `crates/storage/migrations/sqlite/20260428000000_add_user_id.sql`
- Create: `crates/storage/migrations/postgres/20260428000000_add_user_id.sql`

- [ ] **Step 1: Create SQLite migration**

```sql
-- crates/storage/migrations/sqlite/20260428000000_add_user_id.sql
ALTER TABLE usage_records ADD COLUMN user_id TEXT;
ALTER TABLE audit_logs ADD COLUMN user_id TEXT;

UPDATE usage_records SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = usage_records.key_id
);
UPDATE audit_logs SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = audit_logs.key_id
);
```

- [ ] **Step 2: Create PostgreSQL migration**

```sql
-- crates/storage/migrations/postgres/20260428000000_add_user_id.sql
ALTER TABLE usage_records ADD COLUMN user_id TEXT;
ALTER TABLE audit_logs ADD COLUMN user_id TEXT;

UPDATE usage_records SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = usage_records.key_id
);
UPDATE audit_logs SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = audit_logs.key_id
);
```

- [ ] **Step 3: Verify migrations compile**

Run: `cd /workspace/llm-gateway && cargo check 2>&1 | tail -5`
Expected: compiles (migrations are just SQL files, no Rust change yet)

- [ ] **Step 4: Commit**

```bash
git add crates/storage/migrations/sqlite/20260428000000_add_user_id.sql
git add crates/storage/migrations/postgres/20260428000000_add_user_id.sql
git commit -m "feat: add user_id column migration for usage_records and audit_logs"
```

---

### Task 2: Add user_id to Rust types and remove key_ids from UsageFilter

**Files:**
- Modify: `crates/storage/src/types.rs` (lines 420-433, 462-470, 476-499, 501-522, 524-536)

- [ ] **Step 1: Add `user_id: Option<String>` to `UsageRecord` (line ~433)**

Add after the `cost` field, before `created_at`:

```rust
pub user_id: Option<String>,
```

The full struct becomes:
```rust
pub struct UsageRecord {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: Protocol,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cost: f64,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Replace `key_ids` with `user_id` in `UsageFilter` (lines 462-470)**

Replace the `key_ids` field we added earlier with `user_id`:

```rust
#[derive(Debug, Deserialize)]
pub struct UsageFilter {
    pub key_id: Option<String>,
    #[serde(skip)]
    pub user_id: Option<String>,
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub since: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub until: Option<DateTime<Utc>>,
}
```

Remove the `key_ids: Option<Vec<String>>` field entirely.

- [ ] **Step 3: Add `user_id: Option<String>` to `AuditLog` (line ~499)**

Add after the last field:

```rust
pub user_id: Option<String>,
```

- [ ] **Step 4: Add `user_id: Option<String>` to `AuditLogSummary` (line ~522)**

Add after the last field:

```rust
pub user_id: Option<String>,
```

- [ ] **Step 5: Add `user_id` filter to `LogFilter` (lines 524-536)**

Add after `key_id`:

```rust
#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub key_id: Option<String>,
    #[serde(skip)]
    pub user_id: Option<String>,
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub since: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub until: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub offset: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub limit: Option<i64>,
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check 2>&1 | tail -20`
Expected: Many errors from storage implementations (row structs, INSERTs, etc.) — that's fine, we fix those next.

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: add user_id to UsageRecord, AuditLog, AuditLogSummary, UsageFilter, LogFilter"
```

---

### Task 3: Update SQLite storage implementation

**Files:**
- Modify: `crates/storage/src/sqlite.rs` (row structs ~157-289, INSERTs ~1032-1051/1201-1232, WHERE clauses ~1053-1185, SELECTs ~1278-1339)

This is the largest task. All changes are in one file.

- [ ] **Step 1: Add `user_id` to `SqliteUsageRow` (line ~157)**

Find `struct SqliteUsageRow` and add:
```rust
pub user_id: Option<String>,
```

- [ ] **Step 2: Update `From<SqliteUsageRow> for UsageRecord` impl**

Add mapping:
```rust
user_id: row.user_id,
```

- [ ] **Step 3: Add `user_id` to `SqliteAuditSummaryRow` (line ~190)**

Find the struct and add:
```rust
pub user_id: Option<String>,
```

- [ ] **Step 4: Update `From<SqliteAuditSummaryRow> for AuditLogSummary` impl**

Add mapping:
```rust
user_id: row.user_id,
```

- [ ] **Step 5: Add `user_id` to `SqliteAuditRow` (line ~239)**

Find the struct and add:
```rust
pub user_id: Option<String>,
```

- [ ] **Step 6: Update `From<SqliteAuditRow> for AuditLog` impl**

Add mapping:
```rust
user_id: row.user_id,
```

- [ ] **Step 7: Update `record_usage` INSERT (line ~1032)**

Change SQL from:
```sql
INSERT INTO usage_records (id, key_id, model_name, provider_id, channel_id, protocol,
  input_tokens, output_tokens, cache_read_tokens, cost, created_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
```
To:
```sql
INSERT INTO usage_records (id, key_id, model_name, provider_id, channel_id, protocol,
  input_tokens, output_tokens, cache_read_tokens, cost, user_id, created_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
```

Add bind param after cost:
```rust
query = query.bind(record.user_id.clone());
```

- [ ] **Step 8: Update `query_usage` WHERE clause (line ~1053)**

Replace the `key_ids` IN block (that we added earlier) with:
```rust
if let Some(ref user_id) = filter.user_id {
    sql.push_str(" AND user_id = ?");
    bind_vars.push(user_id.clone());
} else if let Some(ref key_id) = filter.key_id {
    sql.push_str(" AND key_id = ?");
    bind_vars.push(key_id.clone());
}
```

Remove all `key_ids` IN (...) logic.

- [ ] **Step 9: Update `query_usage_paginated` WHERE clause (line ~1094)**

Same replacement as Step 8 — replace `key_ids` block with `user_id` filter:
```rust
if let Some(ref user_id) = filter.user_id {
    where_sql.push_str(" AND user_id = ?");
    bind_vars.push(user_id.clone());
} else if let Some(ref key_id) = filter.key_id {
    where_sql.push_str(" AND key_id = ?");
    bind_vars.push(key_id.clone());
}
```

- [ ] **Step 10: Update `query_usage_summary` WHERE clause (line ~1149)**

Same replacement — replace `key_ids` block with `user_id` filter.

- [ ] **Step 11: Update `insert_log` INSERT (line ~1201)**

Add `user_id` as the 22nd column and bind param:
```sql
INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, stream,
  request_body, response_body, status_code, latency_ms, input_tokens, output_tokens,
  created_at, original_model, upstream_model, model_override_reason, request_path,
  upstream_url, request_headers, response_headers, user_id)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
```

Add bind param after response_headers:
```rust
query = query.bind(log.user_id.clone());
```

- [ ] **Step 12: Update `query_logs_paginated` SELECT and WHERE (line ~1278)**

Add `user_id` to the SELECT column list. Add user_id filter support to the WHERE clause:
```rust
if let Some(ref user_id) = filter.user_id {
    where_sql.push_str(" AND user_id = ?");
    bind_vars.push(user_id.clone());
}
```

- [ ] **Step 13: Update `get_log` SELECT (line ~1329)**

Add `user_id` to the SELECT column list:
```sql
SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream,
  request_body, response_body, status_code, latency_ms, input_tokens, output_tokens,
  created_at, original_model, upstream_model, model_override_reason, request_path,
  upstream_url, request_headers, response_headers, user_id
FROM audit_logs WHERE id = ?
```

- [ ] **Step 14: Verify compilation**

Run: `cargo check 2>&1 | tail -20`
Expected: Fewer errors than before — only PostgreSQL storage and API crates should still fail.

- [ ] **Step 15: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat: update SQLite storage for user_id column"
```

---

### Task 4: Update PostgreSQL storage implementation

**Files:**
- Modify: `crates/storage/src/postgres.rs` (row structs ~147-302, INSERTs ~1075-1094/1228-1258, WHERE clauses ~1096-1185, SELECTs ~1295-1339)

Same changes as Task 3 but for PostgreSQL. Uses `$N` placeholders instead of `?`.

- [ ] **Step 1: Add `user_id` to `PgUsageRow` (line ~147)**

```rust
pub user_id: Option<String>,
```

- [ ] **Step 2: Update `From<PgUsageRow> for UsageRecord` impl**

Add mapping: `user_id: row.user_id,`

- [ ] **Step 3: Add `user_id` to `PgAuditSummaryRow` (line ~202)**

```rust
pub user_id: Option<String>,
```

- [ ] **Step 4: Update `From<PgAuditSummaryRow> for AuditLogSummary` impl**

Add mapping: `user_id: row.user_id,`

- [ ] **Step 5: Add `user_id` to `PgAuditRow` (line ~252)**

```rust
pub user_id: Option<String>,
```

- [ ] **Step 6: Update `From<PgAuditRow> for AuditLog` impl**

Add mapping: `user_id: row.user_id,`

- [ ] **Step 7: Update `record_usage` INSERT (line ~1075)**

Change to 12 columns with `$12` for user_id:
```sql
INSERT INTO usage_records (id, key_id, model_name, provider_id, channel_id, protocol,
  input_tokens, output_tokens, cache_read_tokens, cost, user_id, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
```

Renumber: `created_at` moves from `$11` to `$12`, `user_id` is `$11`. Update bind params accordingly.

- [ ] **Step 8: Update `query_usage_paginated` WHERE clause (line ~1107)**

Replace the `key_ids` IN block with `user_id` filter:
```rust
if let Some(ref user_id) = filter.user_id {
    conditions.push(format!("user_id = ${}", bind_vals.len() + 1));
    bind_vals.push(user_id.clone());
} else if let Some(ref key_id) = filter.key_id {
    conditions.push(format!("key_id = ${}", bind_vals.len() + 1));
    bind_vals.push(key_id.clone());
}
```

- [ ] **Step 9: Update `query_usage_summary` WHERE clause (line ~1170)**

Same replacement as Step 8.

- [ ] **Step 10: Update `insert_log` INSERT (line ~1228)**

Add `user_id` as 22nd column with `$22`:
```sql
INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, stream,
  request_body, response_body, status_code, latency_ms, input_tokens, output_tokens,
  created_at, original_model, upstream_model, model_override_reason, request_path,
  upstream_url, request_headers, response_headers, user_id)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
```

Add bind param after response_headers.

- [ ] **Step 11: Update `query_logs_paginated` SELECT and WHERE (line ~1295)**

Add `user_id` to SELECT column list. Add user_id filter to WHERE clause:
```rust
if let Some(ref user_id) = filter.user_id {
    conditions.push(format!("user_id = ${}", bind_vals.len() + 1));
    bind_vals.push(user_id.clone());
}
```

Note: The PostgreSQL `query_logs_paginated` currently ignores filters. This is a pre-existing bug — just add user_id to the SELECT columns and filter support alongside the existing code structure.

- [ ] **Step 12: Update `get_log` SELECT (line ~1318)**

Add `user_id` to SELECT:
```sql
SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream,
  request_body, response_body, status_code, latency_ms, input_tokens, output_tokens,
  created_at, original_model, upstream_model, model_override_reason, request_path,
  upstream_url, request_headers, response_headers, user_id
FROM audit_logs WHERE id = $1
```

- [ ] **Step 13: Verify compilation**

Run: `cargo check 2>&1 | tail -20`
Expected: Only API crate errors remain (AuditTask, proxy, workers, etc.)

- [ ] **Step 14: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat: update PostgreSQL storage for user_id column"
```

---

### Task 5: Propagate user_id through the write path

**Files:**
- Modify: `crates/api/src/lib.rs` (lines 32-54)
- Modify: `crates/api/src/proxy.rs` (lines 287-304, 391-412, 771-792, 852-870, 940-961)
- Modify: `crates/api/src/workers.rs` (lines 123-170)
- Modify: `crates/audit/src/lib.rs` (lines 34-91)

- [ ] **Step 1: Add `user_id` to `AuditTask` in `crates/api/src/lib.rs`**

Add after `key_id`:
```rust
pub user_id: Option<String>,
```

- [ ] **Step 2: Add `user_id` to `SseAuditParams` in `crates/api/src/proxy.rs` (line ~287)**

Add after `key_id`:
```rust
pub user_id: Option<String>,
```

- [ ] **Step 3: Set `user_id` in `SseAuditParams` construction (line ~852)**

Add after `key_id: api_key.id.clone()`:
```rust
user_id: api_key.created_by.clone(),
```

- [ ] **Step 4: Set `user_id` in streaming AuditTask construction (line ~391)**

Add after `key_id: audit_params.key_id`:
```rust
user_id: audit_params.user_id,
```

- [ ] **Step 5: Set `user_id` in upstream error AuditTask construction (line ~771)**

Add after `key_id: api_key.id.clone()`:
```rust
user_id: api_key.created_by.clone(),
```

- [ ] **Step 6: Set `user_id` in non-streaming AuditTask construction (line ~940)**

Add after `key_id: api_key.id.clone()`:
```rust
user_id: api_key.created_by.clone(),
```

- [ ] **Step 7: Pass `user_id` to `UsageRecord` in workers.rs (line ~123)**

Add after `key_id: task.key_id.clone()`:
```rust
user_id: task.user_id.clone(),
```

- [ ] **Step 8: Add `user_id` parameter to `AuditLogger::log_request` signature**

In `crates/audit/src/lib.rs`, add after the `key_id: &str` parameter:
```rust
user_id: Option<&str>,
```

- [ ] **Step 9: Set `user_id` in `AuditLog` construction inside `log_request`**

Add after `key_id: key_id.to_string()`:
```rust
user_id: user_id.map(String::from),
```

- [ ] **Step 10: Update the `log_request` call in `workers.rs` (line ~151)**

Add `task.user_id.as_deref()` as the second argument to `audit_logger.log_request(...)`.

- [ ] **Step 11: Verify compilation**

Run: `cargo check 2>&1 | tail -10`
Expected: Clean compile (only pre-existing warnings)

- [ ] **Step 12: Commit**

```bash
git add crates/api/src/lib.rs crates/api/src/proxy.rs crates/api/src/workers.rs crates/audit/src/lib.rs
git commit -m "feat: propagate user_id through write path (AuditTask → UsageRecord/AuditLog)"
```

---

### Task 6: Simplify query handlers

**Files:**
- Modify: `crates/api/src/management/usage.rs`
- Modify: `crates/api/src/management/logs.rs`

- [ ] **Step 1: Simplify `usage.rs` handlers**

Replace the entire file content with:

```rust
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_storage::{PaginatedResponse, PaginationParams, UsageFilter, UsageRecord, UsageSummaryRecord};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    #[serde(flatten)]
    pub filter: UsageFilter,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<UsageQuery>,
) -> Result<Json<PaginatedResponse<UsageRecord>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let (page, page_size) = query.pagination.normalized();
    let mut filter = query.filter;

    if claims.role != "admin" {
        filter.user_id = Some(claims.sub);
    }

    let result = state
        .storage
        .query_usage_paginated(&filter, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct UsageSummaryQuery {
    #[serde(flatten)]
    pub filter: UsageFilter,
}

pub async fn get_usage_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<UsageSummaryQuery>,
) -> Result<Json<Vec<UsageSummaryRecord>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let mut filter = query.filter;

    if claims.role != "admin" {
        filter.user_id = Some(claims.sub);
    }

    let records = state
        .storage
        .query_usage_summary(&filter)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(records))
}
```

This removes `get_user_key_ids()`, `apply_user_scope()`, and the `list_keys` import. Replaced with a single `filter.user_id = Some(claims.sub)`.

- [ ] **Step 2: Add user scoping to audit log handlers in `logs.rs`**

The `get_logs` handler currently uses `require_admin` — only admins can see logs. Change it to use `require_auth` and scope for non-admins:

Replace the import:
```rust
use crate::extractors::require_admin;
```
With:
```rust
use crate::extractors::require_auth;
```

In `get_logs`, replace:
```rust
let _claims = require_admin(&headers, &state.jwt_secret)?;
```
With:
```rust
let claims = require_auth(&headers, &state.jwt_secret)?;

let mut filter = query.filter;
if claims.role != "admin" {
    filter.user_id = Some(claims.sub);
}
```

Note: `get_log` (single log detail) should stay admin-only since it exposes full request/response bodies. Leave it with `require_admin`.

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/usage.rs crates/api/src/management/logs.rs
git commit -m "feat: simplify query handlers to use user_id filter"
```

---

### Task 7: Fix settlement worker N+1

**Files:**
- Modify: `crates/api/src/settlement.rs` (lines 53-86)

- [ ] **Step 1: Add `query_usage_cost_by_user` to Storage trait**

In `crates/storage/src/lib.rs`, add to the `Storage` trait:

```rust
async fn query_usage_cost_by_user(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<(String, f64)>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 2: Implement in SQLite (`sqlite.rs`)**

```rust
async fn query_usage_cost_by_user(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<(String, f64)>, Box<dyn std::error::Error + Send + Sync>> {
    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT user_id, SUM(cost) FROM usage_records \
         WHERE user_id IS NOT NULL AND created_at >= ? AND created_at < ? \
         GROUP BY user_id"
    )
    .bind(since.to_rfc3339())
    .bind(until.to_rfc3339())
    .fetch_all(&self.pool)
    .await?;
    Ok(rows)
}
```

- [ ] **Step 3: Implement in PostgreSQL (`postgres.rs`)**

```rust
async fn query_usage_cost_by_user(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<(String, f64)>, Box<dyn std::error::Error + Send + Sync>> {
    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT user_id, SUM(cost) FROM usage_records \
         WHERE user_id IS NOT NULL AND created_at >= $1 AND created_at < $2 \
         GROUP BY user_id"
    )
    .bind(since.to_rfc3339())
    .bind(until.to_rfc3339())
    .fetch_all(&self.pool)
    .await?;
    Ok(rows)
}
```

- [ ] **Step 4: Simplify settlement worker loop in `settlement.rs`**

Replace the N+1 loop (lines ~53-86):

```rust
let records = storage
    .query_usage(&llm_gateway_storage::UsageFilter {
        key_id: None,
        user_id: None,
        model_name: None,
        since: Some(last_time),
        until: Some(now),
    })
    .await
    .unwrap_or_default();

let mut account_charges: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
for record in &records {
    if let Some(key) = storage.get_key(&record.key_id).await.unwrap_or(None) {
        if let Some(ref user_id) = key.created_by {
            if let Some(account) = storage
                .get_account_by_user_id(user_id)
                .await
                .unwrap_or(None)
            {
                *account_charges.entry(account.id.clone()).or_insert(0.0) += record.cost;
            }
        }
    }
}
```

With:

```rust
let user_costs = storage.query_usage_cost_by_user(last_time, now).await.unwrap_or_default();

let mut account_charges: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
for (user_id, cost) in &user_costs {
    if let Some(account) = storage.get_account_by_user_id(user_id).await.unwrap_or(None) {
        *account_charges.entry(account.id.clone()).or_insert(0.0) += cost;
    }
}
```

This replaces N+1 queries (one per record) with 1 grouped query + 1 query per user (typically a small number).

- [ ] **Step 5: Remove unused import**

The `query_usage` and `UsageFilter` are no longer used in `settlement.rs`. Remove them from the imports if they were imported.

- [ ] **Step 6: Verify compilation**

Run: `cargo check 2>&1 | tail -10`

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/sqlite.rs crates/storage/src/postgres.rs crates/api/src/settlement.rs
git commit -m "feat: replace settlement N+1 with grouped user cost query"
```

---

### Task 8: Verify and test

**Files:**
- All modified files

- [ ] **Step 1: Run full Rust compilation**

Run: `cd /workspace/llm-gateway && cargo build 2>&1 | tail -10`
Expected: Clean build with only pre-existing warnings.

- [ ] **Step 2: Run Rust tests**

Run: `cargo test 2>&1 | tail -20`
Expected: All tests pass.

- [ ] **Step 3: Run frontend tests**

Run: `cd /workspace/llm-gateway/web && export PATH="/root/.nvm/versions/node/v20.20.2/bin:$PATH" && npm test 2>&1 | tail -10`
Expected: All 47 tests pass.

- [ ] **Step 4: Run frontend build**

Run: `cd /workspace/llm-gateway/web && export PATH="/root/.nvm/versions/node/v20.20.2/bin:$PATH" && npm run build 2>&1 | tail -10`
Expected: Clean build.

- [ ] **Step 5: Start backend and verify migration runs**

Run: `cd /workspace/llm-gateway && cargo run 2>&1 &`
Check logs for migration execution (the ALTER TABLE + UPDATE statements).

- [ ] **Step 6: Verify user_id column exists**

```bash
sqlite3 data/gateway.db ".schema usage_records" | grep user_id
sqlite3 data/gateway.db ".schema audit_logs" | grep user_id
```

Expected: Both show `user_id TEXT`.

- [ ] **Step 7: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix: any compilation or test issues from user_id denormalization"
```
