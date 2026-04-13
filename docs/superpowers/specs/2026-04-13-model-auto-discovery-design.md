# Model Auto-Discovery Design

**Date:** 2026-04-13
**Status:** Approved

## Overview

Add on-demand model discovery from upstream providers. Admin clicks "Sync Models" button to fetch available models and save them to database.

## Problem

Currently, models must be manually created via the admin API. There is no way to automatically fetch available models from upstream providers.

## Solution

Add a "Sync Models" feature that:
1. Fetches model list from upstream provider APIs (OpenAI, Anthropic, etc.)
2. Extracts model name and type from response
3. Saves to database with upsert logic

## Changes

### 1. Database

Add `model_type` VARCHAR(100) column to models table (nullable).

```sql
ALTER TABLE models ADD COLUMN model_type VARCHAR(100);
```

### 2. Backend - Storage Types

Update `Model` struct in `crates/storage/src/types.rs`:

```rust
pub struct Model {
    pub name: String,
    pub provider_id: String,
    pub model_type: Option<String>,  // NEW: free-form string
    pub billing_type: BillingType,
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

### 3. Backend - API

Add new endpoint:
- `POST /api/v1/providers/{id}/sync-models` - trigger model sync for one provider

The endpoint:
1. Gets provider from database
2. For each protocol (OpenAI, Anthropic) with base_url:
   - Calls upstream `/models` endpoint
   - Parses response
   - Upserts model to database
3. Returns sync results (new count, updated count)

### 4. Frontend

- Add "Sync Models" button on provider detail page
- Loading state during sync
- Success/error toast with count

## Data Flow

```
Admin clicks "Sync Models"
  → POST /api/v1/providers/{id}/sync-models
  → For each protocol (OpenAI, Anthropic):
      → GET {base_url}/models
      → Parse JSON response [ {id: "gpt-4o", type: "text"}, ... ]
      → Extract name + type
  → Upsert to database:
      → INSERT if new (enabled: false)
      → UPDATE if exists
  → Return {new: N, updated: M}
```

## Sync Behavior

- **New models:** Created with `enabled: false` (require admin review before use)
- **Existing models:** Updated with new name/type if changed
- **Billing type:** Defaults to Token, admin manually adjusts later
- **Model type:** Stored free-form from upstream (nullable)

## API Request/Response

### Request

```
POST /api/v1/providers/{provider_id}/sync-models
Authorization: Bearer {admin_token}
```

### Response

```json
{
  "new": 5,
  "updated": 2,
  "models": [
    {"name": "gpt-4o", "model_type": "text", "created": true},
    {"name": "gpt-4o-mini", "model_type": "text", "created": true},
    {"name": "o1-preview", "model_type": "reasoning", "created": true}
  ]
}
```

## Acceptance Criteria

1. Admin can click "Sync Models" on provider detail page
2. Models are fetched from all enabled protocols for that provider
3. New models saved with `enabled: false`
4. Model type stored (or null if not provided)
5. Frontend shows loading state and result toast