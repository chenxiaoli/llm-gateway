# Provider Models Catalog

## Problem

When adding a channel model, the model dropdown shows every model in the system regardless of which provider the channel belongs to. An admin on an OpenAI channel sees GLM and Anthropic models in the same flat list.

## Solution

Add a `provider_models` junction table that records which models a provider supports. The `sync-models` endpoint populates it from the upstream provider API. The "Add Channel Model" modal filters the dropdown by the channel's provider.

## Data Model

### New table: `provider_models`

| Column | Type | Constraints |
|--------|------|-------------|
| provider_id | TEXT | NOT NULL, REFERENCES providers(id) ON DELETE CASCADE |
| model_id | TEXT | NOT NULL, REFERENCES models(id) ON DELETE CASCADE |
| upstream_name | TEXT | -- name the provider expects (e.g. "gpt-4o-2024-08-06") |
| created_at | TIMESTAMPTZ | NOT NULL |

- `UNIQUE(provider_id, model_id)` — one entry per provider-model pair

## Changes

### 1. Migration

Add `provider_models` table in both SQLite and PostgreSQL migrations.

### 2. Storage layer

- Add `ProviderModel` struct to `crates/storage/src/types.rs`
- Add `upsert_provider_models(provider_id, models: Vec<ProviderModel>)` to the Storage trait
- Add `list_provider_models(provider_id) -> Vec<ProviderModel>` to the Storage trait
- Implement in both `sqlite.rs` and `postgres.rs`

### 3. sync-models endpoint

In `crates/api/src/management/models.rs`, after creating/updating model records, also upsert into `provider_models` with the model name received from the upstream API (this is the `upstream_name`). Existing entries are updated; removed models are left in place (stale entries are harmless).

### 4. New API endpoint

`GET /api/v1/admin/providers/{id}/models` — returns the provider's models from the `provider_models` table, joined with the `models` table to include model name.

Response shape:
```json
[
  {
    "model_id": "uuid",
    "model_name": "gpt-4o",
    "upstream_name": "gpt-4o-2024-08-06"
  }
]
```

### 5. Frontend

In `web/src/pages/ChannelDetail.tsx`, the "Add Model" modal:

- Replace `useAllModels()` with a new `useProviderModels(providerId)` hook that calls the new endpoint
- When a model is selected, auto-fill `upstream_model_name` from the `upstream_name` field
- Fallback: if provider has no synced models (empty list), show all models as before

### 6. Seed data

The seed function in `crates/storage/src/seed.rs` should populate `provider_models` for seed providers, mapping each provider to its known models based on `seed_providers.json`.

## Not in Scope

- Auto-adding channel models from sync (admin still adds channel models manually)
- Removing stale provider_models entries
- Provider model pricing (pricing policies handle this already)
