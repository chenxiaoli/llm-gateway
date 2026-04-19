# Audit Log Model Override Fields Design

**Date**: 2026-04-17
**Status**: Approved

## Summary

Add three new fields to audit logs to track model override scenarios: `original_model`, `upstream_model`, and `model_override_reason`. This enables debugging when the proxy changes the requested model to a different upstream model.

## Background

When proxying LLM requests, the gateway may replace the client-requested model with a different upstream model due to:
1. **Channel Mapping**: Admin-configured `channel_models` table maps request model → upstream model
2. **Model Equivalence Fallback**: User-configured equivalence groups where available models are used as fallback

Currently, audit logs only record the model field, making it impossible to distinguish what the client requested vs what was actually sent upstream.

## Design

### Field Changes

```rust
// crates/storage/src/types.rs - AuditLog struct
pub struct AuditLog {
    // ... existing fields ...
    pub original_model: Option<String>,      // Client requested model
    pub upstream_model: Option<String>,      // Actually sent to upstream
    pub model_override_reason: Option<String>, // Override reason: "channel_mapping" | "model_equivalence_fallback"
}
```

### Field Semantics

| Field | When Populated | Example |
|-------|----------------|---------|
| `original_model` | Always when model is overridden | `"gpt-4o"` |
| `upstream_model` | Always when model is overridden | `"gpt-4o-mini"` |
| `model_override_reason` | When override occurs | `"channel_mapping"` |

When no override occurs:
- All three fields are `None`

### Reason Values

| Value | Trigger |
|-------|---------|
| `channel_mapping` | `upstream_model_name` from `channel_models` differs from request |
| `model_equivalence_fallback` | User's equivalence group model was not available, fell back |

## Implementation

### Files to Modify

1. **crates/storage/src/types.rs**
   - Add 3 new fields to `AuditLog` struct

2. **crates/storage/src/sqlite.rs**
   - Add migration for new columns
   - Update `insert_log` and query methods

3. **crates/storage/src/postgres.rs**
   - Add migration for new columns
   - Update `insert_log` and query methods

4. **crates/audit/src/lib.rs**
   - Update `log_request` method signature to accept new fields

5. **crates/api/src/proxy.rs**
   - Capture request model before any modification
   - Pass original + upstream model to audit logger when they differ
   - Pass override reason based on which logic triggered the change

### Data Flow

```
Client Request: model="gpt-4o"
       ↓
proxy.rs: lookup channel_models
       ↓
upstream_model = "gpt-4o-mini" (differs from request)
       ↓
AuditLog:
  - model_name: "gpt-4o-mini" (existing field)
  - original_model: "gpt-4o"
  - upstream_model: "gpt-4o-mini"
  - model_override_reason: "channel_mapping"
```

## Backward Compatibility

- New fields are `Option<String>` - existing logs will have `None`
- Database migrations use `ALTER TABLE ADD COLUMN` with nullable columns
- No breaking changes to API responses (fields omitted when `None`)

## Testing

- Unit test: Verify fields populated when channel mapping triggers
- Unit test: Verify fields are `None` when no override occurs
- Integration test: End-to-end request with model override records correct values