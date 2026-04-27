# Provider Proxy URL Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional `proxy_url` field to the Provider entity so upstream HTTP requests can be routed through a configurable proxy per provider.

**Architecture:** Add `proxy_url: Option<String>` to the `providers` table via migration, thread it through Rust types → storage layer → ResolvedChannel → reqwest client construction. Update frontend types and the ProviderDetail form to expose the field in the UI.

**Tech Stack:** Rust (sqlx, reqwest, axum), TypeScript/React, SQLite + PostgreSQL migrations.

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/storage/migrations/sqlite/20260427000000_add_provider_proxy_url.sql` | Create | SQLite migration adding proxy_url column |
| `crates/storage/migrations/postgres/20260427000000_add_provider_proxy_url.sql` | Create | PostgreSQL migration adding proxy_url column |
| `crates/storage/src/types.rs` | Modify | Add proxy_url to Provider, ProviderWithEndpoints, CreateProvider, UpdateProvider |
| `crates/storage/src/sqlite.rs` | Modify | Update SqliteProviderRow and provider CRUD SQL to include proxy_url |
| `crates/storage/src/postgres.rs` | Modify | Update PgProviderRow and provider CRUD SQL to include proxy_url |
| `crates/storage/src/seed.rs` | Modify | Add proxy_url to SeedProvider, set None in get_seed_providers |
| `crates/api/src/proxy.rs` | Modify | Add proxy_url to ResolvedChannel, configure reqwest client per channel |
| `crates/api/src/management/providers.rs` | Modify | Handle proxy_url in create_provider and update_provider handlers |
| `web/src/types/index.ts` | Modify | Add proxy_url to Provider, CreateProviderRequest, UpdateProviderRequest |
| `web/src/pages/ProviderDetail.tsx` | Modify | Add proxy URL input to provider edit form |

---

### Task 1: Database Migrations

**Files:**
- Create: `crates/storage/migrations/sqlite/20260427000000_add_provider_proxy_url.sql`
- Create: `crates/storage/migrations/postgres/20260427000000_add_provider_proxy_url.sql`

- [ ] **Step 1: Create SQLite migration**

Create `crates/storage/migrations/sqlite/20260427000000_add_provider_proxy_url.sql`:

```sql
ALTER TABLE providers ADD COLUMN proxy_url TEXT;
```

- [ ] **Step 2: Create PostgreSQL migration**

Create `crates/storage/migrations/postgres/20260427000000_add_provider_proxy_url.sql`:

```sql
ALTER TABLE providers ADD COLUMN proxy_url TEXT;
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/sqlite/20260427000000_add_provider_proxy_url.sql crates/storage/migrations/postgres/20260427000000_add_provider_proxy_url.sql
git commit -m "feat: add proxy_url column to providers table"
```

---

### Task 2: Rust Type Definitions

**Files:**
- Modify: `crates/storage/src/types.rs:70-120`
- Modify: `crates/storage/src/seed.rs:16-23, 77-85`

- [ ] **Step 1: Add `proxy_url` to `Provider` struct**

In `crates/storage/src/types.rs`, change lines 70-79:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub endpoints: Option<String>,      // JSON string {"default": "...", "openai": "...", "anthropic": "..."}
    pub proxy_url: Option<String>,      // HTTP/SOCKS proxy URL, e.g. "http://user:pass@proxy:8080"
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Add `proxy_url` to `ProviderWithEndpoints` struct**

In `crates/storage/src/types.rs`, change lines 82-106:

```rust
/// Provider with endpoints parsed as JSON object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderWithEndpoints {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub endpoints: Option<std::collections::HashMap<String, String>>,
    pub proxy_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Provider> for ProviderWithEndpoints {
    fn from(p: Provider) -> Self {
        let endpoints = p.endpoints.and_then(|e| serde_json::from_str(&e).ok());
        ProviderWithEndpoints {
            id: p.id,
            name: p.name,
            slug: p.slug,
            endpoints,
            proxy_url: p.proxy_url,
            enabled: p.enabled,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}
```

- [ ] **Step 3: Add `proxy_url` to `CreateProvider` and `UpdateProvider`**

In `crates/storage/src/types.rs`, change lines 108-120:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateProvider {
    pub name: String,
    pub slug: Option<String>,
    pub endpoints: Option<serde_json::Value>,
    pub proxy_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub endpoints: Option<Option<serde_json::Value>>,
    pub proxy_url: Option<Option<String>>,
    pub enabled: Option<bool>,
}
```

- [ ] **Step 4: Add `proxy_url` to `SeedProvider` and seed mapping**

In `crates/storage/src/seed.rs`, change lines 16-23:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedProvider {
    pub name: String,
    #[serde(default)]
    pub endpoints: Option<HashMap<String, String>>,
    #[serde(default)]
    pub proxy_url: Option<String>,
    pub enabled: Option<bool>,
}
```

In `crates/storage/src/seed.rs`, change the Provider construction inside `get_seed_providers` (lines 77-85):

```rust
            Provider {
                id: Uuid::new_v4().to_string(),
                name: p.name.clone(),
                slug,
                endpoints,
                proxy_url: None,
                enabled: p.enabled.unwrap_or(true),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
```

- [ ] **Step 5: Run Rust tests to verify compilation**

Run: `cd /workspace/llm-gateway && cargo test --workspace 2>&1 | tail -30`

Expected: Compilation errors in `sqlite.rs` and `postgres.rs` because the row structs don't have `proxy_url` yet. That's expected — those are fixed in Tasks 3 and 4. Types tests (if any) should still compile in isolation.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/seed.rs
git commit -m "feat: add proxy_url field to Provider types and seed data"
```

---

### Task 3: SQLite Storage Implementation

**Files:**
- Modify: `crates/storage/src/sqlite.rs:72-96` (SqliteProviderRow)
- Modify: `crates/storage/src/sqlite.rs:560-626` (provider CRUD)

- [ ] **Step 1: Update `SqliteProviderRow` to include `proxy_url`**

In `crates/storage/src/sqlite.rs`, change the `SqliteProviderRow` struct (around line 72):

```rust
#[derive(FromRow)]
struct SqliteProviderRow {
    id: String,
    name: String,
    slug: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    endpoints: Option<String>,
    proxy_url: Option<String>,
    enabled: i64,
    created_at: String,
    updated_at: String,
}
```

Update the `From<SqliteProviderRow> for Provider` impl (around line 84):

```rust
impl From<SqliteProviderRow> for Provider {
    fn from(r: SqliteProviderRow) -> Self {
        Provider {
            id: r.id,
            name: r.name,
            slug: r.slug,
            endpoints: r.endpoints,
            proxy_url: r.proxy_url,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}
```

- [ ] **Step 2: Update `create_provider` SQL**

Replace the `create_provider` method (around line 560):

```rust
    async fn create_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "INSERT INTO providers (id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(&provider.proxy_url)
        .bind(provider.enabled as i64)
        .bind(provider.created_at.to_rfc3339())
        .bind(provider.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }
```

- [ ] **Step 3: Update `get_provider` and `list_providers` SELECT queries**

Replace `get_provider` (around line 579):

```rust
    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, DbErr> {
        let row: Option<SqliteProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at
             FROM providers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Provider::from))
    }
```

Replace `list_providers` (around line 591):

```rust
    async fn list_providers(&self) -> Result<Vec<Provider>, DbErr> {
        let rows: Vec<SqliteProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at
             FROM providers",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Provider::from).collect())
    }
```

- [ ] **Step 4: Update `update_provider` SQL**

Replace `update_provider` (around line 602):

```rust
    async fn update_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "UPDATE providers SET name = ?, slug = ?, base_url = ?, endpoints = ?,
             proxy_url = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(&provider.proxy_url)
        .bind(provider.enabled as i64)
        .bind(provider.updated_at.to_rfc3339())
        .bind(&provider.id)
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }
```

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat: add proxy_url to SQLite provider storage"
```

---

### Task 4: PostgreSQL Storage Implementation

**Files:**
- Modify: `crates/storage/src/postgres.rs:55-79` (PgProviderRow)
- Modify: `crates/storage/src/postgres.rs:629-695` (provider CRUD)

- [ ] **Step 1: Update `PgProviderRow` to include `proxy_url`**

In `crates/storage/src/postgres.rs`, change the `PgProviderRow` struct (around line 55):

```rust
#[derive(FromRow)]
struct PgProviderRow {
    id: String,
    name: String,
    slug: String,
    #[allow(dead_code)]
    base_url: Option<String>,
    endpoints: Option<String>,
    proxy_url: Option<String>,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}
```

Update the `From<PgProviderRow> for Provider` impl (around line 67):

```rust
impl From<PgProviderRow> for Provider {
    fn from(r: PgProviderRow) -> Self {
        Provider {
            id: r.id,
            name: r.name,
            slug: r.slug,
            endpoints: r.endpoints,
            proxy_url: r.proxy_url,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
```

- [ ] **Step 2: Update PostgreSQL provider CRUD methods**

Replace `create_provider` (around line 629):

```rust
    async fn create_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "INSERT INTO providers (id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(&provider.proxy_url)
        .bind(provider.enabled)
        .bind(provider.created_at)
        .bind(provider.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }
```

Replace `get_provider` (around line 648):

```rust
    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, DbErr> {
        let row: Option<PgProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at
             FROM providers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Provider::from))
    }
```

Replace `list_providers` (around line 660):

```rust
    async fn list_providers(&self) -> Result<Vec<Provider>, DbErr> {
        let rows: Vec<PgProviderRow> = sqlx::query_as(
            "SELECT id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at
             FROM providers",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Provider::from).collect())
    }
```

Replace `update_provider` (around line 671):

```rust
    async fn update_provider(&self, provider: &Provider) -> Result<Provider, DbErr> {
        sqlx::query(
            "UPDATE providers SET name = $1, slug = $2, base_url = $3, endpoints = $4,
             proxy_url = $5, enabled = $6, updated_at = $7 WHERE id = $8",
        )
        .bind(&provider.name)
        .bind(&provider.slug)
        .bind(None::<String>)
        .bind(&provider.endpoints)
        .bind(&provider.proxy_url)
        .bind(provider.enabled)
        .bind(provider.updated_at)
        .bind(&provider.id)
        .execute(&self.pool)
        .await?;

        Ok(provider.clone())
    }
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat: add proxy_url to PostgreSQL provider storage"
```

---

### Task 5: Management API Handlers

**Files:**
- Modify: `crates/api/src/management/providers.rs:24-52, 87-123`

- [ ] **Step 1: Update `create_provider` handler**

In `crates/api/src/management/providers.rs`, replace the provider construction in `create_provider` (lines 33-43):

```rust
    let provider = Provider {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        slug,
        endpoints: input.endpoints.and_then(|v| {
            if v.is_null() { None } else { Some(v.to_string()) }
        }),
        proxy_url: input.proxy_url,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
```

- [ ] **Step 2: Update `update_provider` handler**

In `crates/api/src/management/providers.rs`, add the `proxy_url` update block after the `endpoints` block (after line 110, before `if let Some(enabled)`):

```rust
    if let Some(proxy_url) = input.proxy_url {
        provider.proxy_url = proxy_url;
    }
```

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/management/providers.rs
git commit -m "feat: handle proxy_url in provider create/update handlers"
```

---

### Task 6: Proxy Threading — ResolvedChannel and reqwest Client

**Files:**
- Modify: `crates/api/src/proxy.rs:32-48` (ResolvedChannel)
- Modify: `crates/api/src/proxy.rs:153-206` (do_reload resolved channel construction)
- Modify: `crates/api/src/proxy.rs:570-601` (cache-miss resolved channel construction)
- Modify: `crates/api/src/proxy.rs:618-641` (failover loop client construction)

- [ ] **Step 1: Add `proxy_url` to `ResolvedChannel`**

In `crates/api/src/proxy.rs`, add `proxy_url` field to the `ResolvedChannel` struct (after line 47, before the closing brace):

```rust
#[derive(Clone, Debug)]
pub struct ResolvedChannel {
    pub channel_id: Uuid,
    pub provider_id: String,
    pub name: String,
    pub endpoint_openai: Option<String>,
    pub endpoint_anthropic: Option<String>,
    pub upstream_api_key: String,
    pub adapter: ProxyProtocol,
    pub timeout_ms: u64,
    pub priority: i32,
    pub model_overrides: HashMap<String, ChannelModelEnriched>,
    pub proxy_url: Option<String>,
}
```

- [ ] **Step 2: Update `do_reload` resolved channel construction**

In `crates/api/src/proxy.rs`, in the `do_reload` method, extract `proxy_url` from the provider. After the `endpoints` parsing block (after line 169, before `let api_key = ...`), add:

```rust
            let proxy_url = provider.proxy_url.clone();
```

Then in the `ResolvedChannel` construction (lines 195-206), add the field:

```rust
            let resolved = ResolvedChannel {
                channel_id: Uuid::parse_str(&channel.id).unwrap_or_else(|_| Uuid::new_v4()),
                provider_id: channel.provider_id.clone(),
                name: channel.name.clone(),
                endpoint_openai,
                endpoint_anthropic,
                upstream_api_key: api_key,
                adapter: ProxyProtocol::OpenAI,
                timeout_ms: 60_000,
                priority: channel.priority,
                model_overrides,
                proxy_url,
            };
```

- [ ] **Step 3: Update cache-miss resolved channel construction**

In the cache-miss path (around lines 590-601), add `proxy_url` to the `ResolvedChannel`:

After the `endpoint_anthropic` extraction (around line 585), add:

```rust
                    let proxy_url = provider.proxy_url.clone();
```

In the `ResolvedChannel` construction:

```rust
                    let resolved = ResolvedChannel {
                        channel_id: Uuid::parse_str(&channel.id).unwrap_or_else(|_| Uuid::new_v4()),
                        provider_id: channel.provider_id.clone(),
                        name: channel.name.clone(),
                        endpoint_openai,
                        endpoint_anthropic,
                        upstream_api_key: api_key,
                        adapter: protocol,
                        timeout_ms: 60_000,
                        priority: channel.priority,
                        model_overrides: HashMap::new(),
                        proxy_url,
                    };
```

- [ ] **Step 4: Replace shared reqwest client with per-channel proxy support**

In `crates/api/src/proxy.rs`, replace line 618:

```rust
    let client = reqwest::Client::new();
```

With:

```rust
    let default_client = reqwest::Client::new();
```

Then, inside the failover loop, replace line 641:

```rust
        let mut req = client.post(&upstream_url);
```

With:

```rust
        let client = match &channel.proxy_url {
            Some(proxy_url) => {
                match reqwest::Proxy::all(proxy_url) {
                    Ok(proxy) => {
                        match reqwest::Client::builder().proxy(proxy).build() {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::warn!("[PROXY] Failed to build proxied client for channel '{}': {}", channel.name, e);
                                last_error = format!("Proxy client error on channel '{}': {}", channel.name, e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[PROXY] Invalid proxy URL '{}' for channel '{}': {}", proxy_url, channel.name, e);
                        last_error = format!("Invalid proxy URL on channel '{}': {}", channel.name, e);
                        continue;
                    }
                }
            }
            None => default_client.clone(),
        };
        let mut req = client.post(&upstream_url);
```

Note: `reqwest::Client` implements `Clone` cheaply (it clones an `Arc` internally), so `default_client.clone()` is fine.

- [ ] **Step 5: Update existing tests to include `proxy_url`**

In `crates/api/src/proxy.rs`, update the test `resolved_channel_carries_all_fields` (around line 989). Add `proxy_url: None` to the `ResolvedChannel` construction.

Also update `stub_registry_resolve_by_model` (around line 1014) and `stub_registry_resolve_by_id` (around line 1033) — add `proxy_url: None` to each `ResolvedChannel` in those tests.

- [ ] **Step 6: Run tests to verify**

Run: `cd /workspace/llm-gateway && cargo test --workspace 2>&1 | tail -30`

Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat: thread proxy_url through ResolvedChannel and configure reqwest client"
```

---

### Task 7: Frontend — TypeScript Types

**Files:**
- Modify: `web/src/types/index.ts:35-54`

- [ ] **Step 1: Add `proxy_url` to TypeScript interfaces**

In `web/src/types/index.ts`, update the `Provider` interface (lines 35-43):

```typescript
export interface Provider {
  id: string;
  name: string;
  slug: string;
  endpoints: Record<string, string> | null;
  proxy_url: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}
```

Update `CreateProviderRequest` (lines 45-48):

```typescript
export interface CreateProviderRequest {
  name: string;
  endpoints?: Record<string, string | null> | null;
  proxy_url?: string | null;
}
```

Update `UpdateProviderRequest` (lines 50-54):

```typescript
export interface UpdateProviderRequest {
  name?: string;
  endpoints?: Record<string, string | null> | null;
  proxy_url?: string | null;
  enabled?: boolean;
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/types/index.ts
git commit -m "feat: add proxy_url to TypeScript Provider types"
```

---

### Task 8: Frontend — Provider Detail Page

**Files:**
- Modify: `web/src/pages/ProviderDetail.tsx:35-83, 149-158`

- [ ] **Step 1: Add proxy URL state**

In `web/src/pages/ProviderDetail.tsx`, add state after the existing state declarations (after line 38, the `provEnabled` state):

```typescript
  const [provProxyUrl, setProvProxyUrl] = useState('');
```

- [ ] **Step 2: Initialize proxy URL from provider data**

In the `useEffect` that initializes form state (around lines 58-71), after `setProvEnabled(provider.enabled);`, add:

```typescript
      setProvProxyUrl(provider.proxy_url || '');
```

- [ ] **Step 3: Include proxy_url in update payload**

In `handleUpdateProvider` (around lines 76-83), update the mutation call to include `proxy_url`:

```typescript
  const handleUpdateProvider = async (e: React.FormEvent) => {
    e.preventDefault();
    const endpoints: Record<string, string | null> = {
      openai: provOpenaiUrl || null,
      anthropic: provAnthropicUrl || null
    };
    await updateMutation.mutateAsync({
      id: provider.id,
      input: {
        name: provName,
        endpoints,
        proxy_url: provProxyUrl || null,
        enabled: provEnabled,
      },
    });
  };
```

- [ ] **Step 4: Add proxy URL input to the form**

In the provider edit form (around line 152, after the Anthropic Endpoint input), add a new form control:

```tsx
        <div className="form-control"><label className="label"><span className="label-text">Proxy URL</span></label><input type="text" value={provProxyUrl} onChange={(e) => setProvProxyUrl(e.target.value)} placeholder="http://proxy:8080" className="input input-bordered w-full" /></div>
```

This goes after the Anthropic endpoint input `<div>` and before the Enabled toggle `<div>`.

- [ ] **Step 5: Verify frontend builds**

Run: `cd /workspace/llm-gateway/web && npm run build 2>&1 | tail -20`

Expected: Build succeeds with no TypeScript errors.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/ProviderDetail.tsx
git commit -m "feat: add proxy URL input to provider detail page"
```

---

### Task 9: Final Verification

- [ ] **Step 1: Run full Rust test suite**

Run: `cd /workspace/llm-gateway && cargo test --workspace 2>&1 | tail -30`

Expected: All tests pass.

- [ ] **Step 2: Run frontend tests**

Run: `cd /workspace/llm-gateway/web && npm test 2>&1 | tail -20`

Expected: All tests pass.

- [ ] **Step 3: Verify frontend build**

Run: `cd /workspace/llm-gateway/web && npm run build 2>&1 | tail -10`

Expected: Build succeeds.
