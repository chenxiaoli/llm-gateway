# Channel Models Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add channel_models junction table for many-to-many channel-model mapping with upstream model name translation.

**Architecture:** New table with storage layer, management API, and routing integration. Falls back to existing behavior if no mapping exists.

**Tech Stack:** Rust (SQLx, Axum), SQLite migrations

---

## File Structure

### New Files:
- `crates/storage/migrations/20260414000001_add_model_id_and_channel_models.sql` - migration

### Modified Files:
- `crates/storage/src/types.rs` - ChannelModel, CreateChannelModel, UpdateChannelModel structs
- `crates/storage/src/lib.rs` - Storage trait with new methods
- `crates/storage/src/sqlite.rs` - SQLite implementation
- `crates/api/src/lib.rs` - management router include
- `crates/api/src/management/mod.rs` - include channel_models routes
- Create: `crates/api/src/management/channel_models.rs` - CRUD handlers

---

## Task 1: Add id Column to Models Table

**Files:**
- Create: `crates/storage/migrations/20260414000001_add_model_id.sql`

- [ ] **Step 1: Create migration to add id to models**

```sql
-- Add id column to models table (using name as initial value)
ALTER TABLE models ADD COLUMN id TEXT;

-- Backfill id from name
UPDATE models SET id = name WHERE id IS NULL;

-- Recreate table with id as primary key (SQLite doesn't support ALTER TABLE for primary key)
PRAGMA foreign_keys = OFF;

CREATE TABLE models_new (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    billing_type TEXT NOT NULL CHECK(billing_type IN ('token', 'request')),
    input_price REAL NOT NULL DEFAULT 0,
    output_price REAL NOT NULL DEFAULT 0,
    request_price REAL NOT NULL DEFAULT 0,
    model_type  TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);

INSERT INTO models_new (id, name, provider_id, billing_type, input_price, output_price, request_price, model_type, enabled, created_at)
SELECT id, name, provider_id, billing_type, input_price, output_price, request_price, model_type, enabled, created_at FROM models;

DROP TABLE models;
ALTER TABLE models_new RENAME TO models;

PRAGMA foreign_keys = ON;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/
git commit -m 'feat(storage): add id column to models table

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 2: Update Model Struct in Types

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add id to Model struct**

Replace `crates/storage/src/types.rs:142-153`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,           // NEW: primary key (was name)
    pub name: String,         // display name
    pub provider_id: String,
    pub model_type: Option<String>,
    pub billing_type: BillingType,
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update CreateModel**

```rust
#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub billing_type: BillingType,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
    // id will be generated from name or UUID
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m 'feat(storage): add id field to Model struct

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 3: Add ChannelModel Types

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add ChannelModel structs**

Add after Models section (around line 180):

```rust
// --- Channel Models (Junction Table) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelModel {
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelModel {
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<Option<i32>>,  // None=keep, Some(None)=clear
    pub enabled: Option<bool>,
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m 'feat(storage): add ChannelModel types

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 4: Create channel_models Migration

**Files:**
- Create: `crates/storage/migrations/20260414000002_create_channel_models.sql`

- [ ] **Step 1: Create migration**

```sql
CREATE TABLE channel_models (
    id                  TEXT PRIMARY KEY,
    channel_id          TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    model_id            TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    upstream_model_name TEXT NOT NULL,
    priority_override   INTEGER,
    enabled             INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(channel_id, model_id)
);

CREATE INDEX idx_channel_models_model ON channel_models(model_id);
CREATE INDEX idx_channel_models_channel ON channel_models(channel_id);
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/
git commit -m 'feat(storage): create channel_models table

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 5: Update Storage Trait

**Files:**
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: Add ChannelModel methods to Storage trait**

Add after model-related methods:

```rust
// --- Channel Models ---

async fn create_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>>;
async fn get_channel_model(&self, id: &str) -> Result<Option<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
async fn list_channel_models(&self) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
async fn list_channel_models_by_channel(&self, channel_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
async fn get_channel_models_for_model(&self, model_id: &str) -> Result<Vec<ChannelModel>, Box<dyn std::error::Error + Send + Sync>>;
async fn get_channels_for_model(&self, model_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
async fn update_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, Box<dyn std::error::Error + Send + Sync>>;
async fn delete_channel_model(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m 'feat(storage): add ChannelModel methods to Storage trait

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 6: Implement SQLite Storage for ChannelModel

**Files:**
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Add SqliteChannelModelRow struct**

```rust
#[derive(FromRow)]
struct SqliteChannelModelRow {
    id: String,
    channel_id: String,
    model_id: String,
    upstream_model_name: String,
    priority_override: Option<i64>,
    enabled: i64,
    created_at: String,
    updated_at: String,
}

impl From<SqliteChannelModelRow> for ChannelModel {
    fn from(r: SqliteChannelModelRow) -> Self {
        ChannelModel {
            id: r.id,
            channel_id: r.channel_id,
            model_id: r.model_id,
            upstream_model_name: r.upstream_model_name,
            priority_override: r.priority_override,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}
```

- [ ] **Step 2: Implement Storage trait methods**

Implement all 8 methods:
- create_channel_model
- get_channel_model
- list_channel_models
- list_channel_models_by_channel
- get_channel_models_for_model
- get_channels_for_model (joins channel_models with channels)
- update_channel_model
- delete_channel_model

- [ ] **Step 3: Run tests**

Run: `cargo check --package llm-gateway-storage`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m 'feat(storage): implement ChannelModel SQLite methods

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 7: Create ChannelModel Management API

**Files:**
- Create: `crates/api/src/management/channel_models.rs`

- [ ] **Step 1: Create handlers**

```rust
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{ChannelModel, CreateChannelModel, UpdateChannelModel};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn create_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(input): Json<CreateChannelModel>,
) -> Result<Json<ChannelModel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Verify channel belongs to provider
    let channel = state.storage.get_channel(&input.channel_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound('Channel not found'.to_string()))?;
    
