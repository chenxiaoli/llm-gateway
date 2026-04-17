# Audit Log Model Override Fields Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three new fields to audit logs to track model override: original_model, upstream_model, model_override_reason.

**Architecture:** Extend AuditLog struct with new optional fields, add database migrations, update storage layer queries, and wire up proxy to capture and log model override information.

**Tech Stack:** Rust, SQLite, PostgreSQL, sqlx

---

## File Structure

```
crates/storage/src/types.rs      # AuditLog struct
crates/storage/migrations/sqlite/YYYYMMDDHHMMSS_add_audit_model_override.sql
crates/storage/migrations/postgres/YYYYMMDDHHMMSS_add_audit_model_override.sql
crates/storage/src/sqlite.rs     # insert_log, query_logs, query_logs_paginated
crates/storage/src/postgres.rs   # insert_log, query_logs, query_logs_paginated
crates/audit/src/lib.rs          # log_request method signature
crates/api/src/lib.rs            # AuditTask struct
crates/api/src/proxy.rs          # Capture and pass model override info
```

---

## Task 1: Update AuditLog Struct

**Files:**
- Modify: `crates/storage/src/types.rs:354-369`
- Test: `crates/storage/src/types.rs` (implicit - compile check)

- [ ] **Step 1: Add three new fields to AuditLog struct**

```rust
// In types.rs, AuditLog struct (around line 354)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: Protocol,
    pub stream: bool,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
    // NEW FIELDS:
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
}
```

- [ ] **Step 2: Run cargo check to verify types compile**

```bash
cd /workspace && cargo check -p llm_gateway_storage
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(audit): add original_model, upstream_model, model_override_reason to AuditLog"
```

---

## Task 2: Create SQLite Migration

**Files:**
- Create: `crates/storage/migrations/sqlite/20260417000000_add_audit_model_override.sql`

- [ ] **Step 1: Create migration file**

```sql
-- SQLite migration: Add model override fields to audit_logs
ALTER TABLE audit_logs ADD COLUMN original_model TEXT;
ALTER TABLE audit_logs ADD COLUMN upstream_model TEXT;
ALTER TABLE audit_logs ADD COLUMN model_override_reason TEXT;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/sqlite/20260417000000_add_audit_model_override.sql
git commit -m "migrations(sqlite): add audit model override fields"
```

---

## Task 3: Create PostgreSQL Migration

**Files:**
- Create: `crates/storage/migrations/postgres/20260417000000_add_audit_model_override.sql`

- [ ] **Step 1: Create migration file**

```sql
-- PostgreSQL migration: Add model override fields to audit_logs
ALTER TABLE audit_logs ADD COLUMN original_model TEXT;
ALTER TABLE audit_logs ADD COLUMN upstream_model TEXT;
ALTER TABLE audit_logs ADD COLUMN model_override_reason TEXT;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/postgres/20260417000000_add_audit_model_override.sql
git commit -m "migrations(postgres): add audit model override fields"
```

---

## Task 4: Update SQLite Storage Layer

**Files:**
- Modify: `crates/storage/src/sqlite.rs:1032-1090` (insert_log, query_logs)
- Test: `crates/storage/src/sqlite.rs` (implicit - cargo check)

- [ ] **Step 1: Update insert_log to include new fields**

```rust
// In sqlite.rs, insert_log method (line ~1032)
async fn insert_log(&self, log: &AuditLog) -> Result<(), DbErr> {
    sqlx::query(
        "INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
         status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&log.id)
    .bind(&log.key_id)
    .bind(&log.model_name)
    .bind(&log.provider_id)
    .bind(&log.channel_id)
    .bind(protocol_str(&log.protocol))
    .bind(log.stream)
    .bind(&log.request_body)
    .bind(&log.response_body)
    .bind(log.status_code)
    .bind(log.latency_ms)
    .bind(log.input_tokens)
    .bind(log.output_tokens)
    .bind(log.created_at.to_rfc3339())
    // NEW:
    .bind(&log.original_model)
    .bind(&log.upstream_model)
    .bind(&log.model_override_reason)
    .execute(&self.pool)
    .await?;
    Ok(())
}
```

- [ ] **Step 2: Update query_logs SELECT to include new fields**

```rust
// In sqlite.rs, query_logs method (line ~1057)
let mut sql = String::from(
    "SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
     status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason
     FROM audit_logs WHERE 1=1",
);
```

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check -p llm_gateway_storage
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat(storage): update sqlite to support audit model override fields"
```

---

## Task 5: Update PostgreSQL Storage Layer

**Files:**
- Modify: `crates/storage/src/postgres.rs:940-1030` (insert_log, query_logs)
- Test: `crates/storage/src/postgres.rs` (implicit - cargo check)

- [ ] **Step 1: Update insert_log to include new fields**

```rust
// In postgres.rs, insert_log method (line ~940)
async fn insert_log(&self, log: &AuditLog) -> Result<(), DbErr> {
    sqlx::query(
        "INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
         status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
    )
    .bind(&log.id)
    .bind(&log.key_id)
    .bind(&log.model_name)
    .bind(&log.provider_id)
    .bind(&log.channel_id)
    .bind(protocol_str(&log.protocol))
    .bind(log.stream)
    .bind(&log.request_body)
    .bind(&log.response_body)
    .bind(log.status_code)
    .bind(log.latency_ms)
    .bind(log.input_tokens)
    .bind(log.output_tokens)
    .bind(log.created_at)
    // NEW:
    .bind(&log.original_model)
    .bind(&log.upstream_model)
    .bind(&log.model_override_reason)
    .execute(&self.pool)
    .await?;
    Ok(())
}
```

- [ ] **Step 2: Update query_logs SELECT to include new fields**

```rust
// In postgres.rs, query_logs method (line ~965)
let mut sql = String::from(
    "SELECT id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body,
     status_code, latency_ms, input_tokens, output_tokens, created_at, original_model, upstream_model, model_override_reason
     FROM audit_logs WHERE 1=1",
);
```

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check -p llm_gateway_storage
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat(storage): update postgres to support audit model override fields"
```

