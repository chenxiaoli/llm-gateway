# Provider Channels Design

**Date:** 2026-04-12
**Status:** Approved

## Overview

Refactor the provider system so that one provider can have multiple channels. A channel is an upstream connection with its own name, API key, optional base URL override, and priority. When proxying requests, the gateway tries channels in priority order, falling back on upstream errors (5xx, timeout).

## Motivation

Currently each provider holds a single API key and base URL. In practice, users often have multiple accounts with the same provider (e.g., multiple OpenAI keys for budget distribution or failover). The current model requires creating duplicate provider entries, which duplicates base URL configuration and confuses model assignment.

## Data Model

### New `channels` table

```sql
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
CREATE INDEX idx_channels_provider ON channels(provider_id);
```

- `base_url`: Optional override. When NULL, the provider's `openai_base_url` or `anthropic_base_url` is used (depending on protocol).
- `priority`: Lower value = higher priority. Channels are tried in ascending priority order during failover.
- `enabled`: Disabled channels are skipped during failover.

### Provider changes

Remove `api_key` column from `providers` table. Provider becomes a "service definition" holding:
- `name` (e.g., "OpenAI", "Anthropic")
- `openai_base_url` — default for OpenAI-compatible requests
- `anthropic_base_url` — default for Anthropic-compatible requests
- `enabled`

### Channel struct

```rust
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

pub struct CreateChannel {
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: Option<i32>,
}

pub struct UpdateChannel {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<Option<String>>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}
```

### Model relationship unchanged

Models still have `provider_id` referencing the provider. No change to the models table.

### Usage and audit log changes

Add `channel_id` column to `usage_records` and `audit_logs` tables to track which channel handled each request.

## API Endpoints

### Channel CRUD (admin only)

**`POST /api/v1/providers/:provider_id/channels`**
- Request: `{ "name": string, "api_key": string, "base_url"?: string, "priority"?: number }`
- Response: created `Channel`
- Default priority is 0

**`GET /api/v1/providers/:provider_id/channels`**
- Response: array of `Channel`, ordered by `priority ASC`

**`GET /api/v1/channels/:id`**
- Response: `Channel`

**`PATCH /api/v1/channels/:id`**
- Request: partial `{ "name"?, "api_key"?, "base_url"?, "priority"?, "enabled"? }`
- Response: updated `Channel`

**`DELETE /api/v1/channels/:id`**
- Returns 204

### Provider changes

- Remove `api_key` from `CreateProvider`, `UpdateProvider`, and the `Provider` response struct
- All other provider endpoints unchanged

## Gateway Failover

When a request arrives for a model:

1. Find the model in storage, get `provider_id`
2. Load the provider, verify it is `enabled`
3. Load all enabled channels for that provider, ordered by `priority ASC`
4. If no channels exist, return 502
5. Iterate through channels:
   - Resolve base URL: `channel.base_url` if set, otherwise `provider.openai_base_url` or `provider.anthropic_base_url` (depending on protocol)
   - Send the upstream request using `channel.api_key`
   - On success (2xx) or client error (4xx): return response to client, record `channel_id` in usage/audit
   - On upstream error (5xx, timeout, connection failure): log the error, try next channel
6. If all channels fail: return 502 Bad Gateway with the last error message

**No retry** — each channel is tried at most once per request. Retry across requests is the client's responsibility.

**Streaming:** If the first attempted channel returns a streaming response (200 with SSE), the stream is forwarded to the client. If a channel fails before streaming starts, the next channel is tried.

## Migration

```sql
-- 004_channels.sql

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
CREATE INDEX idx_channels_provider ON channels(provider_id);

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

## Frontend

### Provider detail page

- Remove API Key field from provider edit form
- Add "Channels" section with a table:
  - Columns: Name, Base URL (shows "Default" when empty), Priority, Status, Actions (Edit/Delete)
  - "Add Channel" button opens a modal with: name, api_key, base_url (optional), priority
  - Edit channel modal same fields + enabled toggle

### Provider list page

No visual change. Columns: Name, Protocols, Status, Created.

### Types

```typescript
interface Channel {
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

interface CreateChannelRequest {
  name: string;
  api_key: string;
  base_url?: string;
  priority?: number;
}

interface UpdateChannelRequest {
  name?: string;
  api_key?: string;
  base_url?: string | null;
  priority?: number;
  enabled?: boolean;
}
```

## Files to modify

**Backend:**
- `crates/storage/src/types.rs` — add Channel, CreateChannel, UpdateChannel; remove api_key from Provider/CreateProvider/UpdateProvider
- `crates/storage/src/lib.rs` — add channel CRUD methods to Storage trait
- `crates/storage/src/sqlite.rs` — implement channel storage
- `crates/storage/src/migrations.rs` — add 004_channels.sql
- `crates/api/src/management/providers.rs` — remove api_key handling, add channel CRUD handlers
- `crates/api/src/management/mod.rs` — add channel routes
- `crates/api/src/openai.rs` — implement channel failover loop
- `crates/api/src/anthropic.rs` — implement channel failover loop

**Frontend:**
- `web/src/types/index.ts` — add Channel types, remove api_key from Provider
- `web/src/api/providers.ts` — add channel CRUD API functions
- `web/src/hooks/useProviders.ts` — add channel hooks
- `web/src/pages/ProviderDetail.tsx` — remove api_key field, add channels section
- `web/src/pages/Providers.tsx` — no change needed

**Tests:**
- `crates/api/tests/test_management_providers.rs` — update for removed api_key, add channel CRUD tests
- `crates/api/tests/test_auth.rs` — no change
- `web/src/pages/Providers.test.tsx` — update if needed