    if channel.provider_id != provider_id {
        return Err(ApiError::BadRequest('Channel does not belong to provider'.to_string()));
    }

    // Verify model exists
    let model = state.storage.get_model(&input.model_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound('Model not found'.to_string()))?;

    let now = chrono::Utc::now();
    let cm = ChannelModel {
        id: uuid::Uuid::new_v4().to_string(),
        channel_id: input.channel_id,
        model_id: input.model_id,
        upstream_model_name: input.upstream_model_name,
        priority_override: input.priority_override,
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let created = state.storage.create_channel_model(&cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_channel_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<ChannelModel>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Get channels for provider, then channel_models for those channels
    let channels = state.storage.list_channels_by_provider(&provider_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    
    let channel_ids: Vec<String> = channels.iter().map(|c| c.id.clone()).collect();
    
    // Get all channel_models for these channels
    let mut all_models = Vec::new();
    for channel_id in channel_ids {
        let cms = state.storage.list_channel_models_by_channel(&channel_id).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        all_models.extend(cms);
    }

    Ok(Json(all_models))
}

pub async fn get_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ChannelModel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let cm = state.storage.get_channel_model(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound('ChannelModel not found'.to_string()))?;

    Ok(Json(cm))
}

pub async fn update_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateChannelModel>,
) -> Result<Json<ChannelModel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut cm = state.storage.get_channel_model(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound('ChannelModel not found'.to_string()))?;

    if let Some(upstream) = input.upstream_model_name {
        cm.upstream_model_name = upstream;
    }
    if let Some(priority) = input.priority_override {
        cm.priority_override = priority;
    }
    if let Some(enabled) = input.enabled {
        cm.enabled = enabled;
    }
    cm.updated_at = chrono::Utc::now();

    let updated = state.storage.update_channel_model(&cm).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_channel_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state.storage.delete_channel_model(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Register routes in management/mod.rs**

Add:

```rust
pub mod channel_models;

pub fn channel_models_router() -> Router {
    Router::new()
        .route('/providers/{provider_id}/channel-models', post(channel_models::create_channel_model).get(channel_models::list_channel_models))
        .route('/channel-models/{id}', get(channel_models::get_channel_model).patch(channel_models::update_channel_model).delete(channel_models::delete_channel_model))
}
```

- [ ] **Step 3: Include in main router**

In `crates/api/src/management/mod.rs`, add `.merge(channel_models_router())`.

- [ ] **Step 4: Run tests**

Run: `cargo check --package llm-gateway-api`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/channel_models.rs crates/api/src/management/mod.rs
git commit -m 'feat(api): add ChannelModel management endpoints

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 8: Integrate ChannelModels into Routing

**Files:**
- Modify: `crates/api/src/openai.rs`
- Modify: `crates/api/src/anthropic.rs`

- [ ] **Step 1: Update openai.rs routing**

In `chat_completions`, before fetching channels:

```rust
// Get channels that support this model (via channel_models)
// If no mapping exists, fall back to all provider channels
let channels = match state.storage.get_channels_for_model(&model_name).await {
    Ok(channels) if !channels.is_empty() => channels,
    _ => {
        // Fallback: use all enabled channels for provider
        state.storage.list_enabled_channels_by_provider(provider_id).await
            .map_err(|e| ApiError::Internal(e.to_string()))?
    }
};
```

- [ ] **Step 2: Get upstream model name**

When making the upstream call, look up the upstream_model_name:

```rust
// Get channel_models to find upstream_model_name
let channel_models = state.storage.get_channel_models_for_model(&model_name).await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

let upstream_name = channel_models
    .iter()
    .find(|cm| cm.channel_id == channel.id)
    .map(|cm| cm.upstream_model_name.as_str())
    .unwrap_or(&model_name);
```

- [ ] **Step 3: Use upstream_name in requests**

Replace model_name with upstream_name in:
- URL construction for OpenAI
- Request body modifications

- [ ] **Step 4: Same changes for anthropic.rs**

- [ ] **Step 5: Run tests**

Run: `cargo check --package llm-gateway-api`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/openai.rs crates/api/src/anthropic.rs
git commit -m 'feat(api): integrate channel_models into routing

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 9: Update KeyModelRateLimits References

**Files:**
- Modify: `crates/storage/src/types.rs`
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Update KeyModelRateLimit to use model_id**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyModelRateLimit {
    pub key_id: String,
    pub model_id: String,  // Changed from model_name
    pub rpm: i64,
    pub tpm: i64,
}
```

- [ ] **Step 2: Update SQLite implementations**

Update all references from model_name to model_id in:
- create_key_model_rate_limit
- get_key_model_rate_limit
- list_key_model_rate_limits
- delete_key_model_rate_limit

- [ ] **Step 3: Run tests**

Run: `cargo check --package llm-gateway-storage`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/sqlite.rs
git commit -m 'refactor(storage): use model_id in KeyModelRateLimit

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Summary

This plan adds 9 tasks covering:
1. Add id column to models table
2. Update Model struct in types
3. Add ChannelModel types
4. Create channel_models migration
5. Update Storage trait
6. Implement SQLite methods
7. Management API endpoints
8. Routing integration
9. KeyModelRateLimit references

**Plan complete and saved to `docs/superpowers/plans/2026-04-14-channel-models.md`.**

Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?