---

## Task 6: Update AuditTask and AuditLogger

**Files:**
- Modify: `crates/api/src/lib.rs:26-38` (AuditTask)
- Modify: `crates/audit/src/lib.rs:34-77` (log_request)
- Test: Implicit - cargo check

- [ ] **Step 1: Update AuditTask struct**

```rust
// In api/src/lib.rs, AuditTask struct
pub struct AuditTask {
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: llm_gateway_storage::Protocol,
    pub stream: bool,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    // NEW:
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
}
```

- [ ] **Step 2: Update log_request method signature**

```rust
// In audit/src/lib.rs, log_request method
pub async fn log_request(
    &self,
    key_id: &str,
    model_name: &str,
    provider_id: &str,
    protocol: Protocol,
    stream: bool,
    request_body: &str,
    response_body: &str,
    status_code: i32,
    latency_ms: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    // NEW:
    original_model: Option<&str>,
    upstream_model: Option<&str>,
    model_override_reason: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
```

- [ ] **Step 3: Update log_request body to include new fields**

```rust
// In audit/src/lib.rs, inside log_request, create AuditLog with new fields:
let log = AuditLog {
    id: uuid::Uuid::new_v4().to_string(),
    key_id: key_id.to_string(),
    model_name: model_name.to_string(),
    provider_id: provider_id.to_string(),
    channel_id: None,
    protocol,
    stream,
    request_body: request_body.to_string(),
    response_body: response_body.to_string(),
    status_code,
    latency_ms,
    input_tokens,
    output_tokens,
    created_at: chrono::Utc::now(),
    // NEW:
    original_model: original_model.map(String::from),
    upstream_model: upstream_model.map(String::from),
    model_override_reason: model_override_reason.map(String::from),
};
```

- [ ] **Step 4: Run cargo check**

```bash
cd /workspace && cargo check --workspace
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/lib.rs crates/audit/src/lib.rs
git commit -m "feat(audit): add model override fields to AuditTask and log_request"
```

---

## Task 7: Update Proxy to Capture and Pass Model Override Info

**Files:**
- Modify: `crates/api/src/proxy.rs:200-220` (streaming) and `crates/api/src/proxy.rs:302-314` (non-streaming)
- Test: `cargo test --workspace`

- [ ] **Step 1: Update proxy.rs to capture request model before modification**

At the beginning of proxy function (around line 49-53), capture original model:

```rust
// After getting model_name from request (around line 49-53)
let original_model = model_name.clone();
```

- [ ] **Step 2: Update streaming audit task (around line 205)**

When creating AuditTask, pass new fields when upstream differs:

```rust
// In proxy.rs, around line 205-218
let model_override_reason = if upstream_name != &model_name {
    Some("channel_mapping".to_string())
} else {
    None
};

let audit_task = AuditTask {
    key_id: api_key.id.clone(),
    model_name: upstream_name.to_string(),  // Use upstream model
    provider_id: provider.id.clone(),
    protocol: proto,
    stream: true,
    request_body: body.clone(),
    response_body: "[streaming]".to_string(),
    status_code: 200,
    latency_ms,
    input_tokens: None,
    output_tokens: None,
    // NEW:
    original_model: if upstream_name != &model_name { Some(model_name) } else { None },
    upstream_model: if upstream_name != &model_name { Some(upstream_name.to_string()) } else { None },
    model_override_reason,
};
```

- [ ] **Step 3: Update non-streaming audit call (around line 302)**

```rust
// In proxy.rs, around line 302-314
let model_override_reason = if upstream_name != &model_name {
    Some("channel_mapping".to_string())
} else {
    None
};

let _ = audit_logger.log_request(
    &key_id,
    &upstream_name.to_string(),  // Use upstream model
    &provider_id,
    proto_for_audit,
    is_stream,
    &request_body,
    &response_body,
    200,
    latency_ms,
    input_tokens,
    output_tokens,
    // NEW:
    if upstream_name != &model_name { Some(&model_name) } else { None },
    if upstream_name != &model_name { Some(upstream_name.as_str()) } else { None },
    model_override_reason.as_deref(),
).await;
```

- [ ] **Step 4: Run cargo check**

```bash
cd /workspace && cargo check --workspace
```

Expected: No errors

- [ ] **Step 5: Run tests**

```bash
cd /workspace && cargo test --workspace
```

Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): capture and log model override info in audit"
```

---

## Implementation Order

1. Task 1: AuditLog struct
2. Task 2: SQLite migration
3. Task 3: PostgreSQL migration
4. Task 4: SQLite storage
5. Task 5: PostgreSQL storage
6. Task 6: AuditTask and AuditLogger
7. Task 7: Proxy

---

## Verification

After all tasks, run:
```bash
cargo test --workspace
cargo build --release
```

All tests should pass and build should succeed.