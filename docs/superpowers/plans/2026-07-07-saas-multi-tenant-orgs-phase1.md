# SaaS Multi-Tenant Orgs — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `orgs` / `members` / `org_settings` tables and a new `org` crate; thread `org_id` (tenant tables) and `owner_org_id` (catalog tables) through every storage method and API handler. Externally invisible — the system still behaves as a single-tenant gateway because every user lands in the bootstrap `default` org.

**Architecture:** Bottom-up. Migration first → storage types → storage trait → postgres impl → JWT claims → org crate → API handler call-site updates → proxy context → frontend store shape. No URL changes, no new routes other than `/api/v1/orgs` and `/api/v1/me/current-org` (both unscoped, no `/{org_slug}`). Phase 2 introduces path-based routing and the visible OrgSwitcher UI.

**Tech Stack:** Rust (Axum, sqlx, async-trait), PostgreSQL 13+ (`gen_random_uuid()`), React + TypeScript + Zustand.

**Spec reference:** `docs/superpowers/specs/2026-07-07-saas-multi-tenant-orgs-design.md` — Decisions, Data Model, and Migration sections are authoritative for SQL and types.

---

## File Structure

**New crate `org`** (`crates/org/`)
- `Cargo.toml` — depends on `storage`, `auth`
- `src/lib.rs` — re-exports
- `src/types.rs` — `OrgContext`, `MemberRole`, `PlatformRole`, re-exports of `Org`/`Member` from storage
- `src/access.rs` — pure permission helpers (`can_manage_org_settings`, etc.)
- `src/error.rs` — `OrgError` enum
- `src/extractors.rs` — `resolve_org_context(claims, storage)` async helper

**Migration**
- `crates/storage/migrations/postgres/20260708000000_saas_orgs.sql` (new) — forward migration, single transaction with self-check
- `crates/storage/migrations/postgres/20260708000000_saas_orgs.down.sql` (new) — emergency rollback (first down-migration in the repo; convention break documented in Task 2)

**Storage** (`crates/storage/src/`)
- `types.rs` — new: `Org`, `CreateOrg`, `UpdateOrg`, `Member`, `MemberRole`, `PlatformRole`, `MembershipSummary`. Modified: add `org_id: String` to 9 tenant types, add `owner_org_id: Option<String>` to 4 catalog types, modify `User` (add `current_org_id`, `platform_role`; drop `role`, `group_id`), add `actor_is_platform_admin: bool` to `AuditLog`.
- `lib.rs` — `Storage` trait: add `org_*`, `member_*`, `get_platform_setting`/`set_platform_setting`, `get_org_setting`/`set_org_setting`/`list_org_settings`. Modify ~70 existing methods to take `org_id: &str` (tenant) or `viewer_org_id: &str` (catalog). Drop legacy `users.role`-based methods (`count_admin_users`).
- `postgres.rs` — implement everything above (~1100 LOC file; mechanical updates).

**Auth**
- `crates/auth/src/lib.rs` — extend `JwtClaims` with `current_org_id: String`, `platform_role: Option<String>`; update `create_jwt` signature.

**API**
- `crates/api/src/extractors.rs` — `require_auth` already returns `JwtClaims`; no signature change, but downstream callers now read `claims.current_org_id` instead of `claims.role`.
- `crates/api/src/auth.rs` — extend `AuthResponse`/`MeResponse` with `current_org`, `orgs`; new `switch_org` handler; registration auto-creates personal org (Phase 3 — left as TODO comment in Phase 1).
- `crates/api/src/management/mod.rs` — register `GET /api/v1/orgs`, `POST /api/v1/me/current-org`.
- `crates/api/src/management/*.rs` — every handler reads `claims.current_org_id` and passes it as `org_id` to storage. The `if claims.role == "admin" { ... } else { ... }` branching is replaced by `if let Some(p) = claims.platform_role { ... } else { ... }` for admin-only endpoints (providers/channels/models/groups/users/pricing-policies management).
- `crates/api/src/proxy.rs` — `get_key_by_hash` now returns the key's `org_id`; threading it into routing/usage/audit is mostly automatic because those storage methods already get the key context.

**Frontend**
- `web/src/types/index.ts` — extend `AuthResponse` and `UserInfo` with `current_org`, `orgs` (or new `OrgSummary` type).
- `web/src/stores/authStore.ts` — extend `AuthState` with `currentOrg: OrgSummary | null`, `orgs: OrgSummary[]`. No OrgSwitcher UI in Phase 1.

**Tests**
- `crates/org/src/access.rs` — table-driven unit tests for permission helpers (inline `#[test_case]`).
- `crates/api/tests/test_auth.rs` — extend to cover `current_org`/`orgs` in login response; add `switch_org` test.
- `crates/api/tests/common/mod.rs` — `make_admin_token()` / `make_user_token(id)` now mint tokens with `current_org_id` and `platform_role` set; the existing "admin" token maps to `platform_role = Some("platform_admin")`.

---

## Deployment Notes

**No breaking changes in Phase 1.** Existing frontend builds run unchanged against the new backend — they ignore the new `current_org`/`orgs` fields in the login response. The migration moves every existing row into `org_default`, so existing api_keys still resolve, existing users still log in, and every existing handler still returns the same data.

**Down migration is emergency-only.** No prior migration in this repo has a `.down.sql`. Task 2 introduces the pattern with explicit documentation; sqlx picks it up automatically because of its `_down.sql` naming convention.

---

### Task 1: Bootstrap `org` crate skeleton

**Files:**
- Create: `crates/org/Cargo.toml`
- Create: `crates/org/src/lib.rs`
- Create: `crates/org/src/types.rs`
- Create: `crates/org/src/error.rs`
- Create: `crates/org/src/access.rs`
- Create: `crates/org/src/extractors.rs`
- Modify: `Cargo.toml` (workspace root — add `crates/org` to members)

- [ ] **Step 1: Add crate to workspace**

`Cargo.toml` at repo root currently lists 12 members under `[workspace] members = [...]`. Add `"crates/org"` after `"crates/audit-worker"`.

```toml
members = [
    "crates/gateway",
    "crates/api",
    "crates/provider",
    "crates/auth",
    "crates/ratelimit",
    "crates/billing",
    "crates/audit",
    "crates/storage",
    "crates/encryption",
    "crates/nats-publisher",
    "crates/usage-worker",
    "crates/audit-worker",
    "crates/org",
]
```

- [ ] **Step 2: Write `crates/org/Cargo.toml`**

```toml
[package]
name = "llm-gateway-org"
version = "0.16.8"
edition = "2021"

[dependencies]
llm-gateway-storage = { path = "../storage" }
llm-gateway-auth = { path = "../auth" }
async-trait = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
```

(Version `0.16.8` matches the other workspace-internal crates as of v1.8.4.)

- [ ] **Step 3: Write `crates/org/src/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrgError {
    #[error("org not found: {0}")]
    NotFound(String),

    #[error("user {0} is not a member of org {1}")]
    NotMember(String, String),

    #[error("forbidden: requires {0}")]
    Forbidden(String),

    #[error("slug already taken: {0}")]
    SlugTaken(String),

    #[error("cannot remove the last owner of org {0}")]
    LastOwner(String),
}

impl From<OrgError> for llm_gateway_storage::StorageError {
    fn from(e: OrgError) -> Self {
        // Map to the existing StorageError variants if/when they exist.
        // For now, we box via std::io::Error to keep storage trait stable.
        llm_gateway_storage::StorageError::Other(e.to_string())
    }
}
```

If `StorageError::Other` doesn't exist, fall back to returning `Box<dyn std::error::Error>` from the helper functions (see Task 7).

- [ ] **Step 4: Write `crates/org/src/types.rs`**

```rust
use llm_gateway_storage::{Member, MemberRole, Org, PlatformRole};

/// Per-request context derived from JWT + membership lookup.
/// In Phase 1 this is constructed from `claims.current_org_id` only
/// (no path-based routing yet — Phase 2 adds OrgResolveLayer/MembershipLayer).
#[derive(Debug, Clone)]
pub struct OrgContext {
    pub org_id: String,
    pub member_role: MemberRole,
    pub platform_role: Option<PlatformRole>,
    pub group_id: Option<String>,
}

impl OrgContext {
    pub fn is_platform_admin(&self) -> bool {
        matches!(self.platform_role, Some(PlatformRole::PlatformAdmin))
    }
}

pub use llm_gateway_storage::{Member, MemberRole, Org, PlatformRole};
```

