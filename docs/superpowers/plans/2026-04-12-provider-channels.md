# Provider Channels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor providers to support multiple channels (upstream connections with their own API key, optional base URL, and priority), and add priority-based failover in the gateway.

**Architecture:** Add a `channels` table as children of providers. Remove `api_key` from providers. The gateway tries enabled channels in priority order on upstream errors. Usage/audit records track which channel handled each request.

**Tech Stack:** Rust (axum, sqlx, SQLite), React 18 (Ant Design, React Query, TypeScript)

---

### Task 1: Add Channel types and update Provider types

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add Channel types and remove api_key from Provider**

In `crates/storage/src/types.rs`, make these changes:

1. Remove `api_key` field from `Provider` struct (line 64)
2. Remove `api_key` field from `CreateProvider` struct (line 76)
3. Remove `api_key` field from `UpdateProvider` struct (line 82)
4. Add `channel_id` field to `UsageRecord` struct (after `provider_id`, line 153)
5. Add `channel_id` field to `AuditLog` struct (after `provider_id`, line 183)
6. Add new Channel types after the Provider section (after line 87):

```rust
// --- Channels ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<Option<String>>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}
```

After changes, the Provider-related types should look like:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProvider {
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub openai_base_url: Option<Option<String>>,
    pub anthropic_base_url: Option<Option<String>>,
    pub enabled: Option<bool>,
}
```

- [ ] **Step 2: Verify compilation**

Run: `source /home/node/.cargo/env && cargo check 2>&1 | head -50`
Expected: Compilation errors in multiple files referencing `api_key` on Provider — this is expected, will be fixed in subsequent tasks.

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(types): add Channel types, remove api_key from Provider"
```

---

### Task 2: Add migration for channels table

**Files:**
- Create: `crates/storage/src/migrations/004_channels.sql`
- Modify: `crates/storage/src/migrations.rs`

- [ ] **Step 1: Create migration file**

Create `crates/storage/src/migrations/004_channels.sql`:

```sql
-- Create channels table
CREATE TABLE IF NOT EXISTS channels (
    id          TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    api_key     TEXT NOT NULL,
    base_url    TEXT,
    priority    INTEGER NOT NULL DEFAULT 0,
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_channels_provider ON channels(provider_id);

-- Migrate existing provider api_keys into default channels
INSERT INTO channels (id, provider_id, name, api_key, base_url, priority, enabled, created_at, updated_at)
SELECT lower(hex(randomblob(4))), id, 'default', api_key, NULL, 0, 1, created_at, updated_at
FROM providers WHERE api_key IS NOT NULL AND api_key != '';

-- Add channel_id to usage_records and audit_logs
ALTER TABLE usage_records ADD COLUMN channel_id TEXT REFERENCES channels(id);
ALTER TABLE audit_logs ADD COLUMN channel_id TEXT REFERENCES channels(id);

-- Remove api_key from providers
ALTER TABLE providers DROP COLUMN api_key;
```

- [ ] **Step 2: Register migration**

In `crates/storage/src/migrations.rs`, add:

```rust
pub const MIGRATION_004: &str = include_str!("migrations/004_channels.sql");

pub const ALL_MIGRATIONS: &[&str] = &[INIT_SQL, MIGRATION_002, MIGRATION_003, MIGRATION_004];
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/migrations/004_channels.sql crates/storage/src/migrations.rs
git commit -m "feat(migrations): add 004_channels migration"
```

---

### Task 3: Add Channel methods to Storage trait and SQLite implementation

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Add channel methods to Storage trait**

In `crates/storage/src/lib.rs`, add these methods after the provider section (after line 26):

```rust
// Channels
async fn create_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
async fn get_channel(&self, id: &str) -> Result<Option<Channel>, Box<dyn std::error::Error + Send + Sync>>;
async fn list_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
async fn list_enabled_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>>;
async fn update_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>>;
async fn delete_channel(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

Also update `record_usage` and `insert_log` in the trait — no signature change needed since `channel_id` is already on the structs.

- [ ] **Step 2: Add SqliteChannelRow struct and From impl**

In `crates/storage/src/sqlite.rs`, add after `SqliteProviderRow` (after line 81):

```rust
struct SqliteChannelRow {
    id: String,
    provider_id: String,
    name: String,
    api_key: String,
    base_url: Option<String>,
    priority: i32,
    enabled: i64,
    created_at: String,
    updated_at: String,
}

