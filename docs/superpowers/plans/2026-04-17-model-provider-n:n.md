# Model-Provider N:N Relationship Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Change Model-Provider from N:1 to N:N, use ChannelModel as sole routing source, Model retains default pricing with ChannelModel override.

**Architecture:** Remove provider_id from Model, update routing to use ChannelModel only for channel lookup, add fallback to provider channels when no ChannelModel exists.

**Tech Stack:** Rust, SQLite, PostgreSQL, sqlx

---

## File Structure

```
crates/storage/src/types.rs       # Model struct (remove provider_id)
crates/storage/src/sqlite.rs     # Storage impl (migrations, queries)
crates/storage/src/postgres.rs   # Storage impl (migrations, queries)
crates/api/src/proxy.rs          # Routing logic (use ChannelModel)
crates/api/src/models.rs        # list_models endpoint
crates/api/src/management/models.rs  # Model CRUD
```

---

## Task 1: Update Model Struct

**Files:**
- Modify: `crates/storage/src/types.rs:152-164`

- [ ] **Step 1: Remove provider_id from Model struct**

```rust
// In types.rs, Model struct (line 152)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub model_type: Option<String>,
    pub billing_type: String,
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    // Removed: provider_id: String,
    // Removed: pricing_policy_id: Option<String>,
}
```

- [ ] **Step 2: Run cargo check to verify types compile**

```bash
cd /workspace && cargo check -p llm_gateway_storage
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "refactor: remove provider_id from Model struct"
```

---

## Task 2: Create SQLite Migration

**Files:**
- Create: `crates/storage/migrations/sqlite/20260417000000_model_remove_provider_id.sql`

- [ ] **Step 1: Create migration file**

```sql
-- SQLite migration: Remove provider_id from models table
ALTER TABLE models DROP COLUMN provider_id;
ALTER TABLE models DROP COLUMN pricing_policy_id;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/sqlite/20260417000000_model_remove_provider_id.sql
git commit -m "migrations(sqlite): remove provider_id from models"
```

---

## Task 3: Create PostgreSQL Migration

**Files:**
- Create: `crates/storage/migrations/postgres/20260417000000_model_remove_provider_id.sql`

- [ ] **Step 1: Create migration file**

```sql
-- PostgreSQL migration: Remove provider_id from models table
ALTER TABLE models DROP COLUMN IF EXISTS provider_id;
ALTER TABLE models DROP COLUMN IF EXISTS pricing_policy_id;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/postgres/20260417000000_model_remove_provider_id.sql
git commit -m "migrations(postgres): remove provider_id from models"
```

---

## Task 4: Update SQLite Storage

**Files:**
- Modify: `crates/storage/src/sqlite.rs` (Model queries and row types)
- Test: `crates/storage/src/sqlite.rs` (implicit - cargo check)

- [ ] **Step 1: Update SqliteModelRow struct** (find and remove provider_id, pricing_policy_id)

```rust
// In sqlite.rs, find SqliteModelRow struct and remove:
// pub provider_id: String,
// pub pricing_policy_id: Option<String>,
```

- [ ] **Step 2: Update list_models query** (remove provider_id from SELECT)

In the `list_models` function's SQL query, remove `provider_id` and `pricing_policy_id` from SELECT.

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check -p llm_gateway_storage
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "refactor(sqlite): update Model queries for N:N"
```

---

## Task 5: Update PostgreSQL Storage

**Files:**
- Modify: `crates/storage/src/postgres.rs` (Model queries and row types)
- Test: `crates/storage/src/postgres.rs` (implicit - cargo check)

- [ ] **Step 1: Update PgModelRow struct** (find and remove provider_id, pricing_policy_id)

```rust
// In postgres.rs, find PgModelRow struct and remove:
// pub provider_id: String,
// pub pricing_policy_id: Option<String>,
```

- [ ] **Step 2: Update list_models query** (remove provider_id from SELECT)

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check -p llm_gateway_storage
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "refactor(postgres): update Model queries for N:N"
```

---

## Task 6: Update Proxy Routing Logic

**Files:**
- Modify: `crates/api/src/proxy.rs:57-115`
- Test: `crates/api/src/proxy.rs` (implicit - cargo check)

**Current Logic (Problem):**
```rust
// Uses model.provider_id to find provider
let provider_id = model_entry.model.provider_id.clone();
let provider = state.storage.get_provider(&provider_id).await?;

// Falls back to provider channels
let channels = match state.storage.get_channels_for_model(&model_name).await {
    Ok(channels) if !channels.is_empty() => channels,
    _ => state.storage.list_enabled_channels_by_provider(&provider_id).await?,
};
```

