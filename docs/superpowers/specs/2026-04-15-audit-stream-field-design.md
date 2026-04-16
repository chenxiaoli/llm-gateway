# Audit Log Stream Field Design

> **Date:** 2026-04-15
> **Feature:** Add stream field to audit logs

## Goal

Add a `stream: bool` field to audit logs to distinguish between streaming and non-streaming requests. This enables:
- Query and filter streaming vs non-streaming requests
- Analyze streaming request patterns
- Debug streaming-related issues

## Current State

Current `AuditLog` struct:
```rust
pub struct AuditLog {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: Protocol,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
}
```

## Design

### 1. Add `stream` field to AuditLog

```rust
pub struct AuditLog {
    // ... existing fields ...
    pub stream: bool,        // NEW: true for streaming, false for non-streaming
    pub created_at: DateTime<Utc>,
}
```

### 2. Update AuditLogger.log_request

```rust
pub async fn log_request(
    &self,
    key_id: &str,
    model_name: &str,
    provider_id: &str,
    protocol: Protocol,
    request_body: &str,
    response_body: &str,
    status_code: i32,
    latency_ms: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    stream: bool,  // NEW parameter
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ...
}
```

### 3. API Handlers Updates

**openai.rs:**
- `chat_completions`: detect `stream` from request body, pass to `log_request`
- `record_stream_usage`: add `stream` parameter, always `true` for streaming

**anthropic.rs:**
- Same changes as openai.rs

### 4. Database Migration

**SQLite:** `crates/storage/migrations/YYYYMMDDNNNNN_add_audit_stream.sql`
```sql
ALTER TABLE audit_logs ADD COLUMN stream INTEGER NOT NULL DEFAULT 0;
```

**PostgreSQL:** `crates/storage/migrations/postgres/YYYYMMDDNNNNN_add_audit_stream.sql`
```sql
ALTER TABLE audit_logs ADD COLUMN stream BOOLEAN NOT NULL DEFAULT FALSE;
```

## Implementation Scope

1. Add `stream` field to types.rs
2. Update audit crate's log_request signature
3. Update openai.rs handlers (3 call sites)
4. Update anthropic.rs handlers (3 call sites)
5. Create SQLite migration
6. Create PostgreSQL migration

## Backward Compatibility

- Default value: `false` for non-streaming
- Existing logs will have `stream=false` (default)
- No breaking changes to existing API