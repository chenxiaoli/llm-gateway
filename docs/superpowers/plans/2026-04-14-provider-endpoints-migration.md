# Provider Endpoints Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `openai_base_url` and `anthropic_base_url` provider fields with unified `base_url` and `endpoints` (JSON) fields.

**Architecture:** Add database migration to restructure provider table, update storage types, modify API handlers to read endpoints from JSON, update frontend forms.

**Tech Stack:** Rust (storage + API), TypeScript (frontend), SQLite

---

### Task 1: Add database migration for endpoints field

**Files:**
- Create: `crates/storage/migrations/20260415000000_provider_endpoints.sql`
- Modify: `crates/storage/src/sqlite.rs:run_migrations`

- [ ] **Step 1: Create migration file**

```sql
-- Migration: Add base_url and endpoints to providers table
-- Run this after existing migrations

-- Add new columns
ALTER TABLE providers ADD COLUMN base_url TEXT;
ALTER TABLE providers ADD COLUMN endpoints TEXT;

-- Migrate existing URLs to endpoints JSON
-- Note: This is a simplified migration. Adjust based on actual data.
UPDATE providers SET endpoints = 
    CASE 
        WHEN openai_base_url IS NOT NULL OR anthropic_base_url IS NOT NULL
        THEN '{"openai":"' || COALESCE(openai_base_url, '') || '","anthropic":"' || COALESCE(anthropic_base_url, '') || '"}'
        ELSE NULL
    END
WHERE openai_base_url IS NOT NULL OR anthropic_base_url IS NOT NULL;

-- Set base_url as fallback (use openai_base_url as default)
UPDATE providers SET base_url = openai_base_url WHERE openai_base_url IS NOT NULL;
```

- [ ] **Step 2: Update run_migrations to include new migration**

In `crates/storage/src/sqlite.rs`, add the new migration to the migrations list.

---

### Task 2: Update storage types

**Files:**
- Modify: `crates/storage/src/types.rs:71-94`
- Modify: `crates/storage/src/sqlite.rs:120-145`

- [ ] **Step 1: Update Provider struct in types.rs**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: Option<String>,           // NEW: single fallback URL
    pub endpoints: Option<String>,          // NEW: JSON {"openai": "...", "anthropic": "..."}
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update SqliteProviderRow struct in sqlite.rs**

```rust
#[derive(sqlx::FromRow)]
struct SqliteProviderRow {
    id: String,
    name: String,
    base_url: Option<String>,
    endpoints: Option<String>,
    enabled: bool,
    created_at: String,
    updated_at: String,
}
```

- [ ] **Step 3: Update from_row implementation**

```rust
impl From<SqliteProviderRow> for Provider {
    fn from(r: SqliteProviderRow) -> Self {
        Provider {
            id: r.id,
            name: r.name,
            base_url: r.base_url,
            endpoints: r.endpoints,
            enabled: r.enabled,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}
```

- [ ] **Step 4: Update CreateProvider and UpdateProvider types**

```rust
#[derive(Debug, Deserialize)]
pub struct CreateProvider {
    pub name: String,
    pub base_url: Option<String>,
    pub endpoints: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub base_url: Option<Option<String>>,
    pub endpoints: Option<Option<String>>,
    pub enabled: Option<bool>,
}
```

- [ ] **Step 5: Update SQLite queries**

In `crates/storage/src/sqlite.rs`, update:
- INSERT query (line ~567): Add `base_url, endpoints` to columns
- SELECT query (line ~585): Add columns
- UPDATE query (line ~608): Add columns

---

### Task 3: Update API types

**Files:**
- Modify: `crates/api/src/management/providers.rs`

- [ ] **Step 1: Update create/update request handlers**

Change input types to accept `base_url` and `endpoints` (JSON string).

---

### Task 4: Update gateway handlers

**Files:**
- Modify: `crates/api/src/openai.rs:208-220`
- Modify: `crates/api/src/anthropic.rs` (similar location)

- [ ] **Step 1: Update openai.rs to read from endpoints JSON**

```rust
// Before:
let base_url = provider.openai_base_url.as_ref()
    .ok_or_else(|| ...)?;

// After:
let endpoints: serde_json::Value = provider.endpoints
    .as_ref()
    .and_then(|e| serde_json::from_str(e).ok())
    .unwrap_or(serde_json::Value::Null);
let base_url = endpoints.get("openai")
    .and_then(|v| v.as_str())
    .or(provider.base_url.as_deref())
    .ok_or_else(|| ApiError::Internal(format!("Provider '{}' has no OpenAI endpoint", provider_id)))?;
```

- [ ] **Step 2: Update anthropic.rs similarly**

```rust
let base_url = endpoints.get("anthropic")
    .and_then(|v| v.as_str())
    .or(provider.base_url.as_deref())
    .ok_or_else(|| ApiError::Internal(format!("Provider '{}' has no Anthropic endpoint", provider_id)))?;
```

---

### Task 5: Update frontend types

**Files:**
- Modify: `web/src/types/index.ts:35-56`

- [ ] **Step 1: Update Provider interface**

```typescript
export interface Provider {
  id: string;
  name: string;
  base_url: string | null;
  endpoints: string | null;  // JSON string
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProviderRequest {
  name: string;
  base_url?: string | null;
  endpoints?: string | null;
}

export interface UpdateProviderRequest {
  name?: string;
  base_url?: string | null;
  endpoints?: string | null;
  enabled?: boolean;
}
```

---

### Task 6: Update frontend Settings page

**Files:**
- Modify: `web/src/pages/Settings.tsx`

- [ ] **Step 1: Update provider form inputs**

Replace `openai_base_url` and `anthropic_base_url` inputs with:
- `base_url` input (single fallback URL)
- `endpoints` JSON editor or two inputs (openai, anthropic)

```typescript
const [provBaseUrl, setProvBaseUrl] = useState('');
const [provOpenaiUrl, setProvOpenaiUrl] = useState('');
const [provAnthropicUrl, setProvAnthropicUrl] = useState('');

// Build endpoints JSON
const endpoints = JSON.stringify({
  openai: provOpenaiUrl || null,
  anthropic: provAnthropicUrl || null
});
```

- [ ] **Step 2: Update provider card display**

Show endpoints as formatted JSON or separate fields.

---

### Task 7: Verify and test

**Files:**
- Run: Backend tests
- Run: Frontend tests

- [ ] **Step 1: Run backend tests**

```bash
cargo test --manifest-path /workspace/Cargo.toml
```

Expected: All tests pass

- [ ] **Step 2: Run frontend tests**

```bash
npm test -- --run
```

Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add provider endpoints JSON field"
```

---

## Summary

| Task | Description |
|------|--------------|
| 1 | Database migration |
| 2 | Storage types (Rust) |
| 3 | API types (Rust) |
| 4 | Gateway handlers (Rust) |
| 5 | Frontend types (TypeScript) |
| 6 | Frontend Settings page |
| 7 | Test and commit |