- [ ] **Step 5: Write `crates/org/src/access.rs` (permission helpers — empty bodies for now)**

```rust
use crate::types::{MemberRole, OrgContext};

/// Admin-or-above in the current org, OR platform_admin.
pub fn can_manage_org_settings(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_invite_members(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_delete_org(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner) || ctx.is_platform_admin()
}

pub fn can_manage_channels(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_create_org_catalog(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_create_platform_catalog(ctx: &OrgContext) -> bool {
    ctx.is_platform_admin()
}

/// Used by channel-listing filter: members see channels in their group + ungrouped;
/// admin/owner/platform_admin see everything.
pub fn can_access_channel(ctx: &OrgContext, channel_group_id: Option<&str>) -> bool {
    match ctx.member_role {
        MemberRole::Owner | MemberRole::Admin => true,
        MemberRole::Member => {
            ctx.is_platform_admin()
                || channel_group_id.is_none()
                || ctx.group_id.as_deref() == channel_group_id
        }
    }
}
```

- [ ] **Step 6: Write `crates/org/src/extractors.rs` (placeholder — full impl in Task 7)**

```rust
//! Phase 1 stub. Task 7 adds the real `resolve_org_context` after the storage
//! trait exposes `get_member`.
```

- [ ] **Step 7: Write `crates/org/src/lib.rs`**

```rust
pub mod access;
pub mod error;
pub mod extractors;
pub mod types;

pub use access::*;
pub use error::OrgError;
pub use types::*;
```

- [ ] **Step 8: Verify crate compiles**

```bash
cargo build -p llm-gateway-org
```

Expected: FAIL — types.rs references `Member`, `MemberRole`, `Org`, `PlatformRole` from `llm_gateway_storage` which don't exist yet. **This is expected and is fixed in Task 3.** Comment out the `use` line in `types.rs` and the bodies of all `access.rs` functions if you want a green build at this checkpoint:

```rust
// crates/org/src/access.rs — temporary stubs
pub fn can_manage_org_settings(_ctx: ()) -> bool { false }
// (etc., one stub per function)
```

Skip this stubbing if you're going straight to Task 2 and Task 3 next.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml crates/org/
git commit -m "feat(org): bootstrap llm-gateway-org crate skeleton"
```

---

### Task 2: Write migration `20260708000000_saas_orgs.sql`

**Files:**
- Create: `crates/storage/migrations/postgres/20260708000000_saas_orgs.sql`
- Create: `crates/storage/migrations/postgres/20260708000000_saas_orgs.down.sql`
- Modify: `crates/storage/build.rs`

- [ ] **Step 1: Write forward migration**

Copy the migration body verbatim from `docs/superpowers/specs/2026-07-07-saas-multi-tenant-orgs-design.md` → section "## Migration" → subsection "### File: `migrations/postgres/20260708000000_saas_orgs.sql`". The block runs from `BEGIN;` to `COMMIT;` and contains 19 numbered steps plus a self-check `DO $$ ... $$` block.

If the spec's column orders or constraint names diverge from what's in the existing schema, prefer the existing names (e.g., `channels.created_by` already exists per migration `20260506000000_channel_created_by.sql`).

- [ ] **Step 2: Write down migration**

Copy verbatim from the spec's "### Rollback" subsection into `20260708000000_saas_orgs.down.sql`. The spec includes the `DELETE FROM ... WHERE owner_org_id IS NOT NULL` cleanup before the `DROP COLUMN` calls.

- [ ] **Step 3: Update `build.rs` to track the new files**

```rust
fn main() {
    println!("cargo:rerun-if-changed=migrations/*");
    println!("cargo:rerun-if-changed=migrations/postgres/*");
    println!("cargo:rerun-if-changed=migrations/postgres/20260708000000_saas_orgs.sql");
    println!("cargo:rerun-if-changed=migrations/postgres/20260708000000_saas_orgs.down.sql");
}
```

- [ ] **Step 4: Manually verify migration runs against a real Postgres**

Start a scratch Postgres (the project's `docker-compose.yml` has one) and apply all migrations in order:

```bash
docker compose up -d postgres
DATABASE_URL=postgres://postgres:postgres@localhost:5432/llm_gateway \
    cargo run --bin llm-gateway-gateway -- --migrate-only 2>/dev/null || \
    sqlx migrate run --source crates/storage/migrations/postgres "$DATABASE_URL"
```

If `--migrate-only` isn't a real flag on the gateway binary, the simplest verification is: temporarily add `tokio::runtime::Runtime::new().unwrap().block_on(storage.run_migrations());` to `main` and run the binary once against an empty DB, then revert.

Expected: migration applies without errors, and `SELECT slug, name FROM orgs;` returns one row `default | Default Org`.

- [ ] **Step 5: Verify self-check triggers on bad data**

```bash
docker compose exec postgres psql -U postgres -d llm_gateway \
    -c "INSERT INTO api_keys (id, name, key_hash, enabled, org_id, created_at, updated_at)
        VALUES ('test_orphan', 'orphan', 'hash', true, NULL, NOW(), NOW());"
```

Wait — `org_id` is NOT NULL after migration. The insert should fail. Try with a NULL org_id:

```bash
docker compose exec postgres psql -U postgres -d llm_gateway \
    -c "ALTER TABLE api_keys ALTER COLUMN org_id DROP NOT NULL;
        INSERT INTO api_keys (id, name, key_hash, enabled, org_id, created_at, updated_at)
        VALUES ('test_orphan', 'orphan', 'hash', NULL, NOW(), NOW());"
```

This proves the column constraint is real. Restore:

```bash
docker compose exec postgres psql -U postgres -d llm_gateway \
    -c "DELETE FROM api_keys WHERE id = 'test_orphan';
        ALTER TABLE api_keys ALTER COLUMN org_id SET NOT NULL;"
```

The migration's `DO $$ ... $$` block only runs at migration time; runtime NOT NULL constraints are what guard steady state.

- [ ] **Step 6: Verify down migration works**

```bash
sqlx migrate revert --source crates/storage/migrations/postgres "$DATABASE_URL"
```

Expected: `orgs`, `members`, `org_settings` gone; `platform_settings` renamed back to `settings`; `users.role` column restored with admins marked `'admin'`.

Re-apply forward migration to leave the DB in the post-Phase-1 state:

```bash
sqlx migrate run --source crates/storage/migrations/postgres "$DATABASE_URL"
```

- [ ] **Step 7: Commit**

```bash
git add crates/storage/migrations/postgres/20260708000000_saas_orgs.sql \
        crates/storage/migrations/postgres/20260708000000_saas_orgs.down.sql \
        crates/storage/build.rs
git commit -m "feat(storage): add orgs/members migration + emergency down"
```

---

### Task 3: Update storage `types.rs`

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add new types (Org, Member, roles, etc.) at top of file**

Insert after existing imports:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

impl MemberRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberRole::Owner => "owner",
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "owner" => MemberRole::Owner,
            "admin" => MemberRole::Admin,
            "member" => MemberRole::Member,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformRole {
    PlatformAdmin,
}

impl PlatformRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformRole::PlatformAdmin => "platform_admin",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "platform_admin" => Some(PlatformRole::PlatformAdmin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Org {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub owner_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateOrg {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UpdateOrg {
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Member {
    pub user_id: String,
    pub org_id: String,
    pub role: MemberRole,
    pub group_id: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MembershipSummary {
    pub org: Org,
    pub role: MemberRole,
    pub group_id: Option<String>,
}
```

- [ ] **Step 2: Modify `User` struct**

Current at types.rs:708:
```rust
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub role: String,
    pub enabled: bool,
    pub refresh_token: Option<String>,
    pub group_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Change to:
```rust
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub platform_role: Option<PlatformRole>,
    pub current_org_id: Option<String>,
    pub enabled: bool,
    pub refresh_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Note: `role` and `group_id` removed (group_id moves to `members.group_id`, role moves to `members.role` + `users.platform_role`). Down migration restores them; the Rust type doesn't need to keep them.

- [ ] **Step 3: Add `org_id: String` to tenant structs**

For each of: `Channel`, `ChannelModel`, `ApiKey`, `UsageRecord`, `AuditLog`, `Account`, `Transaction`, `KeyModelRateLimit`, `Group` — add a field `pub org_id: String,` (place it right after `id: String,` for consistency).

