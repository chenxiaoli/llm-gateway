# Provider Proxy URL Field

## Summary

Add an optional `proxy_url` field to the Provider entity so that all upstream requests routed through that provider's channels use an HTTP/SOCKS proxy. This enables users behind corporate firewalls or in regions requiring proxy access to reach upstream LLM APIs.

## Requirements

- Single optional proxy URL per provider (e.g., `http://user:pass@proxy:8080`, `socks5://proxy:1080`)
- Auth credentials embedded directly in the URL
- Stored as plaintext in the database
- Inherited by all channels under the provider

## Design

### 1. Data Model

Add `proxy_url: Option<String>` to the `providers` table and all related Rust/TypeScript types.

**Rust types** (`crates/storage/src/types.rs`):
- `Provider.proxy_url: Option<String>`
- `ProviderWithEndpoints.proxy_url: Option<String>`
- `CreateProvider.proxy_url: Option<String>`
- `UpdateProvider.proxy_url: Option<Option<String>>` (double-optional to support clearing)

**TypeScript types** (`web/src/types/index.ts`):
- `Provider.proxy_url: string | null`
- `CreateProviderRequest.proxy_url?: string | null`
- `UpdateProviderRequest.proxy_url?: string | null`

### 2. Database Migration

**SQLite** (`migrations/sqlite/20260427000000_add_provider_proxy_url.sql`):
```sql
ALTER TABLE providers ADD COLUMN proxy_url TEXT;
```

**PostgreSQL** (`migrations/postgres/20260427000000_add_provider_proxy_url.sql`):
```sql
ALTER TABLE providers ADD COLUMN proxy_url TEXT;
```

### 3. Storage Layer

Update provider CRUD in both SQLite (`sqlite.rs`) and PostgreSQL (`postgres.rs`) implementations:
- `create_provider`: Include `proxy_url` in INSERT
- `get_provider`: Read `proxy_url` from SELECT
- `list_providers`: Read `proxy_url` from SELECT
- `update_provider`: Include `proxy_url` in UPDATE

**Seed data** (`seed.rs`, `seed_providers.json`):
- Add `proxy_url: None` to `SeedProvider` struct
- No seed providers need a proxy configured

### 4. Proxy Threading to ResolvedChannel

**`ResolvedChannel` struct** (`crates/api/src/proxy.rs`):
- Add `pub proxy_url: Option<String>`

**`InMemoryChannelRegistry::do_reload`**:
- Extract `proxy_url` from the provider and set it on `ResolvedChannel`

**Cache-miss path** in `proxy()`:
- Read provider's `proxy_url` into the resolved channel

### 5. HTTP Client Configuration

In `proxy()`, replace the shared `reqwest::Client::new()` with per-channel client construction:

```rust
// Outside failover loop: create default client for no-proxy channels
let default_client = reqwest::Client::new();

// Inside failover loop:
let client = match &channel.proxy_url {
    Some(url) => reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(url)?)
        .build()?,
    None => &default_client,
};
```

Error handling: if proxy URL is malformed, log a warning and skip the channel (treat as connection error in failover).

### 6. Frontend

**Provider detail page** (`web/src/pages/ProviderDetail.tsx`):
- Add proxy URL text input to the provider edit form
- Label: "Proxy URL", placeholder: "http://proxy:8080"
- Optional field, can be cleared
- Positioned below the endpoint fields

### 7. Files Changed

| File | Change |
|------|--------|
| `crates/storage/src/types.rs` | Add `proxy_url` to Provider types |
| `crates/storage/src/lib.rs` | No changes (Storage trait unchanged) |
| `crates/storage/src/sqlite.rs` | Update provider CRUD SQL |
| `crates/storage/src/postgres.rs` | Update provider CRUD SQL |
| `crates/storage/src/seed.rs` | Add `proxy_url` to `SeedProvider` |
| `crates/storage/seed_providers.json` | No changes (existing providers have no proxy) |
| `crates/storage/migrations/sqlite/20260427000000_add_provider_proxy_url.sql` | New migration |
| `crates/storage/migrations/postgres/20260427000000_add_provider_proxy_url.sql` | New migration |
| `crates/api/src/proxy.rs` | Add `proxy_url` to `ResolvedChannel`, configure reqwest client |
| `crates/api/src/management/providers.rs` | Handle `proxy_url` in create/update handlers |
| `web/src/types/index.ts` | Add `proxy_url` to TypeScript types |
| `web/src/pages/ProviderDetail.tsx` | Add proxy URL input to form |