impl From<SqliteChannelRow> for Channel {
    fn from(r: SqliteChannelRow) -> Self {
        Channel {
            id: r.id,
            provider_id: r.provider_id,
            name: r.name,
            api_key: r.api_key,
            base_url: r.base_url,
            priority: r.priority,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}
```

- [ ] **Step 3: Update SqliteProviderRow and Provider From impl to remove api_key**

In `crates/storage/src/sqlite.rs`:

Change `SqliteProviderRow` to remove `api_key`:

```rust
struct SqliteProviderRow {
    id: String,
    name: String,
    openai_base_url: Option<String>,
    anthropic_base_url: Option<String>,
    enabled: i64,
    created_at: String,
    updated_at: String,
}
```

Update `From<SqliteProviderRow> for Provider` to remove `api_key`:

```rust
impl From<SqliteProviderRow> for Provider {
    fn from(r: SqliteProviderRow) -> Self {
        Provider {
            id: r.id,
            name: r.name,
            openai_base_url: r.openai_base_url,
            anthropic_base_url: r.anthropic_base_url,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}
```

- [ ] **Step 4: Update provider SQL queries to remove api_key**

Update all provider SQL in `sqlite.rs`:

`create_provider`:
```rust
async fn create_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
    sqlx::query(
        "INSERT INTO providers (id, name, openai_base_url, anthropic_base_url, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&provider.id)
    .bind(&provider.name)
    .bind(&provider.openai_base_url)
    .bind(&provider.anthropic_base_url)
    .bind(provider.enabled as i64)
    .bind(provider.created_at.to_rfc3339())
    .bind(provider.updated_at.to_rfc3339())
    .execute(&self.pool)
    .await?;
    Ok(provider.clone())
}
```

`get_provider`:
```rust
async fn get_provider(&self, id: &str) -> Result<Option<Provider>, DbErr> {
    let row: Option<SqliteProviderRow> = sqlx::query_as(
        "SELECT id, name, openai_base_url, anthropic_base_url, enabled, created_at, updated_at
         FROM providers WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;
    Ok(row.map(Provider::from))
}
```

`list_providers`:
```rust
async fn list_providers(&self) -> Result<Vec<Provider>, DbErr> {
    let rows: Vec<SqliteProviderRow> = sqlx::query_as(
        "SELECT id, name, openai_base_url, anthropic_base_url, enabled, created_at, updated_at
         FROM providers",
    )
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(Provider::from).collect())
}
```

`update_provider`:
```rust
async fn update_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
    sqlx::query(
        "UPDATE providers SET name = ?, openai_base_url = ?, anthropic_base_url = ?,
         enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&provider.name)
    .bind(&provider.openai_base_url)
    .bind(&provider.anthropic_base_url)
    .bind(provider.enabled as i64)
    .bind(provider.updated_at.to_rfc3339())
    .bind(&provider.id)
    .execute(&self.pool)
    .await?;
    Ok(provider.clone())
}
```

- [ ] **Step 5: Implement channel CRUD methods**

Add after the `delete_provider` method in `sqlite.rs`:

```rust
// ---- Channels ----

async fn create_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query(
        "INSERT INTO channels (id, provider_id, name, api_key, base_url, priority, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&channel.id)
    .bind(&channel.provider_id)
    .bind(&channel.name)
    .bind(&channel.api_key)
    .bind(&channel.base_url)
    .bind(channel.priority)
    .bind(channel.enabled as i64)
    .bind(channel.created_at.to_rfc3339())
    .bind(channel.updated_at.to_rfc3339())
    .execute(&self.pool)
    .await?;
    Ok(channel.clone())
}

async fn get_channel(&self, id: &str) -> Result<Option<Channel>, Box<dyn std::error::Error + Send + Sync>> {
    let row: Option<SqliteChannelRow> = sqlx::query_as(
        "SELECT id, provider_id, name, api_key, base_url, priority, enabled, created_at, updated_at
         FROM channels WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;
    Ok(row.map(Channel::from))
}

async fn list_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
    let rows: Vec<SqliteChannelRow> = sqlx::query_as(
        "SELECT id, provider_id, name, api_key, base_url, priority, enabled, created_at, updated_at
         FROM channels WHERE provider_id = ? ORDER BY priority ASC",
    )
    .bind(provider_id)
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(Channel::from).collect())
}

async fn list_enabled_channels_by_provider(&self, provider_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
    let rows: Vec<SqliteChannelRow> = sqlx::query_as(
        "SELECT id, provider_id, name, api_key, base_url, priority, enabled, created_at, updated_at
         FROM channels WHERE provider_id = ? AND enabled = 1 ORDER BY priority ASC",
    )
    .bind(provider_id)
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(Channel::from).collect())
}

async fn update_channel(&self, channel: &Channel) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query(
        "UPDATE channels SET name = ?, api_key = ?, base_url = ?, priority = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&channel.name)
    .bind(&channel.api_key)
    .bind(&channel.base_url)
    .bind(channel.priority)
    .bind(channel.enabled as i64)
    .bind(channel.updated_at.to_rfc3339())
    .bind(&channel.id)
    .execute(&self.pool)
    .await?;
    Ok(channel.clone())
}

async fn delete_channel(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(id)
        .execute(&self.pool)
        .await?;
    Ok(())
}
```

- [ ] **Step 6: Update record_usage and insert_log to include channel_id**

Update `record_usage` SQL:

```rust
async fn record_usage(&self, usage: &UsageRecord) -> Result<(), DbErr> {
    sqlx::query(
        "INSERT INTO usage_records (id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cost, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&usage.id)
    .bind(&usage.key_id)
    .bind(&usage.model_name)
    .bind(&usage.provider_id)
    .bind(&usage.channel_id)
    .bind(protocol_str(&usage.protocol))
    .bind(usage.input_tokens)
    .bind(usage.output_tokens)
    .bind(usage.cost)
    .bind(usage.created_at.to_rfc3339())
    .execute(&self.pool)
    .await?;
    Ok(())
}
```

Update `insert_log` SQL:

```rust
async fn insert_log(&self, log: &AuditLog) -> Result<(), DbErr> {
    sqlx::query(
        "INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, request_body, response_body,
         status_code, latency_ms, input_tokens, output_tokens, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&log.id)
    .bind(&log.key_id)
    .bind(&log.model_name)
    .bind(&log.provider_id)
    .bind(&log.channel_id)
    .bind(protocol_str(&log.protocol))
    .bind(&log.request_body)
    .bind(&log.response_body)
    .bind(log.status_code)
    .bind(log.latency_ms)
    .bind(log.input_tokens)
    .bind(log.output_tokens)
    .bind(&log.created_at.to_rfc3339())
    .execute(&self.pool)
    .await?;
    Ok(())
}
```

- [ ] **Step 7: Update SqliteUsageRow and SqliteAuditRow to include channel_id**

Add `channel_id: Option<String>` to both row structs, and include it in their `From` impls. Update the SELECT queries in `query_usage`, `query_usage_paginated`, `query_logs`, and `query_logs_paginated` to include `channel_id`.

- [ ] **Step 8: Verify compilation**

Run: `source /home/node/.cargo/env && cargo check 2>&1 | head -50`
Expected: Errors only in `crates/api` and `crates/audit` referencing removed `api_key`. Storage crate compiles.

- [ ] **Step 9: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/sqlite.rs
git commit -m "feat(storage): add channel CRUD, remove api_key from provider"
```

---

### Task 4: Update provider API handlers to remove api_key

**Files:**
- Modify: `crates/api/src/management/providers.rs`

- [ ] **Step 1: Update provider handlers**

In `crates/api/src/management/providers.rs`:

1. Remove `api_key` from the `Provider` construction in `create_provider` (remove line 23: `api_key: input.api_key,`)
2. Remove the `api_key` update in `update_provider` (remove lines 92-93: `if let Some(api_key) = input.api_key { provider.api_key = api_key; }`)
3. Remove the `api_key` import from `StorageCreateProvider` and `StorageUpdateProvider` usage (it's no longer on those structs)

The `create_provider` handler should construct:
```rust
let provider = Provider {
    id: uuid::Uuid::new_v4().to_string(),
    name: input.name,
    openai_base_url: input.openai_base_url,
    anthropic_base_url: input.anthropic_base_url,
    enabled: true,
    created_at: now,
    updated_at: now,
};
```

- [ ] **Step 2: Verify compilation**

Run: `source /home/node/.cargo/env && cargo check --package llm-gateway-api 2>&1 | head -50`

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/management/providers.rs
git commit -m "feat(api): remove api_key from provider handlers"
```

---

### Task 5: Add channel API handlers

**Files:**
- Create: `crates/api/src/management/channels.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: Create channel handlers**

Create `crates/api/src/management/channels.rs`:

```rust
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{Channel, CreateChannel, UpdateChannel};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(input): Json<CreateChannel>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // Verify provider exists
    state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let now = chrono::Utc::now();
    let channel = Channel {
        id: uuid::Uuid::new_v4().to_string(),
        provider_id,
        name: input.name,
        api_key: input.api_key,
        base_url: input.base_url,
        priority: input.priority.unwrap_or(0),
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_channel(&channel)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<Channel>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channels = state
        .storage
        .list_channels_by_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(channels))
}

pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    Ok(Json(channel))
}

pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateChannel>,
) -> Result<Json<Channel>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    if let Some(name) = input.name {
        channel.name = name;
    }
    if let Some(api_key) = input.api_key {
        channel.api_key = api_key;
    }
    if let Some(base_url) = input.base_url {
        channel.base_url = base_url;
    }
    if let Some(priority) = input.priority {
        channel.priority = priority;
    }
    if let Some(enabled) = input.enabled {
        channel.enabled = enabled;
    }
    channel.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_channel(&channel)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .delete_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Register channel routes**

In `crates/api/src/management/mod.rs`:

1. Add `pub mod channels;` at the top
2. Add channel routes after the provider routes:

```rust
// Channels (admin)
.route(
    "/api/v1/providers/{id}/channels",
    post(channels::create_channel).get(channels::list_channels),
)
.route(
    "/api/v1/channels/{id}",
    get(channels::get_channel).patch(channels::update_channel).delete(channels::delete_channel),
)
```

- [ ] **Step 3: Verify compilation**

Run: `source /home/node/.cargo/env && cargo check --package llm-gateway-api 2>&1 | head -50`

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/channels.rs crates/api/src/management/mod.rs
git commit -m "feat(api): add channel CRUD endpoints"
```

---

### Task 6: Update existing provider tests and add channel tests

**Files:**
- Modify: `crates/api/tests/test_management_providers.rs`

- [ ] **Step 1: Update existing provider tests to remove api_key**

In `crates/api/tests/test_management_providers.rs`:

1. In `test_create_provider`: remove `"api_key": "sk-test"` from the request body JSON
2. In `test_create_provider_dual_protocol`: remove `"api_key": "sk-test"` from the request body JSON
3. In `test_list_providers`: remove `"api_key": "sk-test"` from both request body JSONs
4. In `test_provider_model_lifecycle`: remove `"api_key": "sk-test"` from the request body JSON

- [ ] **Step 2: Add channel CRUD tests**

Add these tests to `crates/api/tests/test_management_providers.rs`:

```rust
#[tokio::test]
async fn test_create_and_list_channels() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    // Create provider (no api_key now)
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/providers")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "OpenAI",
                    "openai_base_url": "https://api.openai.com/v1"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let provider_id = body["id"].as_str().unwrap().to_string();

    // Create two channels
    for (name, priority) in [("primary", 0), ("backup", 1)] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/v1/providers/{}/channels", provider_id))
                    .header("authorization", bearer_token(&admin.token))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({
                        "name": name,
                        "api_key": "sk-test-key",
                        "priority": priority
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // List channels — should be ordered by priority
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/providers/{}/channels", provider_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body: Value = serde_json::from_slice(
        &to_bytes(list_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let channels = list_body.as_array().unwrap();
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0]["name"], "primary");
    assert_eq!(channels[1]["name"], "backup");
}

#[tokio::test]
async fn test_update_and_delete_channel() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    // Create provider and channel
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/providers")
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "TestProvider",
                    "openai_base_url": "https://example.com"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let provider_id = body["id"].as_str().unwrap().to_string();

    let ch_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/providers/{}/channels", provider_id))
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "name": "ch1",
                    "api_key": "sk-old",
                    "priority": 0
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let ch_body: Value = serde_json::from_slice(
        &to_bytes(ch_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let channel_id = ch_body["id"].as_str().unwrap().to_string();

    // Update channel
    let update_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/channels/{}", channel_id))
                .header("authorization", bearer_token(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "updated-ch", "api_key": "sk-new"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);
    let update_body: Value = serde_json::from_slice(
        &to_bytes(update_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(update_body["name"], "updated-ch");

    // Delete channel
    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/channels/{}", channel_id))
                .header("authorization", bearer_token(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);
}
```

- [ ] **Step 3: Run tests**

Run: `source /home/node/.cargo/env && cargo test --package llm-gateway-api 2>&1`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/api/tests/test_management_providers.rs
git commit -m "test: update provider tests, add channel CRUD tests"
```

---

### Task 7: Implement gateway channel failover (OpenAI)

**Files:**
- Modify: `crates/api/src/openai.rs`

- [ ] **Step 1: Add channel failover to non-streaming path**

In `crates/api/src/openai.rs`, the current non-streaming flow (lines 258-334) resolves a single provider and sends one request. Replace it with a channel failover loop.

After finding `model_entry` and `provider_id` (line 97), replace the rest of the non-streaming path:

1. Load enabled channels for the provider
2. Loop through channels, trying each one
3. On success (2xx) or client error (4xx), return immediately
4. On upstream error (5xx, timeout, connection), try next channel
5. Record usage/audit with `channel_id`

Key changes:
- After `let provider_id = &model_entry.model.provider_id;` and loading the provider, add:
```rust
let channels = state
    .storage
    .list_enabled_channels_by_provider(provider_id)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

if channels.is_empty() {
    return Err(ApiError::Internal(format!("No enabled channels for provider '{}'", provider_id)));
}
```

- Replace the single-request block with a loop:
```rust
let mut last_error = String::from("All channels failed");
let client = reqwest::Client::new();

for channel in &channels {
    let base_url = channel.base_url.as_deref()
        .unwrap_or_else(|| provider.openai_base_url.as_deref()
            .ok_or_else(|| ApiError::Internal(format!("No base URL for channel '{}'", channel.name)))?);

    let openai_provider = OpenAiProvider {
        name: channel.name.clone(),
        base_url: base_url.to_string(),
        api_key: channel.api_key.clone(),
    };

    match openai_provider.proxy_request(&client, "/v1/chat/completions", body.clone(), vec![]).await {
        Ok(proxy_result) => {
            let status = proxy_result.status_code;
            if status >= 500 || status == 0 {
                last_error = format!("Channel '{}' returned {}", channel.name, status);
                continue;
            }
            // Success or client error — return to client
            // ... spawn audit/usage with channel.id ...
            return Ok((StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY), proxy_result.response_body).into_response());
        }
        Err(e) => {
            last_error = format!("Channel '{}' error: {}", channel.name, e);
            continue;
        }
    }
}

Err(ApiError::UpstreamError(502, last_error))
```

- [ ] **Step 2: Add channel failover to streaming path**

Same pattern for the streaming block (lines 120-256). Try channels in order until one returns 200. Use the first successful channel's API key and base URL.

- [ ] **Step 3: Add channel_id to UsageRecord and AuditLog construction**

In both streaming and non-streaming paths, set `channel_id: Some(channel.id.clone())` on the `UsageRecord` and pass it to `audit_logger.log_request`.

- [ ] **Step 4: Verify compilation**

Run: `source /home/node/.cargo/env && cargo check --package llm-gateway-api 2>&1 | head -50`

- [ ] **Step 5: Run tests**

Run: `source /home/node/.cargo/env && cargo test --package llm-gateway-api 2>&1`
Expected: All existing tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/openai.rs
git commit -m "feat(gateway): add channel failover for OpenAI requests"
```

---

### Task 8: Implement gateway channel failover (Anthropic)

**Files:**
- Modify: `crates/api/src/anthropic.rs`

- [ ] **Step 1: Add channel failover to both streaming and non-streaming paths**

Same pattern as Task 7, but for the Anthropic handler in `crates/api/src/anthropic.rs`:

1. After finding `model_entry` and `provider_id`, load enabled channels
2. Loop through channels, using `channel.base_url` or `provider.anthropic_base_url`
3. On 5xx/timeout/connection error, try next channel
4. On 2xx/4xx, return to client
5. Record `channel_id` in usage/audit

- [ ] **Step 2: Verify compilation and tests**

Run: `source /home/node/.cargo/env && cargo test --package llm-gateway-api 2>&1`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/anthropic.rs
git commit -m "feat(gateway): add channel failover for Anthropic requests"
```

---

### Task 9: Update frontend types and API

**Files:**
- Modify: `web/src/types/index.ts`
- Modify: `web/src/api/providers.ts`
- Modify: `web/src/hooks/useProviders.ts`

- [ ] **Step 1: Update types**

In `web/src/types/index.ts`:

1. Remove `api_key` from `Provider` interface (line 38)
2. Remove `api_key` from `CreateProviderRequest` interface (line 48)
3. Remove `api_key` from `UpdateProviderRequest` interface (line 53)
4. Add `channel_id` to `UsageRecord` interface (after `provider_id`, line 93)
5. Add `channel_id` to `AuditLog` interface (after `provider_id`, line 113)
6. Add new Channel interfaces:

```typescript
export interface Channel {
  id: string;
  provider_id: string;
  name: string;
  api_key: string;
  base_url: string | null;
  priority: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateChannelRequest {
  name: string;
  api_key: string;
  base_url?: string | null;
  priority?: number;
}

export interface UpdateChannelRequest {
  name?: string;
  api_key?: string;
  base_url?: string | null;
  priority?: number;
  enabled?: boolean;
}
```

- [ ] **Step 2: Add channel API functions**

In `web/src/api/providers.ts`, add:

```typescript
import type { Provider, CreateProviderRequest, UpdateProviderRequest, Channel, CreateChannelRequest, UpdateChannelRequest } from '../types';

export async function listChannels(providerId: string): Promise<Channel[]> {
  const { data } = await apiClient.get<Channel[]>(`/providers/${providerId}/channels`);
  return data;
}

export async function createChannel(providerId: string, input: CreateChannelRequest): Promise<Channel> {
  const { data } = await apiClient.post<Channel>(`/providers/${providerId}/channels`, input);
  return data;
}

export async function updateChannel(id: string, input: UpdateChannelRequest): Promise<Channel> {
  const { data } = await apiClient.patch<Channel>(`/channels/${id}`, input);
  return data;
}

export async function deleteChannel(id: string): Promise<void> {
  await apiClient.delete(`/channels/${id}`);
}
```

- [ ] **Step 3: Add channel hooks**

In `web/src/hooks/useProviders.ts`, add:

```typescript
import { listChannels, createChannel as createChannelApi, updateChannel as updateChannelApi, deleteChannel as deleteChannelApi } from '../api/providers';
import type { CreateChannelRequest, UpdateChannelRequest } from '../types';

export function useChannels(providerId: string) {
  return useQuery({
    queryKey: ['providers', providerId, 'channels'],
    queryFn: () => listChannels(providerId),
    enabled: !!providerId,
  });
}

export function useCreateChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateChannelRequest) => createChannelApi(providerId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      message.success('Channel created');
    },
    onError: () => { message.error('Failed to create channel'); },
  });
}