`AuditLog` additionally gets:
```rust
pub actor_is_platform_admin: bool,
```

- [ ] **Step 4: Add `owner_org_id: Option<String>` to catalog structs**

For each of: `Provider`, `Model`, `PricingPolicy`, `ProviderModel` (if exists — check; if not, skip the ProviderModel struct step) — add:
```rust
pub owner_org_id: Option<String>,
```

(Place after `id: String,` for consistency.)

- [ ] **Step 5: Update `Create*` and `Update*` types to match**

For each `CreateChannel`, `CreateApiKey`, `CreateModel`, etc., add the same `org_id` or `owner_org_id` field. `Update*` types do **not** get the field — scope is immutable post-create per spec decision #14.

- [ ] **Step 6: Verify build**

```bash
cargo build -p llm-gateway-storage 2>&1 | head -50
```

Expected: errors only in `postgres.rs` and `lib.rs` (those are fixed in Tasks 4-5). The types file itself should compile clean.

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(storage): add Org/Member types, scope fields on tenant/catalog rows"
```

---

### Task 4: Extend Storage trait

**Files:**
- Modify: `crates/storage/src/lib.rs`

**Strategy:** the trait has ~70 methods. The mechanical transformation is "every method that touches a tenant table gains `org_id: &str` as the first parameter after `&self`; every method that touches a catalog table gains `viewer_org_id: &str`."

- [ ] **Step 1: Add new methods for org/member/settings management**

Append to the Storage trait body (before the closing `}`):

```rust
// ---- Orgs ----
async fn create_org(&self, org: CreateOrg) -> Result<Org, Box<dyn std::error::Error + Send + Sync>>;
async fn get_org(&self, id: &str) -> Result<Option<Org>, Box<dyn std::error::Error + Send + Sync>>;
async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Org>, Box<dyn std::error::Error + Send + Sync>>;
async fn list_orgs_for_user(&self, user_id: &str) -> Result<Vec<MembershipSummary>, Box<dyn std::error::Error + Send + Sync>>;
async fn update_org(&self, id: &str, updates: UpdateOrg) -> Result<Org, Box<dyn std::error::Error + Send + Sync>>;
async fn delete_org(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

// ---- Members ----
async fn get_member(&self, user_id: &str, org_id: &str) -> Result<Option<Member>, Box<dyn std::error::Error + Send + Sync>>;
async fn list_members(&self, org_id: &str) -> Result<Vec<Member>, Box<dyn std::error::Error + Send + Sync>>;
async fn upsert_member(&self, member: Member) -> Result<Member, Box<dyn std::error::Error + Send + Sync>>;
async fn update_member_role(&self, user_id: &str, org_id: &str, role: MemberRole) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
async fn delete_member(&self, user_id: &str, org_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
async fn count_owners(&self, org_id: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

// ---- Settings split ----
async fn get_platform_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
async fn set_platform_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
async fn get_org_setting(&self, org_id: &str, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
async fn set_org_setting(&self, org_id: &str, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
async fn list_org_settings(&self, org_id: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>>;
```

Keep the existing `get_setting`/`set_setting` methods as thin wrappers around `get_platform_setting`/`set_platform_setting` for the duration of Phase 1 to minimize call-site churn:

```rust
async fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    self.get_platform_setting(key).await
}
async fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    self.set_platform_setting(key, value).await
}
```

Mark them `#[deprecated(note = "use get_platform_setting directly")]` so callers migrate over time.

- [ ] **Step 2: Modify tenant-table methods — add `org_id: &str` as first arg after `&self`**

Apply this transformation to every method in the trait that reads/writes one of: `channels`, `channel_models`, `api_keys`, `usage_records`, `audit_logs`, `accounts`, `transactions`, `key_model_rate_limits`, `groups`.

Verbatim signatures of the most-used methods after the change:

```rust
async fn create_key(&self, org_id: &str, key: ApiKey) -> Result<ApiKey, ...>;
async fn get_key(&self, org_id: &str, id: &str) -> Result<Option<ApiKey>, ...>;
async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, ...>;  // NO org_id — login/proxy resolve first
async fn list_keys(&self, org_id: &str) -> Result<Vec<ApiKey>, ...>;
async fn list_keys_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, ...>;
async fn list_keys_paginated_for_user(&self, org_id: &str, created_by: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<ApiKey>, ...>;

async fn create_channel(&self, org_id: &str, channel: Channel) -> Result<Channel, ...>;
async fn get_channel(&self, org_id: &str, id: &str) -> Result<Option<Channel>, ...>;
async fn list_channels(&self, org_id: &str) -> Result<Vec<Channel>, ...>;
async fn list_channels_by_provider(&self, org_id: &str, provider_id: &str) -> Result<Vec<Channel>, ...>;
async fn list_enabled_channels_by_provider(&self, org_id: &str, provider_id: &str) -> Result<Vec<Channel>, ...>;
async fn update_channel(&self, org_id: &str, id: &str, updates: UpdateChannel) -> Result<Channel, ...>;
async fn delete_channel(&self, org_id: &str, id: &str) -> Result<(), ...>;
async fn disable_channel_until(&self, org_id: &str, id: &str, until: Option<DateTime<Utc>>) -> Result<(), ...>;

async fn record_usage(&self, org_id: &str, record: UsageRecord) -> Result<(), ...>;
async fn query_usage(&self, org_id: &str, filter: UsageFilter) -> Result<Vec<UsageRecord>, ...>;
async fn query_usage_paginated(&self, org_id: &str, filter: UsageFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<UsageRecord>, ...>;
async fn query_usage_summary(&self, org_id: &str, filter: UsageFilter) -> Result<UsageSummary, ...>;
async fn query_channel_usage_summary(&self, org_id: &str, channel_id: &str, filter: UsageFilter) -> Result<UsageSummary, ...>;
async fn query_daily_usage(&self, org_id: &str, filter: UsageFilter) -> Result<Vec<DailyUsage>, ...>;
async fn get_usage_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<UsageRecord>, ...>;

async fn insert_log(&self, org_id: &str, log: AuditLog) -> Result<(), ...>;
async fn query_logs(&self, org_id: &str, filter: AuditFilter) -> Result<Vec<AuditLog>, ...>;
async fn query_logs_paginated(&self, org_id: &str, filter: AuditFilter, page: i64, page_size: i64) -> Result<PaginatedResponse<AuditLog>, ...>;
async fn get_log(&self, org_id: &str, id: &str) -> Result<Option<AuditLog>, ...>;
async fn get_audit_by_request_id(&self, org_id: &str, request_id: &str) -> Result<Option<AuditLog>, ...>;

async fn create_account(&self, org_id: &str, account: Account) -> Result<Account, ...>;
async fn get_account(&self, org_id: &str, id: &str) -> Result<Option<Account>, ...>;
async fn get_account_by_user_id(&self, org_id: &str, user_id: &str) -> Result<Option<Account>, ...>;
async fn update_account(&self, org_id: &str, account: Account) -> Result<Account, ...>;
async fn create_transaction(&self, org_id: &str, t: Transaction) -> Result<Transaction, ...>;
async fn list_transactions(&self, org_id: &str, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, ...>;
async fn deduct_balance(&self, org_id: &str, req: DeductBalance) -> Result<Transaction, ...>;
async fn add_balance(&self, org_id: &str, req: AddBalance) -> Result<Transaction, ...>;

async fn list_groups(&self, org_id: &str) -> Result<Vec<Group>, ...>;
async fn list_groups_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Group>, ...>;
async fn get_group(&self, org_id: &str, id: &str) -> Result<Option<Group>, ...>;
async fn create_group(&self, org_id: &str, group: Group) -> Result<Group, ...>;
async fn update_group(&self, org_id: &str, id: &str, updates: UpdateGroup) -> Result<Group, ...>;
async fn delete_group(&self, org_id: &str, id: &str) -> Result<(), ...>;
async fn get_user_group_id(&self, user_id: &str, org_id: &str) -> Result<Option<String>, ...>;  // note: lookup goes via members
```

For the remaining tenant-table methods (channel_models, key_model_rate_limits, model_fallbacks, etc.), apply the same pattern. Reference the spec's full list.

- [ ] **Step 3: Modify catalog-table methods — add `viewer_org_id: &str` as first arg after `&self`**

For methods touching `providers`, `models`, `pricing_policies`, `provider_models`:

```rust
async fn create_provider(&self, viewer_org_id: &str, provider: Provider) -> Result<Provider, ...>;
async fn get_provider(&self, viewer_org_id: &str, id: &str) -> Result<Option<Provider>, ...>;
async fn list_providers(&self, viewer_org_id: &str) -> Result<Vec<Provider>, ...>;
async fn update_provider(&self, viewer_org_id: &str, id: &str, updates: UpdateProvider) -> Result<Provider, ...>;
async fn delete_provider(&self, viewer_org_id: &str, id: &str) -> Result<(), ...>;

async fn create_model(&self, viewer_org_id: &str, model: Model) -> Result<Model, ...>;
async fn get_model(&self, viewer_org_id: &str, name: &str) -> Result<Option<Model>, ...>;
async fn get_model_by_id(&self, viewer_org_id: &str, id: &str) -> Result<Option<Model>, ...>;
async fn get_model_by_provider(&self, viewer_org_id: &str, provider_id: &str, name: &str) -> Result<Option<Model>, ...>;
async fn list_models(&self, viewer_org_id: &str) -> Result<Vec<Model>, ...>;
async fn list_models_by_provider(&self, viewer_org_id: &str, provider_id: &str) -> Result<Vec<Model>, ...>;
async fn update_model(&self, viewer_org_id: &str, id: &str, updates: UpdateModel) -> Result<Model, ...>;
async fn delete_model(&self, viewer_org_id: &str, id: &str) -> Result<(), ...>;
```

`PricingPolicy` and `ProviderModel` methods follow the same pattern.

- [ ] **Step 4: Modify user methods**

`User` is now `current_org_id`-aware but isn't org-scoped itself (users are global, M:N via members). Most user methods don't gain an org param:

```rust
async fn create_user(&self, user: User) -> Result<User, ...>;          // unchanged
async fn get_user(&self, id: &str) -> Result<Option<User>, ...>;       // unchanged
async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, ...>;  // unchanged
async fn list_users(&self, org_id: &str) -> Result<Vec<User>, ...>;    // NOW org-scoped — joins via members
async fn list_users_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<User>, ...>;
async fn update_user(&self, id: &str, updates: UpdateUser) -> Result<User, ...>;  // unchanged
async fn delete_user(&self, id: &str) -> Result<(), ...>;              // unchanged (memberships cascade)
```

Remove `count_admin_users` (no longer meaningful — admins are per-org via `members.role`, platform_admins are rare and counted via `SELECT COUNT(*) FROM users WHERE platform_role = 'platform_admin'`). Replace call sites with that query if needed.

- [ ] **Step 5: Verify trait compiles in isolation**

```bash
cargo check -p llm-gateway-storage 2>&1 | head -80
```

Expected: many errors in `postgres.rs` (trait impl now mismatches). That's Task 5's job. The `lib.rs` trait definition itself should be syntactically valid.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m "feat(storage): org/member methods + scope params on existing trait methods"
```

---

### Task 5: Update Postgres impl

**Files:**
- Modify: `crates/storage/src/postgres.rs` (~1100 LOC)

This is the largest task by line count. The work is mechanical: every method body needs `WHERE org_id = $1` (or `owner_org_id IS NULL OR owner_org_id = $1` for catalog) added to its queries, plus the new `org_id`/`viewer_org_id` parameter threaded through.

- [ ] **Step 1: Implement the new org/member/settings methods**

Append to the `impl Storage for PostgresStorage` block:

```rust
async fn create_org(&self, org: CreateOrg) -> Result<Org, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, Org>(
        "INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, NOW(), NOW())
         RETURNING id, slug, name, owner_id, created_at, updated_at"
    )
    .bind(&org.id).bind(&org.slug).bind(&org.name).bind(&org.owner_id)
    .fetch_one(&self.pool).await.map_err(|e| e.into())
}

async fn get_org(&self, id: &str) -> Result<Option<Org>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, Org>("SELECT * FROM orgs WHERE id = $1")
        .bind(id).fetch_optional(&self.pool).await.map_err(|e| e.into())
}

async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Org>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, Org>("SELECT * FROM orgs WHERE slug = $1")
        .bind(slug).fetch_optional(&self.pool).await.map_err(|e| e.into())
}

async fn list_orgs_for_user(&self, user_id: &str) -> Result<Vec<MembershipSummary>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, MembershipSummary>(
        "SELECT o.id AS \"org_id\", o.slug, o.name, o.owner_id, o.created_at, o.updated_at,
                m.role, m.group_id
         FROM members m JOIN orgs o ON o.id = m.org_id
         WHERE m.user_id = $1 ORDER BY o.name"
    ).bind(user_id).fetch_all(&self.pool).await.map_err(|e| e.into())
}
// Note: MembershipSummary struct needs FromRow to match these column aliases.

