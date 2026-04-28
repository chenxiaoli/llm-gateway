# API Key Model Fallback Design

## Goal

Allow API keys to configure model fallback chains. When a requested model fails (unavailable, upstream error, or client error), the gateway retries with alternative models in priority order.

## Data Model

### New table: `model_fallbacks`

```sql
CREATE TABLE model_fallbacks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,
    created_by TEXT,
    created_at TEXT NOT NULL
);
```

`config` stores a JSON array of fallback groups:

```json
[
  {"models": ["gpt-4.4", "glm-5.1", "glm-5"], "priorities": [1, 2, 3]},
  {"models": ["claude-sonnet-4", "deepseek-v3"], "priorities": [1, 2]}
]
```

Each group defines equivalent models with priorities. Lower number = higher priority. Multiple keys can reference the same `model_fallbacks` row (reusability).

### Add to `api_keys`

```sql
ALTER TABLE api_keys ADD COLUMN model_fallback_id TEXT REFERENCES model_fallbacks(id);
```

Nullable — keys without fallback configuration work as before.

## Routing Logic

When a request arrives for model X through a key with `model_fallback_id`:

1. Route the request normally (resolve channels, send upstream)
2. If the request fails for **any reason** (model not found, no channels, upstream 5xx, upstream 4xx, connection error):
   a. Load `model_fallbacks.config` for the key
   b. Find a group containing model X
   c. Sort remaining models in the group by priority (ascending)
   d. For each fallback model, re-run the full routing cycle (resolve channels, send upstream)
   e. Return the first successful response
3. If all fallbacks also fail, return the **last error** to the client
4. Audit log captures `original_model`, `upstream_model`, and `model_override_reason: "model_fallback"` when a fallback is used

### Matching logic

- Model name matching is case-insensitive (consistent with existing proxy behavior)
- A model appearing in its own fallback group is skipped during fallback (only *other* models in the group are tried)

## Rust Types

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelFallbackGroup {
    pub models: Vec<String>,
    pub priorities: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelFallbackConfig {
    pub id: String,
    pub name: String,
    pub config: Vec<ModelFallbackGroup>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

## Storage Trait Additions

- `create_model_fallback(config: &ModelFallbackConfig) -> Result<ModelFallbackConfig>`
- `update_model_fallback(config: &ModelFallbackConfig) -> Result<ModelFallbackConfig>`
- `delete_model_fallback(id: &str) -> Result<()>`
- `get_model_fallback(id: &str) -> Result<Option<ModelFallbackConfig>>`
- `list_model_fallbacks() -> Result<Vec<ModelFallbackConfig>>`

## Management API

- `POST /api/v1/admin/model-fallbacks` — create fallback config
- `GET /api/v1/admin/model-fallbacks` — list all
- `GET /api/v1/admin/model-fallbacks/{id}` — get one
- `PUT /api/v1/admin/model-fallbacks/{id}` — update
- `DELETE /api/v1/admin/model-fallbacks/{id}` — delete

API key create/update accepts `model_fallback_id` field.

## Frontend

- New **Model Fallbacks** admin page to create/edit/delete fallback configs
- API key form gets a **Model Fallback** dropdown (select from existing configs or None)
- No changes to customer-facing pages

## Files Modified

| File | Change |
|------|--------|
| `crates/storage/migrations/sqlite/YYYY_add_model_fallbacks.sql` | New migration |
| `crates/storage/migrations/postgres/YYYY_add_model_fallbacks.sql` | New migration |
| `crates/storage/src/types.rs` | Add ModelFallbackGroup, ModelFallbackConfig structs; add model_fallback_id to ApiKey |
| `crates/storage/src/sqlite.rs` | Row struct, CRUD, key update |
| `crates/storage/src/postgres.rs` | Same |
| `crates/storage/src/lib.rs` | Add trait methods |
| `crates/api/src/proxy.rs` | Fallback retry loop after initial routing failure |
| `crates/api/src/management/mod.rs` | Register routes |
| `crates/api/src/management/model_fallbacks.rs` | New CRUD handlers |
| `crates/api/src/management/keys.rs` | Accept model_fallback_id on create/update |
| `web/src/api/` | API client functions |
| `web/src/hooks/` | React Query hooks |
| `web/src/pages/` | Model Fallbacks page, key form update |
| `web/src/components/Layout.tsx` | Add nav item |
