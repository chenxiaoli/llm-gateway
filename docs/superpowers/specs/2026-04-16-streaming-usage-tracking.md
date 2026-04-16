# Streaming Token Tracking Design

## Date: 2026-04-16

## Goal

Add streaming (SSE) token tracking in proxy.rs. When stream ends (receives `[DONE]`), calculate usage and record it via async task.

## Problem

Current proxy.rs returns `response_body.into_response()` which doesn't handle SSE streaming. The original anthropic.rs had streaming logic but is no longer being called after refactoring to use proxy.

## Solution

### Approach: On [DONE] in Stream

When the SSE stream receives `data: [DONE]`:
1. Extract `input_tokens` and `output_tokens` from each event
2. Calculate cost using PricingCalculator
3. Record to storage via tokio::spawn (async)
4. Log audit via audit_logger.log_request()

## Implementation

### Changes to proxy.rs

1. Add imports:
```rust
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::StreamExt;
use tracing::debug;
```

2. Handle is_stream:
- If `req_json.get("stream")` returns true → use SSE path
- Otherwise → use existing buffered response path

3. In SSE handler, on `[DONE]`:
- Parse usage from each event
- Spawn async task to record usage + audit

## Files

- `crates/api/src/proxy.rs` - add streaming logic

## Testing

- Send streaming request to `/v1/messages` with `"stream": true`
- Verify audit_logs table has entry with `stream: true`
- Verify usage_records has token counts