// ... update_org, delete_org, get_member, list_members, upsert_member,
// update_member_role, delete_member, count_owners, get_platform_setting,
// set_platform_setting, get_org_setting, set_org_setting, list_org_settings
// — straightforward query_as / query.execute bodies, ~30-50 LOC each.
```

For the full body of each function, follow the pattern of the existing `create_provider` / `get_provider` / `list_providers` etc. in the same file.

- [ ] **Step 2: Update tenant-table methods to filter by org_id**

For each method that takes `org_id: &str` (Task 4 step 2), modify the SQL to add `AND org_id = $N` to the WHERE clause, and `.bind(org_id)` at the appropriate parameter position.

Example transformation for `list_channels`:

Before:
```rust
async fn list_channels(&self) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, Channel>("SELECT * FROM channels ORDER BY name")
        .fetch_all(&self.pool).await.map_err(|e| e.into())
}
```

After:
```rust
async fn list_channels(&self, org_id: &str) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE org_id = $1 ORDER BY name")
        .bind(org_id).fetch_all(&self.pool).await.map_err(|e| e.into())
}
```

Apply the same pattern to every method listed in Task 4 step 2. The INSERT methods additionally need `.bind(&row.org_id)` in their `INSERT` column list and `RETURNING` clause.

- [ ] **Step 3: Update catalog-table methods to use visibility filter**

For methods taking `viewer_org_id: &str` (Task 4 step 3), the WHERE clause becomes:

```sql
WHERE (owner_org_id IS NULL OR owner_org_id = $1)
```

Example transformation for `list_models`:

```rust
async fn list_models(&self, viewer_org_id: &str) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, Model>(
        "SELECT * FROM models
         WHERE owner_org_id IS NULL OR owner_org_id = $1
         ORDER BY name"
    ).bind(viewer_org_id).fetch_all(&self.pool).await.map_err(|e| e.into())
}
```

Catalog INSERTs bind `owner_org_id` from the struct (which the caller has set to `None` for platform-level or `Some(org_id)` for org-private).

- [ ] **Step 4: Anti-shadowing guard in catalog creates**

In `create_provider`, `create_model`, `create_pricing_policies`, when the incoming `owner_org_id` is `Some(_)` (org-private), first check that no platform-level entry with the same `name`/`slug` exists:

```rust
// Inside create_model, before the INSERT:
if let Some(org_id) = &model.owner_org_id {
    let collision: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM models WHERE name = $1 AND owner_org_id IS NULL"
    ).bind(&model.name).fetch_optional(&self.pool).await?;
    if collision.is_some() {
        return Err(Box::new(OrgError::Other(format!(
            "catalog name reserved at platform level: {}", model.name
        ))));
    }
}
```

(`OrgError::Other` — if you didn't add this variant in Task 1 step 3, add it now: `Other(String)`.)

- [ ] **Step 5: Update `list_users` / `list_users_paginated` to JOIN via members**

```rust
async fn list_users(&self, org_id: &str) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         JOIN members m ON m.user_id = u.id
         WHERE m.org_id = $1
         ORDER BY u.username"
    ).bind(org_id).fetch_all(&self.pool).await.map_err(|e| e.into())
}
```

- [ ] **Step 6: Verify build**

```bash
cargo build -p llm-gateway-storage 2>&1 | tail -30
```

Expected: storage crate compiles clean. Other crates (api, gateway, etc.) will fail — Tasks 6+ fix those.

- [ ] **Step 7: Add migration round-trip test**

At the bottom of `crates/storage/src/postgres.rs` (or in `crates/storage/tests/` if that dir exists; create it if not):

```rust
#[cfg(test)]
mod org_tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn bootstrap_default_org_has_admins_as_owners(pool: PgPool) {
        let storage = PostgresStorage::from_pool(pool);
        let org = storage.get_org_by_slug("default").await.unwrap().unwrap();
        assert_eq!(org.slug, "default");
        // After migration, default org exists; users table may or may not have rows
        // depending on whether seed_data ran. Just assert the org row.
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn catalog_visibility_filter_works(pool: PgPool) {
        let storage = PostgresStorage::from_pool(pool);
        let org_a = storage.create_org(CreateOrg {
            id: "org_a".into(), slug: "a".into(), name: "A".into(),
            owner_id: "u1".into(),
        }).await.unwrap();
        let org_b = storage.create_org(CreateOrg {
            id: "org_b".into(), slug: "b".into(), name: "B".into(),
            owner_id: "u1".into(),
        }).await.unwrap();

        // Platform-level model visible to both orgs
        let _platform = storage.create_model("org_a", Model {
            id: "m_p".into(), name: "gpt-4".into(), owner_org_id: None,
            ..Default::default()
        }).await.unwrap();

        // Org-private model only visible to org_a
        let _private = storage.create_model("org_a", Model {
            id: "m_pv".into(), name: "my-finetune".into(), owner_org_id: Some(org_a.id.clone()),
            ..Default::default()
        }).await.unwrap();

        let visible_to_a = storage.list_models("org_a").await.unwrap();
        let visible_to_b = storage.list_models("org_b").await.unwrap();
        let names_a: Vec<_> = visible_to_a.iter().map(|m| m.name.clone()).collect();
        let names_b: Vec<_> = visible_to_b.iter().map(|m| m.name.clone()).collect();

        assert!(names_a.contains(&"gpt-4".to_string()));
        assert!(names_a.contains(&"my-finetune".to_string()));
        assert!(names_b.contains(&"gpt-4".to_string()));
        assert!(!names_b.contains(&"my-finetune".to_string()));
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn anti_shadowing_rejects_org_private_with_platform_name(pool: PgPool) {
        let storage = PostgresStorage::from_pool(pool);
        let _ = storage.create_org(CreateOrg {
            id: "org_a".into(), slug: "a".into(), name: "A".into(),
            owner_id: "u1".into(),
        }).await.unwrap();

        let _platform = storage.create_model("org_a", Model {
            id: "m_p".into(), name: "gpt-4".into(), owner_org_id: None,
            ..Default::default()
        }).await.unwrap();

        let result = storage.create_model("org_a", Model {
            id: "m_pv".into(), name: "gpt-4".into(), owner_org_id: Some("org_a".into()),
            ..Default::default()
        }).await;

        assert!(result.is_err(), "org-private create with platform name should be rejected");
    }
}
```

Note: `Model { ..Default::default() }` requires `Model` to derive `Default`. If it doesn't already, either add the derive or fill in all fields explicitly. The full list of fields is in `types.rs` Task 3 step 4.

- [ ] **Step 8: Run the new tests**

```bash
cargo test -p llm-gateway-storage -- --nocapture org_tests
```

Expected: 3 passing tests.

- [ ] **Step 9: Commit**

```bash
git add crates/storage/src/postgres.rs crates/storage/tests/ 2>/dev/null
git commit -m "feat(storage): postgres impl of org/member + scoped queries"
```

---

### Task 6: Extend JWT claims

**Files:**
- Modify: `crates/auth/src/lib.rs`

- [ ] **Step 1: Update JwtClaims struct**

Current at lib.rs:55:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}
```

Change to:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub current_org_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_role: Option<String>,
    pub exp: usize,
    pub iat: usize,
}
```

(`role` removed; `current_org_id` and `platform_role` added.)

- [ ] **Step 2: Update `create_jwt` signature**

Before:
```rust
pub fn create_jwt(user_id: &str, role: &str, secret: &str) -> Result<String, String> { ... }
```

After:
```rust
pub fn create_jwt(
    user_id: &str,
    current_org_id: &str,
    platform_role: Option<&str>,
    secret: &str,
) -> Result<String, String> {
    let now = Utc::now();
    let claims = JwtClaims {
        sub: user_id.to_string(),
        current_org_id: current_org_id.to_string(),
        platform_role: platform_role.map(|s| s.to_string()),
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Update unit tests in the same file**

Find any test that calls `create_jwt("uid", "admin", "secret")` and update to `create_jwt("uid", "org_default", Some("platform_admin"), "secret")`. Same for `verify_jwt` round-trip tests.

- [ ] **Step 4: Verify**

```bash
cargo test -p llm-gateway-auth
```

Expected: all auth unit tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/auth/src/lib.rs
git commit -m "feat(auth): JWT claims carry current_org_id + platform_role"
```

---

### Task 7: Implement `org` crate extractors + access tests

**Files:**
- Modify: `crates/org/src/extractors.rs`
- Modify: `crates/org/src/access.rs` (if Step 1 stubbed it)
- Modify: `crates/org/src/types.rs` (remove stub if Step 1 stubbed it)

- [ ] **Step 1: Restore real bodies in `access.rs` and `types.rs`**

If you used stubs in Task 1, replace them with the real bodies from Task 1 steps 4-5.

- [ ] **Step 2: Implement `resolve_org_context`**

```rust
// crates/org/src/extractors.rs
use crate::error::OrgError;
use crate::types::{MemberRole, OrgContext, PlatformRole};
use llm_gateway_auth::JwtClaims;
use llm_gateway_storage::{Member, Storage};

pub async fn resolve_org_context(
    claims: &JwtClaims,
    storage: &dyn Storage,
) -> Result<OrgContext, OrgError> {
    // Phase 1: org_id always comes from claims.current_org_id (no path-based routing yet).
    // Phase 2 will look up via path {org_slug} and require active membership.
    let org_id = claims.current_org_id.clone();

    let member = storage.get_member(&claims.sub, &org_id).await
        .map_err(|e| OrgError::NotFound(format!("member lookup failed: {e}")))?
        .ok_or_else(|| OrgError::NotMember(claims.sub.clone(), org_id.clone()))?;

    // Platform_admin without a member row: in Phase 2 we auto-create a temp row;
    // Phase 1 simplification: platform_admins MUST also have a member row in default org
    // (the migration ensures this for pre-existing admins).
    let platform_role = claims.platform_role.as_deref()
        .and_then(PlatformRole::parse);

    Ok(OrgContext {
        org_id,
        member_role: member.role,
        platform_role,
        group_id: member.group_id,
    })
}
```

- [ ] **Step 3: Write table-driven unit tests for access helpers**

```rust
// crates/org/src/access.rs (append)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemberRole, OrgContext, PlatformRole};

    fn ctx(role: MemberRole, platform: Option<PlatformRole>, group_id: Option<&str>) -> OrgContext {
        OrgContext {
            org_id: "o".into(),
            member_role: role,
            platform_role: platform,
            group_id: group_id.map(String::from),
        }
    }

    #[test]
    fn owner_can_delete_org() {
        assert!(can_delete_org(&ctx(MemberRole::Owner, None, None)));
    }

    #[test]
    fn admin_cannot_delete_org() {
        assert!(!can_delete_org(&ctx(MemberRole::Admin, None, None)));
    }

    #[test]
    fn platform_admin_can_delete_org_even_as_member() {
        assert!(can_delete_org(&ctx(MemberRole::Member, Some(PlatformRole::PlatformAdmin), None)));
    }

    #[test]
    fn member_cannot_invite() {
        assert!(!can_invite_members(&ctx(MemberRole::Member, None, None)));
    }

    #[test]
    fn member_sees_ungrouped_channels() {
        assert!(can_access_channel(&ctx(MemberRole::Member, None, None), None));
    }

    #[test]
    fn member_sees_own_group_channels() {
        assert!(can_access_channel(&ctx(MemberRole::Member, None, Some("g1")), Some("g1")));
    }

    #[test]
    fn member_blocked_from_other_group_channels() {
        assert!(!can_access_channel(&ctx(MemberRole::Member, None, Some("g1")), Some("g2")));
    }

    #[test]
    fn admin_sees_all_channels() {
        assert!(can_access_channel(&ctx(MemberRole::Admin, None, None), Some("anything")));
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p llm-gateway-org
```

Expected: 8 passing tests.

- [ ] **Step 5: Commit**

```bash
git add crates/org/
git commit -m "feat(org): resolve_org_context + permission helper tests"
```

---

### Task 8: Update API auth handlers

**Files:**
- Modify: `crates/api/src/auth.rs`
- Modify: `crates/api/src/lib.rs` (if `AuthResponse` lives there)
- Modify: `crates/api/tests/common/mod.rs`

- [ ] **Step 1: Extend `AuthResponse` and `MeResponse`**

```rust
// crates/api/src/auth.rs
#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserInfo,
    pub current_org: OrgSummary,        // NEW
    pub orgs: Vec<OrgSummary>,          // NEW
}

#[derive(Serialize, Clone)]
pub struct OrgSummary {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub role: String,        // mirrors MemberRole as string
    pub group_id: Option<String>,
}

impl From<llm_gateway_storage::MembershipSummary> for OrgSummary {
    fn from(m: llm_gateway_storage::MembershipSummary) -> Self {
        OrgSummary {
            id: m.org.id,
            slug: m.org.slug,
            name: m.org.name,
            role: m.role.as_str().to_string(),
            group_id: m.group_id,
        }
    }
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub username: String,
    pub platform_role: Option<String>,    // NEW (replaces role)
    pub current_org: OrgSummary,          // NEW
    pub orgs: Vec<OrgSummary>,            // NEW
    pub allow_registration: bool,
}
```

Update `UserInfo` similarly — replace `role: String` with `platform_role: Option<String>`. (If frontend code reads `user.role`, this is a breaking change for the existing UI; the frontend changes are in Task 11.)

- [ ] **Step 2: Update `login` handler**

After verifying credentials and rotating refresh token, look up the user's memberships and pick `current_org`:

```rust
let memberships = state.storage.list_orgs_for_user(&user.id).await?;
let current = memberships.iter()
    .find(|m| m.org.id == user.current_org_id)
    .or_else(|| memberships.first())
    .ok_or_else(|| ApiError::Internal("user has no org membership".into()))?;

let current_org: OrgSummary = current.clone().into();
let orgs: Vec<OrgSummary> = memberships.into_iter().map(Into::into).collect();

let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
let token = create_jwt(&user.id, &current.org.id, platform_role_str, &state.jwt_secret)?;

Ok(Json(AuthResponse {
    token,
    refresh_token: user.refresh_token.clone().unwrap_or_default(),
    user: UserInfo::from(&user),
    current_org,
    orgs,
}))
```

- [ ] **Step 3: Update `register` handler**

Phase 1: auto-create a personal org is **Phase 3**. For Phase 1, registration adds the new user as a member of the `default` org:

```rust
// After create_user:
let default_org = state.storage.get_org_by_slug("default").await?
    .ok_or_else(|| ApiError::Internal("default org missing".into()))?;
state.storage.upsert_member(Member {
    user_id: user.id.clone(),
    org_id: default_org.id.clone(),
    role: MemberRole::Member,
    group_id: None,
    created_by: Some(user.id.clone()),
    created_at: Utc::now(),
}).await?;

// Update user.current_org_id:
let mut updated = user.clone();
updated.current_org_id = Some(default_org.id.clone());
state.storage.update_user(&user.id, UpdateUser { current_org_id: Some(default_org.id.clone()), ..Default::default() }).await?;

// Then proceed as login (Step 2).
```

Leave a TODO comment for Phase 3:
```rust
// TODO(Phase 3): replace the default-org-membership above with auto-creation
// of a personal org whose slug is derived from the username.
```

- [ ] **Step 4: Update `me` handler**

```rust
pub async fn me(...) -> Result<Json<MeResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let user = state.storage.get_user(&claims.sub).await?
        .ok_or_else(|| ApiError::Unauthorized)?;
    let memberships = state.storage.list_orgs_for_user(&claims.sub).await?;
    let current = memberships.iter()
        .find(|m| m.org.id == claims.current_org_id)
        .or_else(|| memberships.first())
        .ok_or_else(|| ApiError::Internal("user has no org membership".into()))?;
    let config = state.storage.get_platform_setting("allow_registration").await?;
    let allow_registration = config.as_deref() == Some("true");

    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        platform_role: user.platform_role.as_ref().map(|p| p.as_str().to_string()),
        current_org: current.clone().into(),
        orgs: memberships.into_iter().map(Into::into).collect(),
        allow_registration,
    }))
}
```

- [ ] **Step 5: Add `switch_org` handler**

```rust
#[derive(Deserialize)]
pub struct SwitchOrgRequest {
    pub org_slug: Option<String>,
    pub org_id: Option<String>,
}

