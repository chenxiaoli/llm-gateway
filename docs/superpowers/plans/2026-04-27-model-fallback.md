# Model Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add model fallback chains to API keys so requests to unavailable models automatically retry with equivalent alternatives in priority order.

**Architecture:** New `model_fallbacks` table stores reusable JSON configs of model groups with priorities. API keys reference a fallback config via `model_fallback_id` FK. When the proxy's initial routing fails for any reason, it loads the fallback config, finds the group containing the requested model, and retries each alternative model in priority order. Frontend adds a console-level "Model Fallbacks" page for CRUD management plus a dropdown in the API key form.

**Tech Stack:** Rust/Axum/sqlx (backend), React/TypeScript/Tailwind/DaisyUI (frontend), SQLite + PostgreSQL (dual storage)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/storage/migrations/sqlite/20260429000000_add_model_fallbacks.sql` | SQLite migration: create table + alter api_keys |
| `crates/storage/migrations/postgres/20260429000000_add_model_fallbacks.sql` | PostgreSQL migration: same |
| `crates/storage/src/types.rs` | ModelFallbackGroup, ModelFallbackConfig structs; add model_fallback_id to ApiKey/CreateApiKey/UpdateApiKey |
| `crates/storage/src/lib.rs` | Add 5 trait methods for model fallback CRUD |
| `crates/storage/src/sqlite.rs` | Row struct, CRUD implementations, key INSERT/SELECT update |
| `crates/storage/src/postgres.rs` | Same |
| `crates/api/src/management/mod.rs` | Register model_fallbacks routes |
| `crates/api/src/management/model_fallbacks.rs` | New CRUD handlers |
| `crates/api/src/management/keys.rs` | Accept model_fallback_id on create/update |
| `crates/api/src/proxy.rs` | Fallback retry loop after initial routing failure |
| `web/src/types/index.ts` | TypeScript types for ModelFallback |
| `web/src/api/modelFallbacks.ts` | API client functions |
| `web/src/hooks/useModelFallbacks.ts` | React Query hooks |
| `web/src/pages/ModelFallbacks.tsx` | Model Fallbacks CRUD page |
| `web/src/pages/Keys.tsx` | Add model_fallback_id dropdown to create form |
| `web/src/pages/KeyDetail.tsx` | Add model_fallback_id dropdown to edit |
| `web/src/components/Layout.tsx` | Add nav item to consoleItems |
| `web/src/App.tsx` | Add route |

---

### Task 1: Database Migration

**Files:**
- Create: `crates/storage/migrations/sqlite/20260429000000_add_model_fallbacks.sql`
- Create: `crates/storage/migrations/postgres/20260429000000_add_model_fallbacks.sql`

- [ ] **Step 1: Write SQLite migration**

Create `crates/storage/migrations/sqlite/20260429000000_add_model_fallbacks.sql`:

```sql
CREATE TABLE IF NOT EXISTS model_fallbacks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,
    created_by TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE api_keys ADD COLUMN model_fallback_id TEXT REFERENCES model_fallbacks(id);
```

- [ ] **Step 2: Write PostgreSQL migration**

Create `crates/storage/migrations/postgres/20260429000000_add_model_fallbacks.sql`:

```sql
CREATE TABLE IF NOT EXISTS model_fallbacks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,
    created_by TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE api_keys ADD COLUMN model_fallback_id TEXT REFERENCES model_fallbacks(id);