**New Logic:**
```rust
// Step 1: Get channel_models for this model
let channel_models = state.storage.get_channel_models_for_model(&model_name).await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

// Step 2: Get channels that have this model enabled
let channels: Vec<Channel> = channel_models
    .iter()
    .filter(|cm| cm.enabled)
    .filter_map(|cm| {
        // Get channel by channel_id from channel_models
        // This requires a new method or filter from all channels
    })
    .collect();

// Step 3: If no channels via ChannelModel, return error
if channels.is_empty() {
    return Err(ApiError::NotFound(format!("Model '{}' not available on any channel", model_name)));
}
```

**Note:** You may need to add a new storage method: `list_channels_by_channel_models`.

- [ ] **Step 1: Update routing to use ChannelModel only**

Replace the current routing logic:
```rust
// OLD: Uses provider_id and falls back to provider channels
// NEW: Uses ChannelModel only

// Get all ChannelModel records for this model
let channel_models = state.storage.get_channel_models_for_model(&model_name).await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

if channel_models.is_empty() {
    return Err(ApiError::NotFound(format!(
        "Model '{}' not available on any channel. Add ChannelModel first.", 
        model_name
    )));
}

// Filter enabled ChannelModels and get their channels
let mut available_channels = Vec::new();
let mut last_error = String::new();

for cm in channel_models.iter().filter(|cm| cm.enabled) {
    // Get channel from storage
    match state.storage.get_channel(&cm.channel_id).await {
        Ok(Some(channel)) if channel.enabled => {
            available_channels.push((channel, cm));
        }
        Ok(Some(_)) => {} // channel disabled
        Ok(None) => {} // channel not found
        Err(e) => last_error = e.to_string(),
    }
}

if available_channels.is_empty() {
    return Err(ApiError::NotFound(format!("Model '{}' not available on any enabled channel", model_name)));
}

// Sort by priority (lower = higher priority)
available_channels.sort_by(|a, b| {
    let priority_a = a.1.priority_override.unwrap_or(0);
    let priority_b = b.1.priority_override.unwrap_or(0);
    priority_a.cmp(&priority_b)
});

// Use first available channel
let (channel, channel_model) = available_channels.remove(0);
```

- [ ] **Step 2: Update pricing logic to use ChannelModel override > Model default**

```rust
// Get pricing: override > default
let billing_type = channel_model.billing_type.clone().unwrap_or_else(|| model_entry.model.billing_type.clone());
let input_price = channel_model.input_price.unwrap_or(model_entry.model.input_price);
let output_price = channel_model.output_price.unwrap_or(model_entry.model.output_price);
let request_price = channel_model.request_price.unwrap_or(model_entry.model.request_price);
```

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check --workspace
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "refactor: use ChannelModel as sole routing source"
```

---

## Task 7: Update models.rs list_models Endpoint

**Files:**
- Modify: `crates/api/src/models.rs`
- Test: `crates/api/src/models.rs` (implicit - cargo check)

- [ ] **Step 1: Update list_models to filter by ChannelModel**

```rust
// In models.rs, update list_models function
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    // Auth check
    let raw_token = extract_bearer_token(&headers)?;
    let token_hash = hash_api_key(&raw_token);
    let _api_key = state
        .storage
        .get_key_by_hash(&token_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    // Get all models
    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Filter: only models that have at least one enabled ChannelModel
    let mut result = Vec::new();
    for m in models {
        let channel_models = state.storage.get_channel_models_for_model(&m.model.name).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        
        if channel_models.iter().any(|cm| cm.enabled) {
            result.push(m);
        }
    }

    // Convert to OpenAI format
    let openai_models: Vec<Value> = result
        .iter()
        .map(|m| {
            json!({
                "id": m.model.name,
                "object": "model",
                "created": m.model.created_at.timestamp(),
                "owned_by": m.provider.name,
            })
        })
        .collect();

    Ok(Json(json!({
        "object": "list",
        "data": openai_models,
    })))
}
```

- [ ] **Step 2: Run cargo check**

```bash
cd /workspace && cargo check --workspace
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/models.rs
git commit -m "refactor: list_models filters by ChannelModel"
```

---

## Task 8: Update Management API Models

**Files:**
- Modify: `crates/api/src/management/models.rs`
- Test: `crates/api/src/management/models.rs` (implicit - cargo check)

- [ ] **Step 1: Update Model CRUD to remove provider_id**

Update create_model, update_model functions to remove provider_id field.

- [ ] **Step 2: Run cargo check**

```bash
cd /workspace && cargo check --workspace
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/management/models.rs
git commit -m "refactor: remove provider_id from Model CRUD"
```

---

## Implementation Order

1. Task 1: Update Model struct (types.rs)
2. Task 2: SQLite migration
3. Task 3: PostgreSQL migration
4. Task 4: Update SQLite storage
5. Task 5: Update PostgreSQL storage
6. Task 6: Update proxy routing logic
7. Task 7: Update models.rs list_models
8. Task 8: Update management API

---

## Verification

After all tasks, run:
```bash
cargo test --workspace
cargo build --release
```

All tests should pass and build should succeed.