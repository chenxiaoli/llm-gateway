# Channel Models Design Spec

**Date:** 2026-04-14

## Overview

Add a `channel_models` junction table to implement many-to-many relationship between channels and models. This enables flexible routing with model-specific upstream names and per-model priority overrides.

## Background

The current architecture has a gap:
- Channel → Model is one-to-many (a channel can only access one model)
- This prevents load balancing and failover across multiple channels for the same model

The new design:
- Model ↔ Channel is many-to-many via `channel_models` junction table
- Each entry maps a channel to a model, with the actual upstream model name
- Enables "gpt-4o" → multiple channels (load balancing) with different upstream names

## Data Model

### Table: channel_models

```sql
CREATE TABLE channel_models (
    id                  TEXT PRIMARY KEY,
    channel_id          TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    model_id            TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    upstream_model_name TEXT NOT NULL,  -- actual name sent to upstream (e.g., "my-gpt4o-deploy")
    priority_override   INTEGER,          -- NULL = use channel.priority, otherwise override
    enabled             BOOLEAN NOT NULL DEFAULT 1,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(channel_id, model_id)
);

CREATE INDEX idx_channel_models_model ON channel_models(model_name);
CREATE INDEX idx_channel_models_channel ON channel_models(channel_id);
```

### Rust Struct

```rust
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

## Routing Logic

### Flow: Request to upstream

```
1. Client requests: model = "gpt-4o"

2. Query channel_models:
   SELECT * FROM channel_models 
   WHERE model_id = (SELECT id FROM models WHERE name = "gpt-4o") AND enabled = true

3. Get channel_ids from results

4. Query channels:
   SELECT * FROM channels 
   WHERE id IN (...) AND enabled = true
   ORDER BY COALESCE(priority_override, priority) ASC

5. For each channel in priority order:
   - Get upstream_model_name from channel_models entry
   - Make upstream request with upstream_model_name
   - On failure, try next channel

6. Fallback (no channel_models entry):
   - Use all enabled channels for the provider
   - Use original model_name
```

### Key Changes

**In openai.rs and anthropic.rs:**

```rust
// Before: list all channels for provider
let channels = state.storage.list_enabled_channels_by_provider(provider_id).await?;

// After: filter to channels that support this model
let channel_models = state.storage.get_channel_models_for_model(&model_name).await?;
let channel_ids: Vec<String> = channel_models.iter().map(|cm| cm.channel_id.clone()).collect();
let channels = if channel_ids.is_empty() {
    // Fallback to all channels
    state.storage.list_enabled_channels_by_provider(provider_id).await?
} else {
    state.storage.get_channels_by_ids(&channel_ids).await?
};
```

## API Endpoints

### Create Channel Model

```
POST /management/providers/{provider_id}/channel-models
{
    "channel_id": "uuid",
    "model_id": "uuid",
    "upstream_model_name": "my-gpt4o-deploy",
    "priority_override": 5  // optional
}
```

### List Channel Models

```
GET /management/providers/{provider_id}/channel-models
```

### Get Channel Model

```
GET /management/channel-models/{id}
```

### Update Channel Model

```
PATCH /management/channel-models/{id}
{
    "upstream_model_name": "new-deployment-name",
    "enabled": false
}
```

### Delete Channel Model

```
DELETE /management/channel-models/{id}
```

## Storage Layer

### New trait methods:

```rust
trait Storage {
    // ... existing methods ...

    async fn create_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr>;
    async fn get_channel_model(&self, id: &str) -> Result<Option<ChannelModel>, DbErr>;
    async fn list_channel_models(&self) -> Result<Vec<ChannelModel>, DbErr>;
    async fn list_channel_models_by_provider(&self, provider_id: &str) -> Result<Vec<ChannelModel>, DbErr>;
    async fn get_channel_models_for_model(&self, model_name: &str) -> Result<Vec<ChannelModel>, DbErr>;
    async fn get_channels_for_model(&self, model_name: &str) -> Result<Vec<Channel>, DbErr>;
    async fn update_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr>;
    async fn delete_channel_model(&self, id: &str) -> Result<(), DbErr>;
}
```

## Frontend

### Provider Detail Page

Add "Channel Models" tab showing:
- Model name
- Channel name
- Upstream name
- Priority override
- Enabled toggle

### Channel Model Form

Fields:
- Model (dropdown of available models)
- Channel (dropdown of provider's channels)
- Upstream model name (text input)
- Priority override (optional number)

## Testing

1. **Create mapping**: Create channel_models entry, verify in DB
2. **List mappings**: GET returns correct entries
3. **Routing with mapping**: Request uses upstream_model_name
4. **Routing fallback**: No mapping → uses all channels with original name
5. **Priority override**: Higher priority_override picks first
6. **Disable mapping**: Disabled → skipped in routing

## Migration

### Step 1: Add id to models table

```sql
ALTER TABLE models ADD COLUMN id TEXT PRIMARY KEY;

-- Backfill id from name for existing rows
UPDATE models SET id = name WHERE id IS NULL;

-- Make id not nullable for new inserts
PRAGMA foreign_keys = OFF;
CREATE TABLE models_new (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    billing_type TEXT NOT NULL CHECK(billing_type IN ('token', 'request')),
    input_price REAL NOT NULL DEFAULT 0,
    output_price REAL NOT NULL DEFAULT 0,
    request_price REAL NOT NULL DEFAULT 0,
    model_type  TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);
INSERT INTO models_new SELECT id, name, provider_id, billing_type, input_price, output_price, request_price, model_type, enabled, created_at FROM models;
DROP TABLE models;
ALTER TABLE models_new RENAME TO models;
PRAGMA foreign_keys = ON;
```

### Step 2: Update foreign key references

```sql
-- Update key_model_rate_limits
ALTER TABLE key_model_rate_limits DROP FOREIGN KEY key_model_rate_limits_ibfk_1;
ALTER TABLE key_model_rate_limits CHANGE model_name model_id TEXT REFERENCES models(id);

-- Update channel_models (new table)
```

### Step 3: Create channel_models

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