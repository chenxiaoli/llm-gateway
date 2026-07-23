# `model=auto` Capability-Aware Routing — Design Spec

**Date:** 2026-07-22
**Status:** Draft (awaiting user review)
**Tracks:** develop (v2.x)

## Motivation

Today every request must name a specific model. Clients that want the gateway to pick a model on their behalf have no way to express that. We want to support `model=auto` so that:

- Clients (and the org's own product surface) can stop hard-coding a model name and let the gateway choose based on what the request actually needs.
- The gateway can route around models that don't support the request's modalities (e.g. don't send an image to a text-only model).
- Admins retain explicit, per-key control over which models are eligible — symmetric with the existing `model_fallbacks` feature.

## Scope

**In scope (V1):**

- New `auto_route_configs` platform-level table + `api_keys.auto_route_id` FK.
- `model=auto` resolution path: load the key's `auto_route_config` → filter by capability → existing channel-priority+weighted routing over the resulting candidate pool.
- Two capability dimensions only: `vision` and `tools`. Detected by introspecting the request body (one extra pass during the existing body-parse step).
- Capability data on `models` via two new `BOOLEAN` columns, populated by admin manual entry on the Models page.
- New API errors: `auto_not_configured`, `auto_no_matching_model`, `auto_all_candidates_failed`, `model_name_reserved`.
- Reserve the keyword `auto` — model creation rejects the name.
- Tests (Rust integration + storage + unit for helpers).
- Frontend: Models page two new checkboxes; new Auto Routes page for CRUD; API-key form gains an auto-route selector.
- CHANGELOG entry.

**Out of scope (YAGNI for V1):**

- More capabilities (audio-in/out, reasoning, json_mode, structured-output, parallel-tools, computer-use).
- Model-level priority (admin expresses model preference via channel priority within each model).
- Dynamic latency / error-rate signals feeding back into selection (no schema today; defer to V2).
- Cheapest-first or any cost-aware tie-breaker (use channel priority+weight).
- Per-key fallback chain interaction on auto-pool exhaustion (auto hard-errors; per-key fallback only applies to non-auto requests).
- "Excluded from auto" flag (admin controls via which models are in the config's pool).
- Auto-routing when no `auto_route_id` is set on the key (hard-error — explicit opt-in).
- Admin preference list per capability set (the config's `model_names` already expresses this).
- Audit-log changes beyond what the existing `client_requested_model` + `routes[].model` columns already capture.

## Decisions

| Decision | Value | Rationale |
|---|---|---|
| Trigger | `model == "auto"` (case-insensitive) | Simple, conventional, easy for clients |
| Capability dimensions | `vision`, `tools` | Covers most routing decisions; small MVP surface |
| Capability source | Admin manual entry | Simplest V1 — admin ticks `supports_vision` / `supports_tools` checkboxes on the Models page. No upstream call, no sync job, no sticky-override logic. Can be extended with auto-discovery later if maintenance burden grows. |
| Capability detection | Implicit body introspection | Zero client change; one extra pass at the existing body-parse step |
| Pool source | Per-key `auto_route_config` (platform-level, model-name-keyed) | Symmetric with `model_fallbacks`; admin gets explicit control; no surprise routes to expensive/experimental models |
| Pool resolution | Filter by capability → existing channel priority+weight | Reuses existing machinery; no new schema column for priority |
| Missing config | 400 `auto_not_configured` | Forces explicit opt-in per key; surfaces misconfigured deployments |
| No matching model | 400 `auto_no_matching_model` | Don't fall back to a model that will fail upstream |
| All candidates failed | 502 `auto_all_candidates_failed` (no per-key fallback) | Auto and fallback are independent concerns; composition adds complexity without clear value |
| `auto` keyword | Reserved (reject at model-create) | Prevents shadowing |
| Storage pattern | New `auto_route_configs` table, mirrors `model_fallbacks` | Clean separation from fallback semantics; admin mental model stays consistent |

## Architecture

### Data model

**Migration `20260722000001_models_capabilities.sql`:**
```sql
ALTER TABLE models
  ADD COLUMN supports_vision BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN supports_tools  BOOLEAN NOT NULL DEFAULT FALSE;
```

**Migration `20260722000002_auto_route_configs.sql`:**
```sql
CREATE TABLE IF NOT EXISTS auto_route_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,            -- JSON: { "model_names": ["gpt-4o", "claude-3-5-sonnet", ...] }
    created_by TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS auto_route_id TEXT REFERENCES auto_route_configs(id);
```

Mirrors `model_fallbacks` shape exactly so the CRUD + UI patterns copy-paste.

### Layer-by-layer changes

#### Storage (Rust)

**`crates/storage/src/types.rs`:**
- `Model` struct: add `pub supports_vision: bool`, `pub supports_tools: bool`.
- New `AutoRouteConfig` struct:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct AutoRouteConfigData {
      pub model_names: Vec<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct AutoRouteConfig {
      pub id: String,
      pub name: String,
      pub config: AutoRouteConfigData,
      pub created_by: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- `ApiKey` struct: add `pub auto_route_id: Option<String>`.

**`crates/storage/src/lib.rs`:**
- New `Storage` trait methods:
  ```rust
  async fn get_auto_route_config(&self, id: &str) -> Result<Option<AutoRouteConfig>, StorageError>;
  async fn list_auto_route_configs(&self) -> Result<Vec<AutoRouteConfig>, StorageError>;
  async fn create_auto_route_config(&self, input: NewAutoRouteConfig) -> Result<AutoRouteConfig, StorageError>;
  async fn update_auto_route_config(&self, id: &str, input: UpdateAutoRouteConfig) -> Result<AutoRouteConfig, StorageError>;
  async fn delete_auto_route_config(&self, id: &str) -> Result<(), StorageError>;
  async fn list_models_with_capabilities(&self, org_id: &str, require_vision: bool, require_tools: bool, candidate_names: &[String]) -> Result<Vec<Model>, StorageError>;
  ```

**`crates/storage/src/postgres.rs`:**
- Update `Model` SELECT sites (4), INSERT (`create_model`), UPDATE (`update_model`) to include the two new columns. Same pattern as the nickname field work.
- `PgModelRow` + `From<...>` updated.
- New `PgAutoRouteConfigRow` + impls for the 5 CRUD methods.
- `list_models_with_capabilities`:
  ```sql
  SELECT m.* FROM models m
  WHERE m.owner_org_id = $1
    AND (NOT $2 OR m.supports_vision)
    AND (NOT $3 OR m.supports_tools)
    AND m.name = ANY($4::text[])
  ```
  When `require_vision = false`, the `supports_vision` filter is skipped (we don't want to exclude text-only models when vision wasn't required). Same for tools.

#### API (Rust)

**New file `crates/api/src/auto_route.rs`:**
- `CapabilitySet { vision: bool, tools: bool }` struct.
- `detect_required_capabilities(body: &serde_json::Value) -> CapabilitySet`:
  - Vision: walk `messages[]`, for each message inspect `content` (string or array). If array, look for any block with `type` in `{"image_url", "image", "input_audio"}` — but only image types trigger vision in V1. Specifically: `{"image_url", "image"}`.
  - Tools: `body["tools"]` is array and non-empty.
  - Anthropic-format `messages[].content[]` uses `type: "image"`; OpenAI uses `type: "image_url"`. Detect both.

**`crates/api/src/error.rs`:**
- `ApiError::AutoNotConfigured` → 400 `auto_not_configured`
- `ApiError::AutoNoMatchingModel { required_vision: bool, required_tools: bool }` → 400 `auto_no_matching_model` with body listing required capabilities
- `ApiError::AutoAllCandidatesFailed` → 502 `auto_all_candidates_failed`
- `ApiError::ModelNameReserved` → 400 `model_name_reserved`

**`crates/api/src/proxy.rs`:**

In `proxy_route_and_forward` (around line 1095, before the existing model lookup):

```rust
let effective_model_name: Option<String> = if model_name.eq_ignore_ascii_case("auto") {
    None  // signal auto-routing
} else {
    Some(model_name.clone())
};

if effective_model_name.is_none() {
    // Auto path.
    let key_id = &key.id;  // already resolved earlier in proxy_inner
    let config_id = key.auto_route_id.as_ref()
        .ok_or(ApiError::AutoNotConfigured)?;
    let config = state.storage.get_auto_route_config(config_id).await?
        .ok_or(ApiError::AutoNotConfigured)?;
    let required = detect_required_capabilities(&body);
    let candidates = state.storage.list_models_with_capabilities(
        org_id, required.vision, required.tools, &config.config.model_names,
    ).await?;
    if candidates.is_empty() {
        return Err(ApiError::AutoNoMatchingModel {
            required_vision: required.vision,
            required_tools: required.tools,
        });
    }
    // Build a multi-model candidate pool and feed it to apply_weighted_routing.
    // Existing single-model flow becomes a special case (one model in the pool).
    AutoResolution::Pool(candidates)
} else {
    AutoResolution::Single(model_name.clone())
};
```

The candidate-gathering step (currently `state.registry.resolve_by_model`) needs a sibling `resolve_by_pool(&[Model])` that returns `(model, channel)` pairs across all pool models. Channel filtering (enabled, available_hours, group_id, circuit-breaker) applies per usual. `apply_weighted_routing` then sorts the combined candidate list by `channel.priority` and weighted-shuffles within tier.

The failover loop is unchanged: iterate candidates, terminal on 4xx, failover on 5xx/429/conn-error. If all candidates fail → return `AutoAllCandidatesFailed` (only when in auto mode).

**`crates/api/src/management/mod.rs`:**
- Register new CRUD routes under `/api/v1/auto-route-configs`:
  ```
  GET    /api/v1/auto-route-configs           list
  POST   /api/v1/auto-route-configs           create
  GET    /api/v1/auto-route-configs/{id}      get
  PATCH  /api/v1/auto-route-configs/{id}      update
  DELETE /api/v1/auto-route-configs/{id}      delete
  ```
  Access control mirrors `model_fallbacks`: platform-admin-gated for write; non-admins see only configs they `created_by`.

**`crates/api/src/management/models.rs` (or wherever model-create lives):**
- In create/update handlers: if `name.eq_ignore_ascii_case("auto")` → `ApiError::ModelNameReserved`.

**`crates/api/src/management/api_keys.rs`:**
- API key create/update DTO gains `auto_route_id: Option<String>`. Persisted to `api_keys.auto_route_id`.

#### Frontend

**`web/src/types/index.ts`:**
- `Model` interface: add `supports_vision: boolean`, `supports_tools: boolean`.
- New `AutoRouteConfig` interface.
- `ApiKey` interface: add `auto_route_id?: string | null`.

**`web/src/api/autoRoutes.ts` (new):** CRUD client mirroring `modelFallbacks.ts`.

**`web/src/hooks/useAutoRouteConfigs.ts` (new):** React Query hooks.

**`web/src/pages/AutoRoutes.tsx` (new):** admin page for CRUD. Mirror `ModelFallbacks.tsx` layout (table + create/edit drawer). Fields: name, multi-select model names (from org's models).

**`web/src/pages/Models.tsx`:** add two checkboxes per row: "Vision" and "Tools". Pre-populated from current `supports_vision` / `supports_tools`.

**`web/src/pages/Keys.tsx` (or wherever API key form lives):** add an "Auto route config" select. Lists all `AutoRouteConfig` rows; nullable.

**`web/src/components/Layout.tsx`:** add "Auto Routes" sidebar entry under Admin group. Gate on platform-admin (same as Model Fallbacks).

**`web/src/i18n/en.json` + `zh.json`:**
- `sidebar.autoRoutes` — "Auto Routes" / "自动路由"
- `autoRoutes.{title, createName, modelName, ...}` — page strings
- `models.supportsVision` / `models.supportsTools` — checkbox labels
- `keys.autoRouteConfig` — select label

### Error handling

| Failure | HTTP | Body |
|---|---|---|
| `model=auto` and key has no `auto_route_id` | 400 | `auto_not_configured` |
| `model=auto` and config has 0 models matching required capabilities | 400 | `auto_no_matching_model` with `required_capabilities: [...]` |
| `model=auto` and all (model, channel) candidates fail | 502 | `auto_all_candidates_failed` |
| Model create with name `auto` | 400 | `model_name_reserved` |
| Config id in `api_keys.auto_route_id` doesn't exist (dangling FK) | 400 | `auto_not_configured` (treat as if NULL) |

### Testing

**Unit (`auto_route.rs`):**
- `detect_required_capabilities` — cases:
  - empty body → no capabilities
  - OpenAI text-only messages → no capabilities
  - OpenAI with `image_url` content block → vision
  - OpenAI with `tools: [...]` → tools
  - OpenAI with both → both
  - Anthropic with `type: "image"` → vision
  - Anthropic with `tools` → tools
  - malformed content (string instead of array, missing type) → no crash, no capabilities

**Storage:**
- `list_models_with_capabilities` filter logic across (vision, tools) × (require, don't require) × (in pool, not in pool).

**Integration (`crates/api/tests/test_auto_route.rs` new):**
- `auto_without_config_returns_400`
- `auto_with_vision_routes_to_vision_capable_model`
- `auto_with_tools_routes_to_tools_capable_model`
- `auto_with_no_capabilities_routes_to_any_model_in_pool`
- `auto_with_unsatisfiable_capabilities_returns_400`
- `auto_failover_across_models_when_first_model_channels_fail`
- `auto_all_candidates_failed_returns_502`
- `auto_recorded_in_audit_log_with_client_requested_model_auto`
- `create_model_named_auto_rejected`
- `auto_ignores_models_outside_pool_even_if_capable`

**Frontend (Vitest):**
- Models page: checkboxes toggle, save calls API.
- Auto Routes page: CRUD flows (mirror existing ModelFallbacks tests).
- Keys page: select populates from configs, nullable.

## Migration notes

- Two migrations, both purely additive. No data movement. No risk to existing flows.
- Existing `models` rows get `supports_vision = FALSE`, `supports_tools = FALSE`. Admin must manually tick these on the Models page for any model they want eligible for auto-routing.
- Existing `api_keys` rows get `auto_route_id = NULL`. Sending `model=auto` against such a key returns `auto_not_configured` until an admin assigns a config.

## Open questions

None — all locked-in decisions documented above.

## Out of scope (explicitly)

- Additional capabilities beyond vision + tools.
- Dynamic signals (latency, error-rate) feeding back into selection.
- Cheapest-first / cost-aware tie-breaking.
- Auto-routing falling through to per-key `model_fallbacks` on pool exhaustion.
- Auto behavior when no config assigned (hard-error; no implicit org-wide pool).
- Audit-log schema changes.
- Per-org or per-key capability overrides.