pub async fn switch_org(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SwitchOrgRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let target_org = if let Some(slug) = body.org_slug {
        state.storage.get_org_by_slug(&slug).await?
    } else if let Some(id) = body.org_id {
        state.storage.get_org(&id).await?
    } else {
        return Err(ApiError::BadRequest("org_slug or org_id required".into()));
    }.ok_or_else(|| ApiError::NotFound("org".into()))?;

    let member = state.storage.get_member(&claims.sub, &target_org.id).await?
        .ok_or_else(|| ApiError::Forbidden("not a member of this org".into()))?;

    // Persist new current_org_id
    state.storage.update_user(&claims.sub, UpdateUser {
        current_org_id: Some(target_org.id.clone()),
        ..Default::default()
    }).await?;

    // Reissue token
    let user = state.storage.get_user(&claims.sub).await?.unwrap();
    let token = create_jwt(&claims.sub, &target_org.id, user.platform_role.as_ref().map(|p| p.as_str()), &state.jwt_secret)?;

    let memberships = state.storage.list_orgs_for_user(&claims.sub).await?;
    Ok(Json(AuthResponse {
        token,
        refresh_token: user.refresh_token.clone().unwrap_or_default(),
        user: UserInfo::from(&user),
        current_org: OrgSummary {
            id: target_org.id, slug: target_org.slug, name: target_org.name,
            role: member.role.as_str().to_string(),
            group_id: member.group_id,
        },
        orgs: memberships.into_iter().map(Into::into).collect(),
    }))
}
```

(`ApiError::BadRequest`, `ApiError::NotFound`, `ApiError::Forbidden` — verify these variants exist in `crates/api/src/error.rs`. If not, add them or use the closest existing variants.)

- [ ] **Step 6: Register new routes**

In `crates/api/src/management/mod.rs`, add to `management_router()`:

```rust
.route("/api/v1/orgs", get(auth::list_orgs).post(auth::create_org))
.route("/api/v1/me/current-org", post(auth::switch_org))
```

`list_orgs` and `create_org` are thin handlers that call `state.storage.list_orgs_for_user(&claims.sub)` and `state.storage.create_org(...)` respectively. They reuse `OrgSummary` for responses. `create_org` requires the caller to become the owner:

```rust
pub async fn create_org(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(body): Json<CreateOrgRequest>) -> Result<Json<OrgSummary>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let user = state.storage.get_user(&claims.sub).await?.ok_or(ApiError::Unauthorized)?;
    let org = state.storage.create_org(CreateOrg {
        id: uuid::Uuid::new_v4().to_string(),
        slug: body.slug, name: body.name, owner_id: claims.sub.clone(),
    }).await?;
    state.storage.upsert_member(Member {
        user_id: claims.sub.clone(), org_id: org.id.clone(),
        role: MemberRole::Owner, group_id: None,
        created_by: Some(claims.sub.clone()), created_at: Utc::now(),
    }).await?;
    Ok(Json(OrgSummary { id: org.id, slug: org.slug, name: org.name, role: "owner".into(), group_id: None }))
}
```

- [ ] **Step 7: Update test helpers**

In `crates/api/tests/common/mod.rs`:

```rust
pub fn make_admin_token() -> String {
    create_jwt("admin-id", "org_default", Some("platform_admin"), TEST_JWT_SECRET).unwrap()
}
pub fn make_user_token(id: &str) -> String {
    create_jwt(id, "org_default", None, TEST_JWT_SECRET).unwrap()
}
```

(Previously they used the 2-arg form `create_jwt(id, "admin"|"user", secret)`.)

- [ ] **Step 8: Update existing auth tests**

Update all assertions in `crates/api/tests/test_auth.rs` that check `body.role` to instead check `body.platform_role` or `body.current_org.role`. The login response shape has changed.

- [ ] **Step 9: Verify**

```bash
cargo test -p llm-gateway-api --test test_auth
```

Expected: existing auth tests pass (with the assertion updates from Step 8). New `switch_org` and `create_org` flows work end-to-end.

- [ ] **Step 10: Commit**

```bash
git add crates/api/src/auth.rs crates/api/src/management/mod.rs crates/api/tests/
git commit -m "feat(api): /api/v1/orgs + /me/current-org; AuthResponse carries orgs"
```

---

### Task 9: Update existing management handlers to thread `org_id`

**Files:**
- Modify: `crates/api/src/management/keys.rs`
- Modify: `crates/api/src/management/channels.rs`
- Modify: `crates/api/src/management/providers.rs`
- Modify: `crates/api/src/management/models.rs`
- Modify: `crates/api/src/management/groups.rs`
- Modify: `crates/api/src/management/users.rs`
- Modify: `crates/api/src/management/usage.rs`
- Modify: `crates/api/src/management/audit.rs`
- Modify: `crates/api/src/management/accounts.rs`
- Modify: `crates/api/src/management/pricing_policies.rs`
- (and any other file under `crates/api/src/management/`)

**Pattern for every handler:**

Before:
```rust
pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<...>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let (page, page_size) = pagination.normalized();
    let result = if claims.role == "admin" {
        state.storage.list_keys_paginated(page, page_size).await ...
    } else {
        state.storage.list_keys_paginated_for_user(&claims.sub, page, page_size).await ...
    };
    ...
}
```

After:
```rust
pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<...>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;
    let (page, page_size) = pagination.normalized();

    // Admin-or-above in this org (or platform_admin) sees all keys in the org;
    // regular member sees only their own.
    let result = if can_manage_channels(&ctx) {
        state.storage.list_keys_paginated(&ctx.org_id, page, page_size).await ...
    } else {
        state.storage.list_keys_paginated_for_user(&ctx.org_id, &claims.sub, page, page_size).await ...
    };
    ...
}
```

The transformation rules:

1. After every `require_auth(...)`, add `let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;`
2. Replace `if claims.role == "admin"` with the appropriate `can_*(&ctx)` check:
   - `/admin/providers`, `/admin/channels`, `/admin/models`, `/admin/groups`, `/admin/pricing-policies`, `/admin/users` → `can_manage_channels(&ctx)` (or a more specific helper if it exists)
   - Org settings mutation → `can_manage_org_settings(&ctx)`
   - Member management → `can_invite_members(&ctx)`
   - Org deletion → `can_delete_org(&ctx)`
3. Pass `&ctx.org_id` as the first storage-method argument.
4. For catalog endpoints (providers/models/pricing_policies/provider_models), pass `&ctx.org_id` as `viewer_org_id` (same value, different semantic name).
5. Drop the `require_admin` calls entirely; the `can_*` checks are stricter and replace them.

- [ ] **Step 1: Update `keys.rs`**

Apply the pattern above to: `list_keys`, `create_key`, `get_key`, `update_key`, `delete_key`. The `/admin/*` variant of keys (if any) becomes the same handler — `can_manage_channels` filters appropriately.

- [ ] **Step 2: Update `channels.rs`**

Apply to: `list_channels`, `create_channel`, `get_channel`, `update_channel`, `delete_channel`, `disable_channel_until`, etc.

For `list_channels`, members should only see channels in their group:
```rust
let mut channels = state.storage.list_channels(&ctx.org_id).await?;
if !can_manage_channels(&ctx) {
    channels.retain(|c| can_access_channel(&ctx, c.group_id.as_deref()));
}
```

- [ ] **Step 3: Update `providers.rs`**

Catalog methods take `viewer_org_id`:
```rust
let providers = state.storage.list_providers(&ctx.org_id).await?;
```

Mutations (`create_provider`, `update_provider`, `delete_provider`) additionally check whether the caller has permission to mutate the entry:
```rust
if let Some(owner_org_id) = &provider.owner_org_id {
    // Org-private entry — caller must be admin+ of that org
    if owner_org_id != &ctx.org_id || !can_create_org_catalog(&ctx) {
        return Err(ApiError::Forbidden(...));
    }
} else {
    // Platform-level entry — caller must be platform_admin
    if !can_create_platform_catalog(&ctx) {
        return Err(ApiError::Forbidden(...));
    }
}
```

- [ ] **Step 4: Update `models.rs`, `pricing_policies.rs`**

Same as Step 3.

- [ ] **Step 5: Update `groups.rs`, `users.rs`, `usage.rs`, `audit.rs`, `accounts.rs`**

Tenant tables — apply the standard pattern (Step 1 transformation). `groups` and `users` use `&ctx.org_id` for filtering; `users.rs` list handlers use `state.storage.list_users(&ctx.org_id)`.

- [ ] **Step 6: Update settings handler**

If a `settings.rs` exists, split into platform-settings and org-settings handlers:

```rust
// Platform-level (only platform_admin)
pub async fn get_platform_setting(...) -> Result<Json<...>, ApiError> {
    let claims = require_auth(...)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;
    if !ctx.is_platform_admin() {
        return Err(ApiError::Forbidden(...));
    }
    let value = state.storage.get_platform_setting(&key).await?;
    ...
}

// Org-level (admin+ of org)
pub async fn get_org_setting(...) -> Result<Json<...>, ApiError> {
    let claims = require_auth(...)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;
    if !can_manage_org_settings(&ctx) {
        return Err(ApiError::Forbidden(...));
    }
    let value = state.storage.get_org_setting(&ctx.org_id, &key).await?;
    ...
}
```

If `settings.rs` doesn't exist, the existing `/api/v1/admin/settings` route stays and reads/writes via `get_platform_setting`/`set_platform_setting`.

- [ ] **Step 7: Verify all management tests pass**

```bash
cargo test -p llm-gateway-api --test test_management_keys --test test_management_providers --test test_user_groups --test test_users --test test_settings
```

Expected: all pass. Some test fixtures may need updating (e.g., insert an `org_default` row and an api_key with `org_id='org_default'` instead of unscoped).

- [ ] **Step 8: Commit (one commit per file or one bulk commit, your choice)**

```bash
git add crates/api/src/management/
git commit -m "feat(api): thread org_id through management handlers"
```

---

### Task 10: Update proxy handler

**Files:**
- Modify: `crates/api/src/proxy.rs`
- Modify: `crates/api/src/extractors.rs` (if api_key auth helper lives there)

- [ ] **Step 1: Verify `get_key_by_hash` returns org_id**

In `crates/storage/src/postgres.rs`, the `get_key_by_hash` method's `SELECT` and `query_as::<_, ApiKey>` already include all columns. Since `ApiKey` now has `org_id: String` (Task 3 step 3), and the column exists (Task 2 migration), the existing query assembles it correctly. No code change needed if the SELECT is `SELECT *` or includes the new column.

If the query is column-explicit, add `org_id` to the column list.

- [ ] **Step 2: Thread `key.org_id` through the proxy**

The proxy entry points (`proxy`, `proxy_with_protocol`, `messages`, `responses`) currently call `state.storage.get_key_by_hash(&hash)` to resolve the api_key. After that resolution, every downstream call should use `key.org_id`:

Find every `state.storage.*` call inside `proxy.rs` that hits a tenant table (channels, channel_models, usage_records, audit_logs, accounts, transactions, key_model_rate_limits) and pass `&key.org_id` as the first argument.

Specifically:
- `state.storage.list_enabled_channels_by_provider(...)` → `state.storage.list_enabled_channels_by_provider(&key.org_id, ...)`
- `state.storage.record_usage(...)` → `state.storage.record_usage(&key.org_id, ...)`
- `state.storage.insert_log(...)` → `state.storage.insert_log(&key.org_id, ...)`
- `state.storage.get_account_by_user_id(...)` → `state.storage.get_account_by_user_id(&key.org_id, ...)`
- `state.storage.deduct_balance(...)` → `state.storage.deduct_balance(&key.org_id, ...)`

Audit/usage construction (`AuditTask`, `UsageRecord`) — set the `.org_id` field from `key.org_id` when building the struct.

- [ ] **Step 3: Set `actor_is_platform_admin` on audit writes**

Inside proxy.rs, audit writes happen via `dispatch_audit_task`. The api_key path never produces a platform_admin audit row (api_keys don't carry platform role), so `actor_is_platform_admin: false` for all proxy-driven audit writes. Verify this field is set (default `false` from migration is correct for INSERTs that don't specify it).

- [ ] **Step 4: Verify proxy integration tests**

```bash
cargo test -p llm-gateway-api --test test_proxy 2>/dev/null || cargo test -p llm-gateway-api proxy
```

If there's no dedicated proxy test file, run all api tests:

```bash
cargo test -p llm-gateway-api
```

Expected: all pass. Test fixtures that create api_keys need to also populate `org_id` (see Task 9 step 7 fixture note).

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/proxy.rs crates/api/src/extractors.rs
git commit -m "feat(api): proxy threads key.org_id through routing/usage/audit"
```

---

### Task 11: Frontend store + types

**Files:**
- Modify: `web/src/types/index.ts`
- Modify: `web/src/stores/authStore.ts`
- Modify: `web/src/api/auth.ts`

- [ ] **Step 1: Add `OrgSummary` type**

```ts
// web/src/types/index.ts
export interface OrgSummary {
  id: string
  slug: string
  name: string
  role: 'owner' | 'admin' | 'member'
  group_id: string | null
}
```

- [ ] **Step 2: Extend `AuthResponse`, `UserInfo`, `MeResponse`**

```ts
export interface AuthResponse {
  token: string
  refresh_token: string
  user: UserInfo
  current_org: OrgSummary        // NEW
  orgs: OrgSummary[]             // NEW
}

export interface UserInfo {
  id: string
  username: string
  platform_role: 'platform_admin' | null   // was: role: 'admin' | 'user'
}

// If MeResponse exists as a separate type, extend it the same way.
```

- [ ] **Step 3: Update `authStore.ts`**

```ts
interface AuthState {
  user: User | null
  currentOrg: OrgSummary | null     // NEW
  orgs: OrgSummary[]                // NEW
  isLoading: boolean

  login(req: LoginRequest): Promise<void>
  register(req: RegisterRequest): Promise<void>
  logout(): void
  setUser(user: User): void
  setCurrentOrg(org: OrgSummary): void   // NEW
  refreshOrgs(): Promise<void>           // NEW
}

export const useAuthStore = create<AuthState>((set, get) => ({
  user: null,
  currentOrg: null,
  orgs: [],
  isLoading: false,

  login: async (req) => {
    const resp = await apiLogin(req)
    setToken(resp.token)
    setRefreshToken(resp.refresh_token)
    set({
      user: resp.user,
      currentOrg: resp.current_org,
      orgs: resp.orgs,
    })
  },

  register: async (req) => {
    const resp = await apiRegister(req)
    setToken(resp.token)
    setRefreshToken(resp.refresh_token)
    set({
      user: resp.user,
      currentOrg: resp.current_org,
      orgs: resp.orgs,
    })
  },

  logout: () => {
    clearToken()
    clearRefreshToken()
    set({ user: null, currentOrg: null, orgs: [] })
  },

  setUser: (user) => set({ user }),
  setCurrentOrg: (org) => set({ currentOrg: org }),

  refreshOrgs: async () => {
    const me = await getMe()
    set({ user: me.user, currentOrg: me.current_org, orgs: me.orgs })
  },
}))
```

(`User` type is whatever wraps `UserInfo` — adapt as needed.)

- [ ] **Step 4: Leave OrgSwitcher UI for Phase 2**

Per spec, Phase 1 frontend has no OrgSwitcher. `currentOrg`/`orgs` are stored but not yet rendered. Add a TODO comment in `web/src/components/Layout.tsx`:

```tsx
// TODO(Phase 2): render OrgSwitcher here using useAuthStore.currentOrg + .orgs
```

- [ ] **Step 5: Update tests**

`web/src/api/auth.test.ts` (or similar) — update mock responses to include `current_org` and `orgs`. Update assertions that checked `user.role` to check `user.platform_role` instead.

- [ ] **Step 6: Verify**

```bash
source ~/.nvm/nvm.sh && cd web && npm run build
```

Expected: TypeScript check passes. Build succeeds.

```bash
npm test
```

Expected: all Vitest tests pass.

- [ ] **Step 7: Commit**

```bash
cd web && git add src/types/ src/stores/ src/api/ src/components/Layout.tsx 2>/dev/null
git commit -m "feat(web): authStore carries currentOrg + orgs (Phase 1; no UI yet)"
```

---

### Task 12: End-to-end verification

**Files:** (no file changes — verification only)

- [ ] **Step 1: Full backend test suite**

```bash
cargo test --workspace
```

Expected: all tests pass. If specific tests fail due to the JwtClaims signature change, revisit Task 6 step 3.

- [ ] **Step 2: Full frontend test + build**

```bash
source ~/.nvm/nvm.sh && cd web && npm test && npm run build
```

Expected: tests pass, build clean.

- [ ] **Step 3: Manual smoke test**

Start the dev servers:

```bash
cargo run &
source ~/.nvm/nvm.sh && cd web && npm run dev
```

In a browser:
1. Open `http://localhost:5173`
2. Log in with an existing admin username/password
3. Verify the dashboard loads, existing keys/channels/providers/models are visible
4. Verify creating a new key works
5. Make a test API call: `curl -H "Authorization: Bearer <your-api-key>" http://localhost:8080/v1/chat/completions -d '{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}'`
6. Verify the audit log page shows the new request

Expected: behavior identical to pre-Phase-1. No new UI surfaces.

- [ ] **Step 4: Database spot-check**

```bash
docker compose exec postgres psql -U postgres -d llm_gateway -c "
  SELECT slug, name, owner_id FROM orgs;
  SELECT COUNT(*) FROM members WHERE org_id = 'org_default';
  SELECT COUNT(*) FROM api_keys WHERE org_id = 'org_default';
  SELECT COUNT(*) FROM channels WHERE org_id = 'org_default';
  SELECT COUNT(*) FROM providers WHERE owner_org_id IS NULL;
  SELECT COUNT(*) FROM models WHERE owner_org_id IS NULL;
"
```

Expected:
- 1 org (`default`, with non-null owner_id)
- members count = users count
- api_keys/channels count matches pre-migration totals
- providers/models all NULL owner_org_id (platform-level)

- [ ] **Step 5: Final commit (if any cleanup)**

If any test fixes or doc updates accumulated, commit them. Otherwise, no commit needed.

```bash
git status
git log --oneline -15
```

Confirm the branch has commits for each task and the tree is clean.

---

## Self-Review Notes

**Spec coverage check** — every Phase 1 deliverable from the spec's "### Phase 1" subsection maps to a task:
- Migration → Task 2 ✓
- New crate `org` → Task 1 + Task 7 ✓
- Storage trait `org_id` everywhere → Task 4 + Task 5 ✓
- API handlers read `current_org_id` → Task 8 + Task 9 ✓
- JWT claims → Task 6 ✓
- `/api/v1/orgs`, `/api/v1/me/current-org` → Task 8 step 6 ✓
- Frontend OrgSwitcher "added but only one org visible" → Task 11 step 4 (deferred the visible UI to Phase 2; Phase 1 only stores the data) — **deviation from spec, documented**

**Placeholder scan** — no TBD / TODO outside the explicit Phase 3 hooks in Task 8 step 3 and Task 11 step 4. Those are intentional forward references, not gaps.

**Type consistency** — `OrgContext`, `OrgSummary`, `MembershipSummary`, `MemberRole`, `PlatformRole` referenced consistently across tasks. `OrgError::Other` added in Task 5 step 4 if not already present in Task 1 step 3 — flagged inline.

**Risks worth flagging to the implementer** (call out in PR description):
1. ~70 storage methods gain a parameter; downstream crates that import the trait may have call sites in `gateway/` or `usage-worker/` / `audit-worker/` that also need updating. Run `cargo build --workspace` after Task 5 to surface them.
2. The first `.down.sql` in the repo. If the team prefers to stay forward-only, delete `20260708000000_saas_orgs.down.sql` and remove the spec reference to it.
3. Frontend Phase 1 deviates from the spec's "OrgSwitcher added but only one org visible" — Phase 1 stores the data but renders nothing. If the team wants the visible-but-disabled OrgSwitcher in Phase 1, add a Task 11.5 to `web/src/components/Layout.tsx`.