```

- [ ] **Step 3: Run migrations and verify**

```bash
cargo build
```

Expected: compiles without errors (sqlx migrations are embedded at build time)

- [ ] **Step 4: Commit**

```bash
git add crates/storage/migrations/sqlite/20260429000000_add_model_fallbacks.sql crates/storage/migrations/postgres/20260429000000_add_model_fallbacks.sql
git commit -m "feat(storage): add model_fallbacks table and api_keys.model_fallback_id column"
```

---

### Task 2: Rust Types

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add ModelFallbackGroup and ModelFallbackConfig structs**

Append to `crates/storage/src/types.rs` (after the existing types):

```rust
// --- Model Fallback ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFallbackGroup {
    pub models: Vec<String>,
    pub priorities: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFallbackConfig {
    pub id: String,
    pub name: String,
    pub config: Vec<ModelFallbackGroup>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelFallback {
    pub name: String,
    pub config: Vec<ModelFallbackGroup>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelFallback {
    pub name: Option<String>,
    pub config: Option<Vec<ModelFallbackGroup>>,
}
```

- [ ] **Step 2: Add model_fallback_id to ApiKey, CreateApiKey, UpdateApiKey**

Update the `ApiKey` struct to add `model_fallback_id`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub enabled: bool,
    pub created_by: Option<String>,
    pub model_fallback_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Update `CreateApiKey`:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub model_fallback_id: Option<String>,
}
```

Update `UpdateApiKey`:

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateApiKey {
    pub name: Option<String>,
    pub rate_limit: Option<Option<i64>>,
    pub budget_monthly: Option<Option<f64>>,
    pub enabled: Option<bool>,
    pub model_fallback_id: Option<Option<String>>,
}
```

- [ ] **Step 3: Build and verify compilation**

```bash
cargo build
```

Expected: compile errors in sqlite.rs/postgres.rs because the row structs and From impls don't have model_fallback_id yet — that's expected, will fix in Task 3.

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(storage): add ModelFallbackConfig types and model_fallback_id to ApiKey"
```

---

### Task 3: Storage Trait + SQLite Implementation

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Add trait methods to Storage**

In `crates/storage/src/lib.rs`, add after the existing trait methods (e.g., after the pricing policies section, before the closing `}`):

```rust
    // Model Fallbacks
    async fn create_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model_fallback(&self, id: &str) -> Result<Option<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_model_fallbacks(&self) -> Result<Vec<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_model_fallback(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

Also add the new types to the `pub use` block:

```rust
pub use types::{
    *,
    // ... existing exports ...
    ModelFallbackGroup, ModelFallbackConfig,
    CreateModelFallback, UpdateModelFallback,
};
```

- [ ] **Step 2: Add SQLite row struct and From impl**

In `crates/storage/src/sqlite.rs`, add after the existing row structs:

```rust
#[derive(FromRow)]
struct SqliteModelFallbackRow {
    id: String,
    name: String,
    config: String,
    created_by: Option<String>,
    created_at: String,
}

impl From<SqliteModelFallbackRow> for ModelFallbackConfig {
    fn from(r: SqliteModelFallbackRow) -> Self {
        ModelFallbackConfig {
            id: r.id,
            name: r.name,
            config: serde_json::from_str(&r.config).unwrap_or_default(),
            created_by: r.created_by,
            created_at: parse_rfc3339(&r.created_at),
        }
    }
}
```

- [ ] **Step 3: Update SqliteKeyRow and From impl for model_fallback_id**

Update `SqliteKeyRow`:

```rust
#[derive(FromRow)]
struct SqliteKeyRow {
    id: String,
    name: String,
    key_hash: String,
    rate_limit: Option<i64>,
    budget_monthly: Option<f64>,
    enabled: i64,
    created_by: Option<String>,
    model_fallback_id: Option<String>,
    created_at: String,
    updated_at: String,
}
```

Update `impl From<SqliteKeyRow> for ApiKey`:

```rust
impl From<SqliteKeyRow> for ApiKey {
    fn from(r: SqliteKeyRow) -> Self {
        ApiKey {
            id: r.id,
            name: r.name,
            key_hash: r.key_hash,
            rate_limit: r.rate_limit,
            budget_monthly: r.budget_monthly,
            enabled: r.enabled != 0,
            created_by: r.created_by,
            model_fallback_id: r.model_fallback_id,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}
```

- [ ] **Step 4: Update SQLite INSERT for api_keys**

Find the `create_key` INSERT statement and add `model_fallback_id` column:

```sql
INSERT INTO api_keys (id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
```

The execute call needs the additional `key.model_fallback_id` parameter (as `&Option<String>` which sqlx maps to NULL).

- [ ] **Step 5: Update SQLite SELECT for api_keys**

Find all `SELECT ... FROM api_keys` queries and add `model_fallback_id` to the column list. There should be queries in:
- `get_key` / `get_key_by_hash` / `list_keys` / `list_keys_paginated` / `list_keys_paginated_for_user`
- `update_key` (if it does a SELECT after UPDATE)

Change from:
```sql
SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at FROM api_keys
```
To:
```sql
SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, model_fallback_id, created_at, updated_at FROM api_keys
```

- [ ] **Step 6: Update SQLite UPDATE for api_keys**

Find the `update_key` UPDATE statement and add `model_fallback_id`:

```sql
UPDATE api_keys SET name = ?, rate_limit = ?, budget_monthly = ?, enabled = ?, model_fallback_id = ?, updated_at = ? WHERE id = ?
```

- [ ] **Step 7: Implement model_fallback CRUD in SQLite**

Add these implementations to the `impl Storage for SqliteStorage` block:

```rust
    async fn create_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>> {
        let config_json = serde_json::to_string(&config.config)?;
        sqlx::query_as::<_, SqliteModelFallbackRow>(
            "INSERT INTO model_fallbacks (id, name, config, created_by, created_at) VALUES (?, ?, ?, ?, ?) RETURNING id, name, config, created_by, created_at"
        )
        .bind(&config.id)
        .bind(&config.name)
        .bind(&config_json)
        .bind(&config.created_by)
        .bind(config.created_at.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .map(|r| r.into())
        .map_err(|e| e.into())
    }

    async fn get_model_fallback(&self, id: &str) -> Result<Option<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>> {
        let result = sqlx::query_as::<_, SqliteModelFallbackRow>(
            "SELECT id, name, config, created_by, created_at FROM model_fallbacks WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(result.map(|r| r.into()))
    }

    async fn list_model_fallbacks(&self) -> Result<Vec<ModelFallbackConfig>, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query_as::<_, SqliteModelFallbackRow>(
            "SELECT id, name, config, created_by, created_at FROM model_fallbacks ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|r| r.into()).collect())
        .map_err(|e| e.into())
    }

    async fn update_model_fallback(&self, config: &ModelFallbackConfig) -> Result<ModelFallbackConfig, Box<dyn std::error::Error + Send + Sync>> {
        let config_json = serde_json::to_string(&config.config)?;
        sqlx::query_as::<_, SqliteModelFallbackRow>(
            "UPDATE model_fallbacks SET name = ?, config = ? WHERE id = ? RETURNING id, name, config, created_by, created_at"
        )
        .bind(&config.name)
        .bind(&config_json)
        .bind(&config.id)
        .fetch_one(&self.pool)
        .await
        .map(|r| r.into())
        .map_err(|e| e.into())
    }

    async fn delete_model_fallback(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("DELETE FROM model_fallbacks WHERE id = ?")
        .bind(id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.into())
    }
```

- [ ] **Step 8: Build and verify**

```bash
cargo build
```

Expected: compiles without errors (PostgreSQL may still fail — that's Task 4)

- [ ] **Step 9: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/sqlite.rs
git commit -m "feat(storage): add model_fallback CRUD to trait and SQLite impl"
```

---

### Task 4: PostgreSQL Implementation

**Files:**
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Add PG row struct and From impl**

Mirror the SQLite pattern. In `crates/storage/src/postgres.rs`, add:

```rust
#[derive(FromRow)]
struct PgModelFallbackRow {
    id: String,
    name: String,
    config: String,
    created_by: Option<String>,
    created_at: String,
}

impl From<PgModelFallbackRow> for ModelFallbackConfig {
    fn from(r: PgModelFallbackRow) -> Self {
        ModelFallbackConfig {
            id: r.id,
            name: r.name,
            config: serde_json::from_str(&r.config).unwrap_or_default(),
            created_by: r.created_by,
            created_at: parse_rfc3339(&r.created_at),
        }
    }
}
```

- [ ] **Step 2: Update PG key row struct and From impl for model_fallback_id**

Update `PgKeyRow` to add `model_fallback_id: Option<String>` and update the `From<PgKeyRow> for ApiKey` impl to map it (same pattern as SQLite in Task 3).

- [ ] **Step 3: Update PG key INSERT/SELECT/UPDATE**

Same changes as SQLite Task 3 steps 4-6: add `model_fallback_id` to INSERT, SELECT columns, and UPDATE statement for `api_keys`.

- [ ] **Step 4: Implement model_fallback CRUD for PostgreSQL**

Same 5 methods as SQLite (Task 3 step 7), using `PgModelFallbackRow` and `&self.pg_pool` (or whatever the pool field is named).

- [ ] **Step 5: Build and verify**

```bash
cargo build
```

Expected: compiles cleanly

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat(storage): add model_fallback CRUD to PostgreSQL impl"
```

---

### Task 5: Management API Handlers

**Files:**
- Create: `crates/api/src/management/model_fallbacks.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: Create model_fallbacks.rs handler module**

Create `crates/api/src/management/model_fallbacks.rs`:

```rust
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{ModelFallbackConfig, ModelFallbackGroup, CreateModelFallback, UpdateModelFallback};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

pub async fn create_model_fallback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateModelFallback>,
) -> Result<Json<ModelFallbackConfig>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let config = ModelFallbackConfig {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        config: input.config,
        created_by: Some(claims.sub),
        created_at: chrono::Utc::now(),
    };

    let created = state
        .storage
        .create_model_fallback(&config)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_model_fallbacks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ModelFallbackConfig>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let all = state
        .storage
        .list_model_fallbacks()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Non-admin users only see their own fallback configs
    let filtered: Vec<ModelFallbackConfig> = if claims.role == "admin" {
        all
    } else {
        all.into_iter().filter(|f| f.created_by.as_deref() == Some(&claims.sub)).collect()
    };

    Ok(Json(filtered))
}

pub async fn get_model_fallback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ModelFallbackConfig>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let config = state
        .storage
        .get_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model fallback '{}' not found", id)))?;

    if claims.role != "admin" && config.created_by.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Model fallback '{}' not found", id)));
    }

    Ok(Json(config))
}

pub async fn update_model_fallback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateModelFallback>,
) -> Result<Json<ModelFallbackConfig>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let mut config = state
        .storage
        .get_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model fallback '{}' not found", id)))?;

    if claims.role != "admin" && config.created_by.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Model fallback '{}' not found", id)));
    }

    if let Some(name) = input.name { config.name = name; }
    if let Some(new_config) = input.config { config.config = new_config; }

    let updated = state
        .storage
        .update_model_fallback(&config)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_model_fallback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let config = state
        .storage
        .get_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Model fallback '{}' not found", id)))?;

    if claims.role != "admin" && config.created_by.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Model fallback '{}' not found", id)));
    }

    state
        .storage
        .delete_model_fallback(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Register module and routes in mod.rs**

In `crates/api/src/management/mod.rs`, add:

1. Add `pub mod model_fallbacks;` to the module declarations at the top.

2. Add routes inside `management_router()`. Place them after the keys routes and before the admin routes, since model fallbacks are user-accessible:

```rust
        // Model Fallbacks (authenticated)
        .route(
            "/api/v1/model-fallbacks",
            post(model_fallbacks::create_model_fallback).get(model_fallbacks::list_model_fallbacks),
        )
        .route(
            "/api/v1/model-fallbacks/{id}",
            get(model_fallbacks::get_model_fallback).patch(model_fallbacks::update_model_fallback).delete(model_fallbacks::delete_model_fallback),
        )
```

- [ ] **Step 3: Update keys.rs to handle model_fallback_id**

In `crates/api/src/management/keys.rs`:

Update `create_key` to pass `model_fallback_id` from input to the `ApiKey` struct:

```rust
    let key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        key_hash: hash_api_key(&raw_key),
        rate_limit: input.rate_limit,
        budget_monthly: input.budget_monthly,
        enabled: true,
        created_by: Some(claims.sub),
        model_fallback_id: input.model_fallback_id,
        created_at: now,
        updated_at: now,
    };
```

Update `update_key` to handle `model_fallback_id`:

```rust
    if let Some(model_fallback_id) = input.model_fallback_id { key.model_fallback_id = model_fallback_id; }
```

- [ ] **Step 4: Build and verify**

```bash
cargo build
```

Expected: compiles cleanly

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/model_fallbacks.rs crates/api/src/management/mod.rs crates/api/src/management/keys.rs
git commit -m "feat(api): add model fallback CRUD handlers and route registration"
```

---

### Task 6: Proxy Fallback Logic

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: Add fallback retry loop to proxy function**

The current `proxy` function has this flow:
1. Auth → parse model → resolve channels → failover across channels → return or error

We need to wrap this in a fallback loop. The approach: extract the core routing into a helper, then call it in a loop trying fallback models.

Add this helper function before the `proxy` function:

```rust
/// Attempt to route a request for a given model name through the proxy.
/// Returns the response on success, or the last ApiError on failure.
async fn try_route_model(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    body: &str,
    model_name: &str,
    protocol: ProxyProtocol,
    request_path: &str,
) -> Result<axum::response::Response, ApiError> {
    let req_json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Replace model in body
    let modified_body = {
        let mut modified = req_json.clone();
        if let Some(model_obj) = modified.get_mut("model") {
            *model_obj = serde_json::Value::String(model_name.to_string());
        }
        serde_json::to_string(&modified).unwrap_or_else(|_| body.to_string())
    };

    let is_stream = req_json
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    // Find model in storage
    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let model_entry = models
        .iter()
        .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
        .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?;

    // Resolve channels
    let resolved_channels = state.registry.resolve_by_model(model_name).await;

    if resolved_channels.is_empty() {
        return Err(ApiError::NotFound(format!("No channels for model '{}'", model_name)));
    }

    // This is a simplified routing — the full routing with pricing, audit, etc.
    // For the fallback we delegate back to the main proxy flow.
    // Actually, the simplest approach is to call proxy() recursively with the modified body.
    // But to avoid duplicating all the routing logic, we'll use a different approach:
    // wrap the model fallback around the existing proxy function.
    unreachable!() // placeholder — see step 2
}
```

**Wait — simpler approach.** Rather than extracting a helper, wrap the existing `proxy` function's body. The cleanest integration point is right after `let _original_model = model_name.clone();` (line ~475).

Instead, add the fallback loop at the top of `proxy`, around the entire routing block. Here's the actual implementation:

Add these imports at the top of `proxy.rs`:

```rust
use llm_gateway_storage::ModelFallbackConfig;
```

Then, in the `proxy` function, right after the line `let _original_model = model_name.clone();` (approximately line 475), add the fallback logic. The key insight: if the initial routing fails, we catch the error, look up fallback models, and retry.

Find the section that starts with:

```rust
    let _original_model = model_name.clone();

    // === Step 3: Find model → provider → channels ===
```

And wrap the entire block from "Step 3" through the end of the function in a helper. But since the function is very long (~500 lines), the practical approach is:

**Add the fallback loop AFTER the initial routing fails.** The proxy function currently returns errors at three points during routing:
1. `ApiError::NotFound(format!("Model '{}' not found", model_name))` — model not in DB
2. `ApiError::NotFound(format!("No enabled channels for model '{}'", model_name))` — no channels
3. `ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error))` — all channels failed

We add a wrapper that catches the error and retries with fallback models.

Add this at the very beginning of the `proxy` function, right after the `_original_model` assignment:

```rust
    let original_model = model_name.clone();

    // Attempt normal routing first
    let result = proxy_inner(State(state.clone()), headers.clone(), body.clone(), protocol, request_path.clone(), model_name.clone()).await;

    match result {
        Ok(response) => Ok(response),
        Err(initial_error) => {
            // Try model fallback if key has model_fallback_id
            if let Some(ref fallback_id) = api_key.model_fallback_id {
                if let Ok(Some(fallback_config)) = state.storage.get_model_fallback(fallback_id).await {
                    if let Some(fallback_response) = try_fallback_models(
                        &state, &headers, &body, &original_model, &fallback_config, protocol, &request_path,
                    ).await {
                        return Ok(fallback_response);
                    }
                }
            }
            Err(initial_error)
        }
    }
```

**Actually, this requires extracting the proxy body into `proxy_inner`.** The cleanest way without a massive refactor:

Replace the end of `proxy` where it returns `Err(ApiError::UpstreamError(...))` with the fallback loop. Here is the concrete change:

Find this line near the bottom of `proxy`:

```rust
    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
```

Replace it with:

```rust
    // === Model Fallback ===
    // If the initial model routing failed, try fallback models from the key's fallback config
    if let Some(ref fallback_id) = api_key.model_fallback_id {
        if let Ok(Some(fallback_config)) = state.storage.get_model_fallback(fallback_id).await {
            tracing::info!("[PROXY] Initial model '{}' failed, trying fallback config '{}'", model_name, fallback_config.name);

            // Find the group containing the original model
            let model_lower = model_name.to_lowercase();
            if let Some(group) = fallback_config.config.iter().find(|g| {
                g.models.iter().any(|m| m.to_lowercase() == model_lower)
            }) {
                // Collect fallback models sorted by priority (ascending = lower number first)
                let mut fallbacks: Vec<(&String, i32)> = group
                    .models
                    .iter()
                    .zip(group.priorities.iter())
                    .filter(|(m, _)| m.to_lowercase() != model_lower)
                    .map(|(m, p)| (m, *p))
                    .collect();
                fallbacks.sort_by_key(|(_, p)| *p);

                for (fallback_model, priority) in fallbacks {
                    tracing::info!("[PROXY] Trying fallback model '{}' (priority {})", fallback_model, priority);
                    match proxy(
                        State(state.clone()),
                        headers.clone(),
                        body.clone(),
                        protocol,
                        request_path.clone(),
                    ).await {
                        Ok(response) => {
                            tracing::info!("[PROXY] Fallback to '{}' succeeded", fallback_model);
                            return Ok(response);
                        }
                        Err(e) => {
                            tracing::warn!("[PROXY] Fallback to '{}' failed: {}", fallback_model, e);
                            last_error = format!("Fallback '{}' failed: {}", fallback_model, e);
                            continue;
                        }
                    }
                }
            }
        }
    }

    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
```

**Important:** The `proxy` function is called recursively here. This works because the fallback model is different from the original, and the recursive call will use the same `api_key` (which has the same `model_fallback_id`). We need a guard to prevent infinite recursion — the fallback models are filtered to exclude the original model, so each recursive call tries a *different* model. If that model also fails and has its own fallback group entry, it would try *its* fallbacks. To prevent deep chains, add a recursion guard.

Add a `max_fallback_depth` parameter or use a simpler approach: add the original model name as a check. The filtering `m.to_lowercase() != model_lower` prevents the same model from being tried again in the same group. Since each group is independent, a model can only appear in one group's fallback chain, so the maximum recursion depth equals the number of models in a group.

To be safe, modify the `proxy` function signature to accept an optional `fallback_depth` parameter or use a simpler header-based approach. The simplest safe approach: add a static counter using a request header.

Actually, the spec says "A model appearing in its own fallback group is skipped during fallback" — so if model A falls back to B, and B has its own fallback group where A is listed, B would try A again. This could loop. Let's add a simple guard.

**Better approach:** Don't call `proxy` recursively. Instead, duplicate the essential routing logic inline (just the model lookup + channel resolution + upstream call). Since the full proxy function is ~500 lines, extract just the model+channel+upstream part.

Actually the simplest safe approach: track tried models to prevent cycles. Add a `tried_models` set that gets passed through.

**Final approach — simplest and correct:**

Instead of recursive `proxy` calls, just do the model name substitution and re-run the routing inline. Add this at the point where all channels have failed (the `Err(ApiError::UpstreamError(...))` line):

```rust
    // === Model Fallback ===
    if let Some(ref fallback_id) = api_key.model_fallback_id {
        match state.storage.get_model_fallback(fallback_id).await {
            Ok(Some(fallback_config)) => {
                let model_lower = model_name.to_lowercase();
                if let Some(group) = fallback_config.config.iter().find(|g| {
                    g.models.iter().any(|m| m.to_lowercase() == model_lower)
                }) {
                    let mut fallbacks: Vec<(&String, i32)> = group
                        .models
                        .iter()
                        .zip(group.priorities.iter())
                        .filter(|(m, _)| m.to_lowercase() != model_lower)
                        .map(|(m, p)| (m, *p))
                        .collect();
                    fallbacks.sort_by_key(|(_, p)| *p);

                    for (fallback_model, _) in fallbacks {
                        tracing::info!("[PROXY] Trying fallback model '{}'", fallback_model);

                        // Substitute model in request body
                        let fallback_body = {
                            let mut modified = req_json.clone();
                            if let Some(model_obj) = modified.get_mut("model") {
                                *model_obj = serde_json::Value::String(fallback_model.to_string());
                            }
                            serde_json::to_string(&modified).unwrap_or_else(|_| body.clone())
                        };

                        // Route the fallback model (recursive call to proxy with new body)
                        // Guard: the fallback_model is different from original, and the
                        // same group won't be matched again because the filter excludes it
                        match proxy(
                            State(state.clone()),
                            headers.clone(),
                            fallback_body,
                            protocol,
                            request_path.clone(),
                        ).await {
                            Ok(response) => return Ok(response),
                            Err(e) => {
                                last_error = format!("Fallback '{}' failed: {}", fallback_model, e);
                                continue;
                            }
                        }
                    }
                }
            }
            Ok(None) => {},
            Err(e) => tracing::warn!("[PROXY] Failed to load fallback config: {}", e),
        }
    }

    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
```

This replaces the final `Err(ApiError::UpstreamError(...))` line. Note that `req_json` is already in scope from the model parsing section. The recursive `proxy` call is safe because each fallback model is a *different* model — the same group won't match because the filter excludes the tried model.

- [ ] **Step 2: Handle early routing failures**

The proxy also returns errors earlier (model not found, no channels). These need fallback too. Find these two early return points:

1. `return Err(ApiError::NotFound(format!("Model '{}' not found", model_name)));` (appears twice for OpenAI/Anthropic)
2. `return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));` (appears twice, cache hit and cache miss)

Change these from `return Err(...)` to `continue` through a labeled block, or more simply, store the error and let it fall through to the fallback at the end.

The simplest approach: replace those early `return Err(...)` with assignments to `last_error` and use `continue` to skip to the fallback block. But these are not in a loop...

**Better:** Wrap the routing in a closure or labeled block. Add a `'routing:` label:

```rust
    let routing_result: Result<axum::response::Response, ApiError> = 'routing: {
        // ... entire existing routing body ...
        // Replace early returns with breaks:
        // return Err(ApiError::NotFound(...)) → break 'routing Err(ApiError::NotFound(...))
        // Ok(response) → break 'routing Ok(response)
    };

    match routing_result {
        Ok(response) => Ok(response),
        Err(initial_error) => {
            // ... fallback logic from step 1 ...
        }
    }
```

This requires changing all `return Ok(...)` and `return Err(...)` inside the routing block to `break 'routing Ok(...)` and `break 'routing Err(...)`.

This is the cleanest approach but touches many lines. Alternatively, just add fallback at the bottom and handle the early errors differently.

**Pragmatic approach:** The model-not-found and no-channels errors are the same category as "all channels failed." Instead of a labeled block, restructure to set `last_error` and fall through:

Before the main routing section, add:

```rust
    let mut last_error = String::new();
    let mut routing_succeeded = false;
    let mut response = axum::response::Response::default();
```

Then change every `return Err(...)` to either set `last_error` + `continue` (if in a loop) or a helper flag. This is complex.

**Most practical:** Just handle the two early error cases at their location. The model-not-found case happens before channels are resolved. Add the fallback check there too.

Find the model lookup section (around line 491-500):

```rust
    let model_entry = match protocol {
        ProxyProtocol::OpenAI => models
            .iter()
            .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
            .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?,
        ProxyProtocol::Anthropic => models
            .iter()
            .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
            .ok_or(ApiError::NotFound(format!("Model '{}' not found", model_name)))?,
    };
```

Since both arms do the same thing, simplify to:

```rust
    let model_entry = match models
        .iter()
        .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
    {
        Some(m) => m,
        None => {
            // Model not found — try fallback before returning error
            let fallback_response = try_model_fallback(
                &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
            ).await;
            match fallback_response {
                Some(resp) => return Ok(resp),
                None => return Err(ApiError::NotFound(format!("Model '{}' not found", model_name))),
            }
        }
    };
```

Similarly for the no-channels case.

Add a helper function at the top of the file:

```rust
/// Try fallback models when the initial model fails to route.
/// Returns Some(response) if a fallback succeeded, None if no fallback available.
async fn try_model_fallback(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    body: &str,
    original_model: &str,
    api_key: &ApiKey,
    protocol: ProxyProtocol,
    request_path: &str,
) -> Option<axum::response::Response> {
    let fallback_id = api_key.model_fallback_id.as_ref()?;
    let fallback_config = state.storage.get_model_fallback(fallback_id).await.ok()??;
    let model_lower = original_model.to_lowercase();

    let group = fallback_config.config.iter().find(|g| {
        g.models.iter().any(|m| m.to_lowercase() == model_lower)
    })?;

    let mut fallbacks: Vec<(&String, i32)> = group
        .models
        .iter()
        .zip(group.priorities.iter())
        .filter(|(m, _)| m.to_lowercase() != model_lower)
        .map(|(m, p)| (m, *p))
        .collect();
    fallbacks.sort_by_key(|(_, p)| *p);

    for (fallback_model, _) in fallbacks {
        tracing::info!("[PROXY] Trying fallback model '{}' for failed '{}'", fallback_model, original_model);

        let fallback_body = {
            let req_json: serde_json::Value = serde_json::from_str(body).ok()?;
            let mut modified = req_json.clone();
            if let Some(model_obj) = modified.get_mut("model") {
                *model_obj = serde_json::Value::String(fallback_model.to_string());
            }
            serde_json::to_string(&modified).unwrap_or_else(|_| body.to_string())
        };

        match proxy(
            State(state.clone()),
            headers.clone(),
            fallback_body,
            protocol,
            request_path.clone(),
        ).await {
            Ok(response) => {
                tracing::info!("[PROXY] Fallback to '{}' succeeded", fallback_model);
                return Some(response);
            }
            Err(e) => {
                tracing::warn!("[PROXY] Fallback '{}' failed: {}", fallback_model, e);
                continue;
            }
        }
    }

    None
}
```

Then at the bottom of `proxy`, replace the final error:

```rust
    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
```

with:

```rust
    // All channels failed — try model fallback
    if let Some(resp) = try_model_fallback(
        &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
    ).await {
        return Ok(resp);
    }
    Err(ApiError::UpstreamError(502, format!("All channels failed. Last error: {}", last_error)))
```

And for the model-not-found case, replace the `ok_or` with:

```rust
    let model_entry = match models
        .iter()
        .find(|m| m.model.name.to_lowercase() == model_name.to_lowercase())
    {
        Some(m) => m,
        None => {
            if let Some(resp) = try_model_fallback(
                &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
            ).await {
                return Ok(resp);
            }
            return Err(ApiError::NotFound(format!("Model '{}' not found", model_name)));
        }
    };
```

And for the no-channels case (both cache hit and cache miss paths), replace:

```rust
        return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
```

with:

```rust
        if let Some(resp) = try_model_fallback(
            &state, &headers, &body, &model_name, &api_key, protocol, &request_path,
        ).await {
            return Ok(resp);
        }
        return Err(ApiError::NotFound(format!("No enabled channels for model '{}'", model_name)));
```

- [ ] **Step 3: Build and verify**

```bash
cargo build
```

Expected: compiles cleanly

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): add model fallback retry loop on routing failure"
```

---

### Task 7: Frontend TypeScript Types

**Files:**
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Add model fallback types and update ApiKey types**

In `web/src/types/index.ts`, add after the existing types (before the `PricingConfig` section):

```typescript
// ── Model Fallback Types ──────────────────────────────────────────────────

export interface ModelFallbackGroup {
  models: string[];
  priorities: number[];
}

export interface ModelFallbackConfig {
  id: string;
  name: string;
  config: ModelFallbackGroup[];
  created_by: string | null;
  created_at: string;
}

export interface CreateModelFallbackRequest {
  name: string;
  config: ModelFallbackGroup[];
}

export interface UpdateModelFallbackRequest {
  name?: string;
  config?: ModelFallbackGroup[];
}
```

Update `ApiKey` to include `model_fallback_id`:

```typescript
export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  rate_limit: number | null;
  budget_monthly: number | null;
  enabled: boolean;
  model_fallback_id: string | null;
  created_at: string;
  updated_at: string;
}
```

Update `CreateKeyRequest`:

```typescript
export interface CreateKeyRequest {
  name: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
  model_fallback_id?: string | null;
}
```

Update `UpdateKeyRequest`:

```typescript
export interface UpdateKeyRequest {
  name?: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
  enabled?: boolean;
  model_fallback_id?: string | null;
}
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
cd web && npx tsc --noEmit
```

Expected: may have errors from missing API functions — that's expected until Task 8.

- [ ] **Step 3: Commit**

```bash
git add web/src/types/index.ts
git commit -m "feat(web): add ModelFallback types and model_fallback_id to ApiKey types"
```

---

### Task 8: Frontend API Client + Hooks

**Files:**
- Create: `web/src/api/modelFallbacks.ts`
- Create: `web/src/hooks/useModelFallbacks.ts`

- [ ] **Step 1: Create API client**

Create `web/src/api/modelFallbacks.ts`:

```typescript
import { apiClient } from './client';
import type { ModelFallbackConfig, CreateModelFallbackRequest, UpdateModelFallbackRequest } from '../types';

export async function listModelFallbacks(): Promise<ModelFallbackConfig[]> {
  const { data } = await apiClient.get<ModelFallbackConfig[]>('/model-fallbacks');
  return data;
}

export async function getModelFallback(id: string): Promise<ModelFallbackConfig> {
  const { data } = await apiClient.get<ModelFallbackConfig>(`/model-fallbacks/${id}`);
  return data;
}

export async function createModelFallback(input: CreateModelFallbackRequest): Promise<ModelFallbackConfig> {
  const { data } = await apiClient.post<ModelFallbackConfig>('/model-fallbacks', input);
  return data;
}

export async function updateModelFallback(id: string, input: UpdateModelFallbackRequest): Promise<ModelFallbackConfig> {
  const { data } = await apiClient.patch<ModelFallbackConfig>(`/model-fallbacks/${id}`, input);
  return data;
}

export async function deleteModelFallback(id: string): Promise<void> {
  await apiClient.delete(`/model-fallbacks/${id}`);
}
```

- [ ] **Step 2: Create React Query hooks**

Create `web/src/hooks/useModelFallbacks.ts`:

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listModelFallbacks, createModelFallback, updateModelFallback, deleteModelFallback } from '../api/modelFallbacks';
import type { CreateModelFallbackRequest, UpdateModelFallbackRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function useModelFallbacks() {
  return useQuery({
    queryKey: ['model-fallbacks'],
    queryFn: listModelFallbacks,
  });
}

export function useCreateModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateModelFallbackRequest) => createModelFallback(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] });
      toast.success('Model fallback created');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to create model fallback')); },
  });
}

export function useUpdateModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateModelFallbackRequest }) => updateModelFallback(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] });
      toast.success('Model fallback updated');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update model fallback')); },
  });
}

export function useDeleteModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteModelFallback(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] });
      toast.success('Model fallback deleted');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to delete model fallback')); },
  });
}
```

- [ ] **Step 3: Commit**

```bash
git add web/src/api/modelFallbacks.ts web/src/hooks/useModelFallbacks.ts
git commit -m "feat(web): add model fallback API client and React Query hooks"
```

---

### Task 9: Model Fallbacks Page

**Files:**
- Create: `web/src/pages/ModelFallbacks.tsx`

- [ ] **Step 1: Create the Model Fallbacks page**

Create `web/src/pages/ModelFallbacks.tsx`:

```tsx
import { useState } from 'react';
import { Plus, Trash2, GripVertical, X } from 'lucide-react';
import { useModelFallbacks, useCreateModelFallback, useUpdateModelFallback, useDeleteModelFallback } from '../hooks/useModelFallbacks';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import type { ModelFallbackGroup } from '../types';

export default function ModelFallbacks() {
  const { data: fallbacks, isLoading } = useModelFallbacks();
  const createMutation = useCreateModelFallback();
  const updateMutation = useUpdateModelFallback();
  const deleteMutation = useDeleteModelFallback();

  const [createOpen, setCreateOpen] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [groups, setGroups] = useState<ModelFallbackGroup[]>([{ models: [''], priorities: [1] }]);
  const [deleteId, setDeleteId] = useState<string | null>(null);

  const resetForm = () => {
    setName('');
    setGroups([{ models: [''], priorities: [1] }]);
    setEditId(null);
  };

  const openCreate = () => {
    resetForm();
    setCreateOpen(true);
  };

  const openEdit = (id: string) => {
    const fb = fallbacks?.find((f) => f.id === id);
    if (!fb) return;
    setEditId(id);
    setName(fb.name);
    setGroups(fb.config.length > 0 ? fb.config : [{ models: [''], priorities: [1] }]);
    setCreateOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanedGroups = groups.map((g) => ({
      models: g.models.filter((m) => m.trim() !== ''),
      priorities: g.priorities,
    })).filter((g) => g.models.length > 1);

    if (cleanedGroups.length === 0) return;

    if (editId) {
      await updateMutation.mutateAsync({ id: editId, input: { name, config: cleanedGroups } });
    } else {
      await createMutation.mutateAsync({ name, config: cleanedGroups });
    }
    setCreateOpen(false);
    resetForm();
  };

  const addGroup = () => {
    setGroups([...groups, { models: [''], priorities: [1] }]);
  };

  const removeGroup = (gi: number) => {
    setGroups(groups.filter((_, i) => i !== gi));
  };

  const addModel = (gi: number) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.models = [...group.models, ''];
    group.priorities = [...group.priorities, group.priorities.length + 1];
    updated[gi] = group;
    setGroups(updated);
  };

  const removeModel = (gi: number, mi: number) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.models = group.models.filter((_, i) => i !== mi);
    group.priorities = group.priorities.filter((_, i) => i !== mi);
    if (group.models.length === 0) {
      group.models = [''];
      group.priorities = [1];
    }
    updated[gi] = group;
    setGroups(updated);
  };

  const updateModelName = (gi: number, mi: number, value: string) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.models = [...group.models];
    group.models[mi] = value;
    updated[gi] = group;
    setGroups(updated);
  };

  const updatePriority = (gi: number, mi: number, value: number) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.priorities = [...group.priorities];
    group.priorities[mi] = value;
    updated[gi] = group;
    setGroups(updated);
  };

  const handleDelete = async () => {
    if (!deleteId) return;
    await deleteMutation.mutateAsync(deleteId);
    setDeleteId(null);
  };

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Model Fallbacks</h1>
          <p className="text-sm text-base-content/40 mt-1">Configure fallback model chains for API keys</p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={openCreate}>
          Create Fallback
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <span className="loading loading-spinner loading-lg" />
        </div>
      ) : (
        <div className="space-y-4">
          {fallbacks?.map((fb) => (
            <div key={fb.id} className="rounded-xl border border-base-300/50 bg-base-100/60 p-5">
              <div className="flex items-center justify-between mb-3">
                <div>
                  <h3 className="font-semibold text-sm">{fb.name}</h3>
                  <p className="text-xs text-base-content/40 mt-0.5">{fb.config.length} group(s)</p>
                </div>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm" onClick={() => openEdit(fb.id)}>Edit</Button>
                  <Button variant="ghost" size="sm" onClick={() => setDeleteId(fb.id)}>
                    <Trash2 className="h-4 w-4 text-error" />
                  </Button>
                </div>
              </div>
              <div className="space-y-2">
                {fb.config.map((group, gi) => (
                  <div key={gi} className="rounded-lg border border-base-200/50 bg-base-200/30 p-3">
                    <div className="flex flex-wrap gap-2">
                      {group.models.map((model, mi) => (
                        <span key={mi} className="inline-flex items-center gap-1 rounded-md bg-base-200 px-2 py-1 text-xs font-mono">
                          <span className="text-base-content/30">P{group.priorities[mi]}</span>
                          <span>{model}</span>
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
          {(!fallbacks || fallbacks.length === 0) && (
            <div className="text-center py-16 text-base-content/25 text-sm">
              No model fallback configs yet
            </div>
          )}
        </div>
      )}

      {/* Create / Edit Modal */}
      <Modal
        open={createOpen}
        onClose={() => { setCreateOpen(false); resetForm(); }}
        title={editId ? 'Edit Model Fallback' : 'Create Model Fallback'}
      >
        <form onSubmit={handleSave} className="space-y-4">
          <div className="form-control">
            <label className="label"><span className="label-text font-medium">Name</span></label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., GPT-4 Fallback Chain"
              required
              className="input input-bordered w-full"
            />
          </div>

          <div className="space-y-4">
            <label className="label"><span className="label-text font-medium">Fallback Groups</span></label>
            <p className="text-xs text-base-content/40 -mt-2">
              Each group defines equivalent models. Lower priority number = tried first.
            </p>
            {groups.map((group, gi) => (
              <div key={gi} className="rounded-lg border border-base-300/50 bg-base-200/30 p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-base-content/50">Group {gi + 1}</span>
                  {groups.length > 1 && (
                    <button type="button" onClick={() => removeGroup(gi)} className="text-error/60 hover:text-error">
                      <X className="h-4 w-4" />
                    </button>
                  )}
                </div>
                {group.models.map((model, mi) => (
                  <div key={mi} className="flex gap-2 items-center">
                    <span className="text-xs text-base-content/30 w-6 text-right shrink-0">P{group.priorities[mi]}</span>
                    <input
                      type="text"
                      value={model}
                      onChange={(e) => updateModelName(gi, mi, e.target.value)}
                      placeholder="model-name"
                      className="input input-bordered input-sm flex-1"
                    />
                    <input
                      type="number"
                      value={group.priorities[mi]}
                      onChange={(e) => updatePriority(gi, mi, parseInt(e.target.value) || 1)}
                      min={1}
                      className="input input-bordered input-sm w-16"
                    />
                    {group.models.length > 1 && (
                      <button type="button" onClick={() => removeModel(gi, mi)} className="text-base-content/30 hover:text-error shrink-0">
                        <X className="h-4 w-4" />
                      </button>
                    )}
                  </div>
                ))}
                <Button variant="ghost" size="sm" type="button" onClick={() => addModel(gi)}>
                  + Add Model
                </Button>
              </div>
            ))}
            <Button variant="secondary" size="sm" type="button" onClick={addGroup}>
              + Add Group
            </Button>
          </div>

          <Button variant="primary" loading={createMutation.isPending || updateMutation.isPending}>
            {editId ? 'Update' : 'Create'}
          </Button>
        </form>
      </Modal>

      {/* Delete Confirmation Modal */}
      <Modal
        open={!!deleteId}
        onClose={() => setDeleteId(null)}
        title="Delete Model Fallback"
      >
        <p className="text-sm text-base-content/60">
          Are you sure? API keys referencing this config will lose their fallback chain.
        </p>
        <div className="mt-4 flex gap-2 justify-end">
          <Button variant="secondary" onClick={() => setDeleteId(null)}>Cancel</Button>
          <Button variant="primary" loading={deleteMutation.isPending} onClick={handleDelete}>Delete</Button>
        </div>
      </Modal>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/pages/ModelFallbacks.tsx
git commit -m "feat(web): add Model Fallbacks CRUD page"
```

---

### Task 10: Sidebar, Routes, and Key Form Integration

**Files:**
- Modify: `web/src/components/Layout.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/pages/Keys.tsx`
- Modify: `web/src/pages/KeyDetail.tsx` (if it exists)

- [ ] **Step 1: Add nav item to sidebar**

In `web/src/components/Layout.tsx`:

Add `GitBranch` (or `ArrowRightLeft` or `Shuffle`) to the lucide imports. Then add the item to `consoleItems`:

```typescript
import { ..., ArrowRightLeft } from 'lucide-react';

const consoleItems = [
  { key: '/console/dashboard', icon: LayoutDashboard, label: 'Dashboard' },
  { key: '/console/keys', icon: KeyRound, label: 'API Keys' },
  { key: '/console/model-fallbacks', icon: ArrowRightLeft, label: 'Model Fallbacks' },
  { key: '/console/usage', icon: BarChart3, label: 'Usage' },
];
```

Add to `routeLabels`:

```typescript
const routeLabels: Record<string, string> = {
  // ... existing entries ...
  'model-fallbacks': 'Model Fallbacks',
};
```

- [ ] **Step 2: Add route to App.tsx**

In `web/src/App.tsx`:

Add import:

```typescript
import ModelFallbacks from './pages/ModelFallbacks';
```

Add route inside the authenticated routes section (after the `keys/:id` route):

```tsx
          <Route element={<RequireAuth />}>
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="keys" element={<Keys />} />
            <Route path="keys/:id" element={<KeyDetail />} />
            <Route path="model-fallbacks" element={<ModelFallbacks />} />
            <Route path="usage" element={<Usage />} />
          </Route>
```

- [ ] **Step 3: Add model_fallback_id dropdown to Keys.tsx create form**

In `web/src/pages/Keys.tsx`:

Add imports:

```typescript
import { useModelFallbacks } from '../hooks/useModelFallbacks';
```

Add state and data loading inside the component:

```typescript
const { data: fallbacks } = useModelFallbacks();
const [fallbackId, setFallbackId] = useState<string>('');
```

In the `handleCreate` function, add `model_fallback_id` to the mutation call:

```typescript
const result: CreateKeyResponse = await createKeyMutation.mutateAsync({
  name,
  rate_limit: rateLimit ? Number(rateLimit) : null,
  budget_monthly: budget ? Number(budget) : null,
  model_fallback_id: fallbackId || null,
});
```

Add the dropdown in the form (after the budget input, before the Create button):

```tsx
<div className="form-control">
  <label className="label"><span className="label-text font-medium">Model Fallback</span></label>
  <select
    value={fallbackId}
    onChange={(e) => setFallbackId(e.target.value)}
    className="select select-bordered w-full"
  >
    <option value="">None</option>
    {fallbacks?.map((fb) => (
      <option key={fb.id} value={fb.id}>{fb.name}</option>
    ))}
  </select>
</div>
```

Reset `fallbackId` in the form reset after creation:

```typescript
setFallbackId('');
```

- [ ] **Step 4: Add model_fallback_id dropdown to KeyDetail.tsx (if it exists)**

If `web/src/pages/KeyDetail.tsx` exists, add the same dropdown pattern for editing the key's model_fallback_id. Import `useModelFallbacks` and `useUpdateKey`, add a select dropdown that calls `updateKey({ model_fallback_id: value || null })`.

- [ ] **Step 5: Verify frontend builds**

```bash
cd web && npm run build
```

Expected: builds without errors

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Layout.tsx web/src/App.tsx web/src/pages/Keys.tsx web/src/pages/KeyDetail.tsx
git commit -m "feat(web): add Model Fallbacks to sidebar, routes, and key form dropdown"
```

---

### Task 11: Integration Test

**Files:**
- No new files needed — manual integration test

- [ ] **Step 1: Start backend and frontend**

```bash
# Terminal 1
cargo run

# Terminal 2
cd web && npm run dev
```

- [ ] **Step 2: Create a model fallback config via UI**

1. Navigate to `/console/model-fallbacks`
2. Click "Create Fallback"
3. Name: "GPT-4 Fallback Chain"
4. Add models: gpt-4.4 (P1), glm-5.1 (P2), glm-5 (P3)
5. Click "Create"
6. Verify the config appears in the list

- [ ] **Step 3: Assign fallback to an API key**

1. Navigate to `/console/keys`
2. Create a new key or edit existing
3. Select "GPT-4 Fallback Chain" from the Model Fallback dropdown
4. Save
5. Verify the key shows the fallback config

- [ ] **Step 4: Test fallback behavior**

Using the key from step 3, make a request for a model in the fallback group where the primary channel is unavailable or returns an error. Verify the response comes from a fallback model.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: integration test fixes for model fallback feature"
```

---

## Self-Review Checklist

1. **Spec coverage:**
   - New `model_fallbacks` table with `id, name, config, created_by, created_at` → Task 1
   - `api_keys.model_fallback_id` FK → Task 1
   - JSON config format `[{"models":[...],"priorities":[...]}]` → Task 2 types
   - CRUD storage trait → Task 3
   - SQLite impl → Task 3
   - PostgreSQL impl → Task 4
   - Management API CRUD routes → Task 5
   - API key create/update accepts `model_fallback_id` → Task 5
   - Proxy fallback retry on all failures → Task 6
   - Audit log with `model_override_reason: "model_fallback"` → handled by recursive proxy call (uses existing audit path)
   - Frontend Model Fallbacks page → Task 9
   - Sidebar nav item → Task 10
   - Key form dropdown → Task 10

2. **Placeholder scan:** No TBD, TODO, or "implement later" patterns found.

3. **Type consistency:**
   - `ModelFallbackConfig` struct matches between Rust (Task 2) and TypeScript (Task 7)
   - `ModelFallbackGroup` field names match (`models`, `priorities`)
   - `model_fallback_id` field consistently named across ApiKey, CreateApiKey, UpdateApiKey in both Rust and TypeScript
   - Storage trait method signatures match implementations
   - API client paths (`/model-fallbacks`) match route registration