export function useUpdateChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelRequest }) => updateChannelApi(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      message.success('Channel updated');
    },
    onError: () => { message.error('Failed to update channel'); },
  });
}

export function useDeleteChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteChannelApi(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      message.success('Channel deleted');
    },
    onError: () => { message.error('Failed to delete channel'); },
  });
}
```

- [ ] **Step 4: Verify frontend builds**

Run: `cd web && npx tsc --noEmit 2>&1 | head -20`
Expected: Errors in ProviderDetail.tsx and Providers.tsx referencing removed api_key

- [ ] **Step 5: Commit**

```bash
git add web/src/types/index.ts web/src/api/providers.ts web/src/hooks/useProviders.ts
git commit -m "feat(web): add channel types, API, and hooks"
```

---

### Task 10: Update frontend Provider pages

**Files:**
- Modify: `web/src/pages/Providers.tsx`
- Modify: `web/src/pages/ProviderDetail.tsx`

- [ ] **Step 1: Update Providers list page**

In `web/src/pages/Providers.tsx`:

1. Remove `api_key` from the create form (remove the Form.Item for api_key at lines 83-85)
2. Remove `api_key` from the `handleCreate` function (remove `api_key: values.api_key,`)
3. Remove `api_key` from the form values type

- [ ] **Step 2: Update ProviderDetail page**

In `web/src/pages/ProviderDetail.tsx`:

1. Remove the API Key Form.Item from the provider edit form (lines 143-145)
2. Remove `api_key` from `form.setFieldsValue` in `openEditModel` and from the initial values
3. Add a new "Channels" Card below the provider form with:
   - A table showing channels: Name, Base URL, Priority, Status, Actions
   - An "Add Channel" button
   - Channel CRUD using `useChannels`, `useCreateChannel`, `useUpdateChannel`, `useDeleteChannel` hooks

The channels table:

```tsx
const channelColumns = [
  { title: 'Name', dataIndex: 'name', key: 'name' },
  {
    title: 'Base URL', key: 'base_url',
    render: (v: string | null) => v || <span style={{ color: '#999' }}>Default</span>,
  },
  { title: 'Priority', dataIndex: 'priority', key: 'priority' },
  {
    title: 'Status', dataIndex: 'enabled', key: 'enabled',
    render: (enabled: boolean) => <Tag color={enabled ? 'green' : 'red'}>{enabled ? 'Active' : 'Disabled'}</Tag>,
  },
  {
    title: 'Actions', key: 'actions',
    render: (_: unknown, record: Channel) => (
      <Space>
        <a onClick={() => openEditChannel(record)}>Edit</a>
        <Popconfirm title={`Delete channel "${record.name}"?`} onConfirm={() => handleDeleteChannel(record.id)}>
          <a style={{ color: '#ff4d4f' }}>Delete</a>
        </Popconfirm>
      </Space>
    ),
  },
];
```

4. Add a channel modal with fields: name, api_key, base_url (optional), priority, enabled (edit only)

- [ ] **Step 3: Verify frontend builds and tests**

Run: `cd web && npx tsc --noEmit 2>&1 && npx vitest run 2>&1`
Expected: TypeScript compiles, tests pass

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/Providers.tsx web/src/pages/ProviderDetail.tsx
git commit -m "feat(web): update provider pages for channels"
```

---

### Task 11: Update frontend tests

**Files:**
- Modify: `web/src/test/server.ts` (if MSW handlers reference api_key on providers)
- Modify: existing test files if needed

- [ ] **Step 1: Update MSW handlers**

In `web/src/test/server.ts`, update any mock provider responses to remove `api_key` field. Add `channel_id: null` to usage/audit mock responses.

- [ ] **Step 2: Run frontend tests**

Run: `cd web && npx vitest run 2>&1`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add web/src/test/server.ts
git commit -m "test(web): update MSW handlers for provider channels"
```

---

### Task 12: Final verification

- [ ] **Step 1: Run all backend tests**

Run: `source /home/node/.cargo/env && cargo test --package llm-gateway-api 2>&1`
Expected: All tests pass, zero warnings

- [ ] **Step 2: Run all frontend tests**

Run: `cd web && npx vitest run 2>&1`
Expected: All tests pass

- [ ] **Step 3: Run TypeScript check**

Run: `cd web && npx tsc --noEmit 2>&1`
Expected: No errors
