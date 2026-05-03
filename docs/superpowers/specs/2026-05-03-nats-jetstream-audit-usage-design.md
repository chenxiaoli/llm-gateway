# NATS JetStream for Usage & Audit — Design Spec

**Date:** 2026-05-03
**Status:** Approved

## Goal

Replace the in-process `mpsc::channel(100)` for audit/usage with NATS JetStream. Provides durable event persistence (no data loss under load), survives gateway restarts, and enables external consumers (billing service, analytics pipeline, compliance archiver).

## Architecture

```
Gateway (proxy.rs)
  │
  ├─ parses response → publishes UsageEvent  → NATS "gateway.usage"
  │                                AuditEvent → NATS "gateway.audit"
  │
  ├─ NATS JetStream (durable, replay)
  │   ├─ Stream "GATEWAY_USAGE"  (7d / 1M msgs)
  │   │   └─ Consumer: usage-worker → record_usage()
  │   │   └─ Consumer: (future) billing-service
  │   │
  │   └─ Stream "GATEWAY_AUDIT"  (30d / 5M msgs)
  │       └─ Consumer: audit-worker → insert_log()
  │       └─ Consumer: (future) compliance-archiver
```

## Data Flow

### Proxy response handling (proxy.rs)

1. Parse token counts from response bytes (move `parse_usage` from workers.rs)
2. Calculate cost (move `PricingCalculator` call from workers.rs)
3. Publish `UsageEvent` to `GATEWAY_USAGE` stream
4. Publish `AuditEvent` to `GATEWAY_AUDIT` stream
5. Both publishes are fire-and-forget (JetStream confirms persistence)

### Usage consumer (in-process)

- Push consumer on `GATEWAY_USAGE`
- Calls `storage.record_usage()` — thin DB write, no parsing needed

### Audit consumer (in-process)

- Push consumer on `GATEWAY_AUDIT`
- Reads `audit_log_request` / `audit_log_response` settings (once on startup, cached)
- Optionally redacts bodies, calls `storage.insert_log()`

## Event Types

### UsageEvent — small, structured, pre-computed

```json
{
  "id": "uuid",
  "key_id": "...",
  "user_id": "...",
  "model_name": "...",
  "provider_id": "...",
  "channel_id": "...",
  "protocol": "openai",
  "input_tokens": 150,
  "output_tokens": 42,
  "cache_read_tokens": 0,
  "cache_creation_tokens": 0,
  "cost": 1200,
  "latency_ms": 340,
  "created_at": "2026-05-03T12:00:00Z"
}
```

### AuditEvent — heavy, raw payload

```json
{
  "id": "uuid",
  "key_id": "...",
  "user_id": "...",
  "model_name": "...",
  "provider_id": "...",
  "channel_id": "...",
  "protocol": "openai",
  "status_code": 200,
  "latency_ms": 340,
  "original_model": "...",
  "upstream_model": "...",
  "request_path": "/v1/chat/completions",
  "upstream_url": "https://...",
  "request_body": "{...}",
  "response_body": "{...}",
  "request_headers": "{...}",
  "response_headers": "{...}",
  "created_at": "2026-05-03T12:00:00Z"
}
```

## Configuration

```toml
[nats]
url = "nats://localhost:4222"
```

When `[nats]` is present → JetStream mode. When absent → current mpsc behavior (backward compatible, no NATS dependency).

## New Crate

`crates/nats-publisher/` — thin wrapper over `async-nats`:
- Connect to NATS, create streams (idempotent on startup)
- `publish_usage(event)` / `publish_audit(event)` methods
- Owned `async_nats::client` stored in `AppState`

## Files Changed

| File | Action |
|---|---|
| `Cargo.toml` | Add `nats-publisher` member, `async-nats` dep |
| `crates/nats-publisher/src/lib.rs` | Create — NATS client, event types, publish methods |
| `crates/api/src/lib.rs` | Add NATS client to `AppState`, remove mpsc channel |
| `crates/api/src/proxy.rs` | Parse usage + publish to NATS instead of try_send |
| `crates/api/src/workers.rs` | Replace mpsc consumer with NATS push consumers |
| `crates/gateway/src/main.rs` | Init NATS client, start consumers |
| `config.toml` | Add `[nats]` section |

## Not Changed

- DB schema, storage trait, billing crate
- Frontend — zero changes
- Audit logger crate (kept for settings logic)

## Stream Retention

| Stream | Retention | Max Messages | Purpose |
|---|---|---|---|
| `GATEWAY_USAGE` | 7 days | 1M | Billing, aggregation |
| `GATEWAY_AUDIT` | 30 days | 5M | Compliance, debugging |

## Scope

- Gateway publishes to NATS, in-process consumers write to DB
- External consumers attach independently (out of scope for this change)
- No schema changes, no frontend changes
- Backward compatible — works without NATS config
