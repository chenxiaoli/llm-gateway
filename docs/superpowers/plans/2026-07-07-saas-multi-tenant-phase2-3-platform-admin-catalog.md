# SaaS Multi-Tenant Orgs — Phase 2, Plan 2.3: Platform-admin + Catalog Filter

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Prerequisite:** Plans 2.1 (URL migration) and 2.2 (members + org settings) shipped. The middleware chain, `OrgContext`, members endpoints, and org CRUD are all in place.

**Goal:** Add the two remaining Phase 2 deliverables: platform-admin impersonation (so hosting-company staff can debug an org's data without `if platform_admin { bypass }` branches scattered through handlers) and the org-private catalog (so an org can have its own providers/models/pricing policies without shadowing the platform-level entries).

**Architecture:** Platform-admin impersonation lives in the membership layer: when a request comes in with `platform_role = platform_admin` but no member row exists, the layer creates a temp row (`role = admin`, `created_by = 'system'`) and lets the request proceed as if the admin were a regular org admin. A janitor task cleans up stale temp rows. Catalog visibility is enforced in the storage trait via a `(owner_org_id IS NULL OR owner_org_id = $1)` filter that every `list_*` and `get_*` call already uses (from Phase 1) — what's new in this plan is the **anti-shadowing** write check (rejecting `create_model(owner_org_id = X, name = "gpt-4")` when a platform-level `gpt-4` exists) and the **`can_mutate_catalog_entry`** access check in handlers.

**Tech Stack:** Rust (Axum, tokio cron), sqlx, React + TypeScript.

**Spec reference:** `docs/superpowers/specs/2026-07-07-saas-multi-tenant-orgs-design.md` — Platform admin impersonation (lines 747-763), Anti-shadowing (lines 214, 938), Catalog visibility filter (lines 216-226), `can_mutate_catalog_entry` (lines 631-633), Phase 2 deliverables (lines 985-990).

---

## File Structure

### Create

**Backend**
- `crates/api/src/janitor.rs` — background task that deletes stale platform-admin temp member rows
- `crates/storage/src/catalog.rs` — `CatalogNameReserved` error + anti-shadowing check helper

**Frontend**
- `web/src/components/CatalogFilter.tsx` — "Platform" vs "Ours" segmented control
- `web/src/components/ImpersonationBanner.tsx` — top-of-page banner shown when a platform_admin is operating in an org they don't own

### Modify

**Backend**
- `crates/api/src/middleware.rs` — `membership_layer` gains platform-admin impersonation path
- `crates/api/src/management/providers.rs`, `models.rs`, `pricing_policies.rs`, `channel_models.rs` — call `can_mutate_catalog_entry` before writes; surface "Platform" vs "Ours" in list responses
- `crates/storage/src/lib.rs` — catalog `create_*` methods gain anti-shadowing check
- `crates/storage/src/postgres.rs` — implement anti-shadowing (EXISTS check before INSERT)
- `crates/storage/src/types.rs` — add `CatalogScope { owner_org_id: Option<String> }` field to catalog `Create*` structs if not already present from Phase 1
- `crates/audit/src/lib.rs` — `AuditEvent` sets `actor_is_platform_admin = true` when context's `platform_role` is `Some(PlatformAdmin)`
- `crates/gateway/src/main.rs` — spawn the janitor task alongside the server

**Frontend**
- `web/src/pages/Providers.tsx`, `Models.tsx`, `PricingPolicies.tsx` — add `CatalogFilter` and pass it to the API client
- `web/src/api/providers.ts`, `models.ts`, `pricing-policies.ts` — list endpoints accept a `scope: 'platform' | 'org' | 'all'` query param
- `web/src/components/Layout.tsx` — render `ImpersonationBanner` above the main content
- `web/src/hooks/useProviders.ts`, etc. — `queryKey` includes the scope

### Migration

Small one — adds a `last_seen` column to `members` so the janitor can identify stale temp rows.

- Create: `crates/storage/migrations/postgres/20260801000000_members_last_seen.sql`

---

## Deployment Notes

**Not a breaking change.** Adds new behavior (impersonation, anti-shadowing, catalog filter) without changing existing routes. The catalog filter's "All" option preserves the v2.1.0 default of "platform + org" visibility, so existing UI flows see no difference.

**Operational note:** Platform-admin impersonation shows up in audit logs with `actor_is_platform_admin = true`. The compliance team should know this column exists before Plan 2.3 ships, not after.

---

### Task 1: Migration — add `members.last_seen`

**Files:**
- Create: `crates/storage/migrations/postgres/20260801000000_members_last_seen.sql`

The janitor uses `last_seen` to decide which temp rows to clean up. Existing members get `last_seen = NOW()` as a one-time backfill.

- [ ] **Step 1: Write the migration**

```sql
-- 20260801000000_members_last_seen.sql
ALTER TABLE members ADD COLUMN last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Backfill existing rows to NOW() — their next request will update last_seen.
UPDATE members SET last_seen = NOW() WHERE last_seen IS NULL;

CREATE INDEX idx_members_system_impersonation_last_seen
    ON members(last_seen)
    WHERE created_by = 'system';
```

- [ ] **Step 2: Test the migration round-trip**

```bash
cargo test -p llm-gateway-storage -- --nocapture migrations
```

If no migration test framework exists yet, manually verify by running the migration against a test DB:

```bash
psql -U llm_gateway -d llm_gateway_test -f crates/storage/migrations/postgres/20260801000000_members_last_seen.sql
psql -U llm_gateway -d llm_gateway_test -c "\d members"
# Expected: last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW()
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/postgres/20260801000000_members_last_seen.sql
git commit -m "feat(storage): members.last_seen for platform-admin impersonation janitor"
```

---

### Task 2: Backend — Update `membership_layer` to bump `last_seen` on every request

**Files:**
- Modify: `crates/api/src/middleware.rs`

Every authenticated request touches `membership_layer`; we use that as the natural place to update `last_seen`. Cheap write — ~1ms — and only runs once per request.

- [ ] **Step 1: Write failing test**

Append to `middleware::tests`:

```rust
#[tokio::test]
async fn membership_layer_updates_last_seen() {
    let state = make_state_with_members("secret",
        vec![("user-1", "org_default", MemberRole::Member)]).await;

    // Capture last_seen before the request
    let before: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT last_seen FROM members WHERE user_id = $1 AND org_id = $2",
    )
    .bind("user-1")
    .bind("org_default")
    .fetch_one(&state.pool)
    .await
    .unwrap();

    // Wait a moment so the timestamp will differ
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let app = Router::new()
        .route("/{org_slug}/keys", get(|| async { "ok" }))
        .layer(from_fn_with_state(state.clone(), membership_layer));

    let token = make_token("user-1", "org_default");
    let _ = app
        .oneshot(
            Request::builder()
                .uri("/default/keys")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let after: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT last_seen FROM members WHERE user_id = $1 AND org_id = $2",
    )
    .bind("user-1")
    .bind("org_default")
    .fetch_one(&state.pool)
    .await
    .unwrap();

    assert!(after > before);
}
```

- [ ] **Step 2: Run test — expect FAIL**

- [ ] **Step 3: Update membership_layer**

In `crates/api/src/middleware.rs`, after constructing `ctx`:

```rust
// Bump last_seen (cheap write; runs once per request).
// Failures here are non-fatal — log and continue.
if let Err(e) = state.storage.touch_member_last_seen(&claims.sub, &org.id).await {
    tracing::warn!("failed to update members.last_seen: {e}");
}

req.extensions_mut().insert(ctx);
Ok(next.run(req).await)
```

Add the storage method:

`crates/storage/src/lib.rs`:

```rust
async fn touch_member_last_seen(&self, user_id: &str, org_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

`crates/storage/src/postgres.rs`:

```rust
async fn touch_member_last_seen(&self, user_id: &str, org_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query!(
        "UPDATE members SET last_seen = NOW() WHERE user_id = $1 AND org_id = $2",
        user_id,
        org_id
    )
    .execute(&self.pool)
    .await
    .map(|_| ())
    .map_err(Into::into)
}
```

- [ ] **Step 4: Run test — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/middleware.rs crates/storage/src/
git commit -m "feat(api,storage): touch_member_last_seen on every authenticated request"
```

---

### Task 3: Backend — Platform-admin impersonation in `membership_layer`

**Files:**
- Modify: `crates/api/src/middleware.rs`

When the JWT has `platform_role = platform_admin` and no member row exists, the layer creates a temp member with `role = admin, created_by = 'system'`. Existing temp rows are reused (idempotent `ON CONFLICT`).

- [ ] **Step 1: Write failing test**

```rust
use llm_gateway_storage::{Member, MemberRole};

#[tokio::test]
async fn platform_admin_without_membership_gets_temp_admin_row() {
    let state = make_state_with_orgs("secret", vec!["org_a", "org_b"]).await;
    // Note: user "admin-1" is a platform_admin but NOT in org_b's members table.

    let app = Router::new()
        .route(
            "/{org_slug}/probe",
            get(|req: Request| async move {
                let ctx = req.extensions().get::<OrgContext>().unwrap();
                format!("{:?}:{:?}", ctx.member_role, ctx.platform_role)
            }),
        )
        .layer(from_fn_with_state(state.clone(), membership_layer));

    let token = make_token_with_platform_role("admin-1", "org_a", "platform_admin");
    let body = axum::body::to_bytes(
        app.oneshot(
            Request::builder()
                .uri("/org-b/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .into_body(),
        usize::MAX,
    )
    .await
    .unwrap();
    assert_eq!(&body[..], b"Admin:Some(PlatformAdmin)");

    // Verify the temp row exists
    let temp_member: Option<(String, String)> = sqlx::query_as(
        "SELECT role, created_by FROM members WHERE user_id = $1 AND org_id = $2",
    )
    .bind("admin-1")
    .bind("org_b")
    .fetch_optional(&state.pool)
    .await
    .unwrap();
    let (role, created_by) = temp_member.unwrap();
    assert_eq!(role, "admin");
    assert_eq!(created_by, "system");
}

#[tokio::test]
async fn platform_admin_with_existing_membership_uses_real_row() {
    let state = make_state_with_members("secret",
        vec![("admin-1", "org_a", MemberRole::Owner)]).await;
    // admin-1 is already an owner of org_a — should NOT get a temp row created.

    let app = Router::new()
        .route("/{org_slug}/probe", get(|| async { "ok" }))
        .layer(from_fn_with_state(state.clone(), membership_layer));

    let token = make_token_with_platform_role("admin-1", "org_a", "platform_admin");
    let _ = app
        .oneshot(
            Request::builder()
                .uri("/org-a/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Confirm no temp row was created in any other org
    let temp_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM members WHERE user_id = $1 AND created_by = 'system'",
    )
    .bind("admin-1")
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(temp_count, 0);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Update membership_layer**

Replace the membership check section:

```rust
let member = match state.storage.get_member(&claims.sub, &org.id).await {
    Ok(Some(m)) => m,
    Ok(None) => {
        // No membership row. If the caller is a platform_admin, create a
        // temp row with admin privileges. Otherwise 403.
        if claims.platform_role.as_deref() == Some("platform_admin") {
            state
                .storage
                .upsert_member(Member {
                    user_id: claims.sub.clone(),
                    org_id: org.id.clone(),
                    role: MemberRole::Admin,
                    group_id: None,
                    created_by: Some("system".to_string()),
                    created_at: chrono::Utc::now(),
                })
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
        } else {
            return Err(ApiError::Forbidden);
        }
    }
    Err(e) => return Err(ApiError::Internal(format!("storage error: {e}"))),
};

let ctx = OrgContext {
    user_id: claims.sub.clone(),
    org_id: org.id.clone(),
    member_role: member.role,
    platform_role: claims.platform_role.as_deref().map(|_| PlatformRole::PlatformAdmin),
    group_id: member.group_id,
};
```

`upsert_member` must implement `ON CONFLICT (user_id, org_id) DO UPDATE SET role = EXCLUDED.role, created_by = EXCLUDED.created_by` so a second request reuses the temp row instead of erroring on PK violation.

- [ ] **Step 4: Run tests — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/middleware.rs
git commit -m "feat(api): platform-admin impersonation via temp member row"
```

---

### Task 4: Backend — Janitor task to clean up stale temp rows

**Files:**
- Create: `crates/api/src/janitor.rs`
- Modify: `crates/gateway/src/main.rs`

A background tokio task wakes every 5 minutes and deletes rows where `created_by = 'system' AND last_seen < NOW() - INTERVAL '1 hour'`. The threshold is intentionally generous — the janitor is a safety net, not the primary exit signal.

- [ ] **Step 1: Write failing test**

`crates/api/src/janitor.rs`:

```rust
use std::sync::Arc;
use chrono::{Duration, Utc};
use crate::AppState;

/// Delete temp member rows older than the threshold. Runs on a tokio task
/// spawned by the gateway. Returns the count of deleted rows for logging.
pub async fn cleanup_stale_impersonations(
    state: &Arc<AppState>,
    older_than: Duration,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let cutoff = Utc::now() - older_than;
    let result = sqlx::query!(
        "DELETE FROM members WHERE created_by = 'system' AND last_seen < $1",
        cutoff
    )
    .execute(&state.storage.pool)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::make_state_with_impersonation_rows;

    #[tokio::test]
    async fn deletes_only_old_system_rows() {
        let state = make_state_with_impersonation_rows(vec![
            ("admin-1", "org_a", Utc::now() - Duration::minutes(5)),   // fresh — keep
            ("admin-2", "org_a", Utc::now() - Duration::hours(2)),     // stale — delete
            ("admin-1", "org_b", Utc::now() - Duration::hours(3)),     // stale — delete
        ]).await;

        let deleted = cleanup_stale_impersonations(&state, Duration::hours(1)).await.unwrap();
        assert_eq!(deleted, 2);

        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM members WHERE created_by = 'system'",
        )
        .fetch_one(&state.storage.pool)
        .await
        .unwrap();
        assert_eq!(remaining, 1);
    }

    #[tokio::test]
    async fn preserves_real_memberships() {
        let state = make_state_with_impersonation_rows(vec![
            ("admin-1", "org_a", Utc::now() - Duration::hours(5)),
        ]).await;
        // Also seed a real membership (created_by != 'system')
        sqlx::query!(
            "INSERT INTO members (user_id, org_id, role, created_by) VALUES ($1, $2, 'member', $3)",
            "real-user",
            "org_a",
            "real-user"
        )
        .execute(&state.storage.pool)
        .await
        .unwrap();

        let deleted = cleanup_stale_impersonations(&state, Duration::hours(1)).await.unwrap();
        assert_eq!(deleted, 1);  // only the system row

        let real_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM members WHERE user_id = 'real-user'",
        )
        .fetch_one(&state.storage.pool)
        .await
        .unwrap();
        assert_eq!(real_count, 1);
    }
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Implement the function**

(Already in the file above.)

- [ ] **Step 4: Spawn the task from gateway**

`crates/gateway/src/main.rs` (add after the server starts):

```rust
// Spawn platform-admin impersonation janitor
{
    let state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        interval.tick().await; // skip the immediate first tick
        loop {
            interval.tick().await;
            match llm_gateway_api::janitor::cleanup_stale_impersonations(
                &state,
                chrono::Duration::hours(1),
            ).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("janitor: removed {n} stale platform-admin temp rows"),
                Err(e) => tracing::warn!("janitor: failed: {e}"),
            }
        }
    });
}
```

If `state.storage.pool` isn't directly accessible (encapsulation), add a method `pool()` or `cleanup_stale_impersonations()` directly to the `Storage` trait and call it via `state.storage.cleanup_stale_impersonations(...)`.

- [ ] **Step 5: Run tests — expect PASS**

```bash
cargo test -p llm-gateway-api janitor::tests -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/janitor.rs crates/api/src/lib.rs crates/gateway/src/main.rs
git commit -m "feat(api): janitor cleans up stale platform-admin temp rows"
```

---

### Task 5: Backend — Audit log records `actor_is_platform_admin`

**Files:**
- Modify: `crates/audit/src/lib.rs`
- Modify: `crates/api/src/workers.rs` (or wherever audit events are constructed)

Phase 1 added the `actor_is_platform_admin: bool` column. Now we populate it.

- [ ] **Step 1: Write failing test**

`crates/audit/src/lib.rs` (test module):

```rust
#[tokio::test]
#[sqlx::test(fixtures("audit_seed"))]
async fn audit_event_records_platform_admin_flag(pool: sqlx::PgPool) {
    let storage = PostgresStorage::new(pool);
    let event = AuditEvent {
        request_id: "req-1".into(),
        org_id: "org_default".into(),
        user_id: "admin-1".into(),
        actor_is_platform_admin: true,  // NEW
        // ... rest of the fields
    };
    storage.write_audit_event(&event).await.unwrap();

    let flag: bool = sqlx::query_scalar(
        "SELECT actor_is_platform_admin FROM audit_logs WHERE request_id = $1",
    )
    .bind("req-1")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(flag);
}
```

- [ ] **Step 2: Run test — expect FAIL**

- [ ] **Step 3: Update AuditEvent construction**

`crates/audit/src/lib.rs`:

```rust
pub struct AuditEvent {
    pub request_id: String,
    pub org_id: String,
    pub user_id: String,
    pub actor_is_platform_admin: bool,  // NEW
    // ... existing fields
}
```

In `crates/api/src/workers.rs` (or wherever audit events are built from request context), populate the field from `OrgContext.platform_role`:

```rust
let event = AuditEvent {
    request_id,
    org_id: ctx.org_id.clone(),
    user_id: ctx.user_id.clone(),
    actor_is_platform_admin: ctx.platform_role == Some(PlatformRole::PlatformAdmin),
    // ...
};
```

Proxy-path audit events use the resolved api_key's org_id (unchanged from Phase 1). They set `actor_is_platform_admin = false` (proxy API never impersonates).

- [ ] **Step 4: Run test — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/audit/src/lib.rs crates/api/src/
git commit -m "feat(audit): record actor_is_platform_admin on every event"
```

---

### Task 6: Backend — Anti-shadowing check in catalog `create_*` storage methods

**Files:**
- Create: `crates/storage/src/catalog.rs`
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/postgres.rs`

Anti-shadowing rule: an org cannot create an entry whose `name`/`slug` matches an existing platform-level entry. This guards against confusion like an org creating a model named `gpt-4` that hides the real one.

- [ ] **Step 1: Write failing test**

`crates/storage/src/catalog.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{make_storage, seed_platform_model};

    #[tokio::test]
    #[sqlx::test]
    async fn create_org_model_rejects_platform_name_collision(pool: sqlx::PgPool) {
        let storage = make_storage(pool);
        seed_platform_model(&storage, "gpt-4").await;  // owner_org_id = NULL

        let result = storage
            .create_model(CreateModel {
                owner_org_id: Some("org_a".into()),
                name: "gpt-4".into(),
                // ... other fields
            })
            .await;

        let err = result.unwrap_err().to_string();
        assert!(err.contains("CatalogNameReserved") || err.contains("reserved by a platform-level entry"));
    }

    #[tokio::test]
    #[sqlx::test]
    async fn create_org_model_succeeds_for_unique_name(pool: sqlx::PgPool) {
        let storage = make_storage(pool);
        seed_platform_model(&storage, "gpt-4").await;

        let result = storage
            .create_model(CreateModel {
                owner_org_id: Some("org_a".into()),
                name: "my-finetune".into(),
                // ... other fields
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[sqlx::test]
    async fn create_platform_model_ignores_org_entries(pool: sqlx::PgPool) {
        let storage = make_storage(pool);
        // Org-private entry first
        storage.create_model(CreateModel {
            owner_org_id: Some("org_a".into()),
            name: "my-finetune".into(),
            // ...
        }).await.unwrap();

        // Platform-level entry with same name should be allowed — different scope
        let result = storage
            .create_model(CreateModel {
                owner_org_id: None,
                name: "my-finetune".into(),
                // ...
            })
            .await;

        assert!(result.is_ok());
    }
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Add `CatalogError` and helper**

`crates/storage/src/catalog.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("name '{0}' is reserved by a platform-level entry — cannot shadow it with an org-private one")]
    NameReserved(String),

    #[error("slug '{0}' is reserved by a platform-level entry")]
    SlugReserved(String),
}

/// Returns Ok(()) if the name is not in use by a platform-level entry.
/// Called from create_provider/create_model/create_pricing_policy.
pub async fn check_name_not_reserved_for_platform(
    pool: &sqlx::PgPool,
    table: &str,
    name: &str,
) -> Result<(), CatalogError> {
    let exists: Option<(String,)> = sqlx::query_as(&format!(
        "SELECT name FROM {table} WHERE name = $1 AND owner_org_id IS NULL"
    ))
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| CatalogError::NameReserved(format!("lookup failed: {e}")))?;

    if exists.is_some() {
        return Err(CatalogError::NameReserved(name.to_string()));
    }
    Ok(())
}

/// Same as above but for slug-keyed tables (providers).
pub async fn check_slug_not_reserved_for_platform(
    pool: &sqlx::PgPool,
    table: &str,
    slug: &str,
) -> Result<(), CatalogError> {
    let exists: Option<(String,)> = sqlx::query_as(&format!(
        "SELECT slug FROM {table} WHERE slug = $1 AND owner_org_id IS NULL"
    ))
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| CatalogError::SlugReserved(format!("lookup failed: {e}")))?;

    if exists.is_some() {
        return Err(CatalogError::SlugReserved(slug.to_string()));
    }
    Ok(())
}
```

Re-export from `crates/storage/src/lib.rs`:

```rust
pub mod catalog;
pub use catalog::{CatalogError, check_name_not_reserved_for_platform, check_slug_not_reserved_for_platform};
```

- [ ] **Step 4: Wire into `create_model` / `create_provider` / `create_pricing_policy`**

In `crates/storage/src/postgres.rs`, modify each create method:

```rust
async fn create_model(&self, model: CreateModel) -> Result<Model, StorageError> {
    if let Some(ref _org_id) = model.owner_org_id {
        check_name_not_reserved_for_platform(&self.pool, "models", &model.name).await
            .map_err(|e| StorageError::Other(e.to_string()))?;
    }
    // ... existing INSERT
}
```

Repeat for `create_provider` (slug-keyed), `create_pricing_policy` (name-keyed), `create_provider_model` (junction; check by id reference is fine).

If `StorageError::Other` doesn't exist, add it as a `#[error(transparent)] Other(#[from] Box<dyn std::error::Error + Send + Sync>)` variant.

- [ ] **Step 5: Run tests — expect PASS**

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/catalog.rs crates/storage/src/lib.rs crates/storage/src/postgres.rs
git commit -m "feat(storage): anti-shadowing — reject org-private catalog creates matching platform names"
```

---

### Task 7: Backend — `can_mutate_catalog_entry` access check in catalog handlers

**Files:**
- Modify: `crates/org/src/access.rs`
- Modify: `crates/api/src/management/providers.rs`
- Modify: `crates/api/src/management/models.rs`
- Modify: `crates/api/src/management/pricing_policies.rs`
- Modify: `crates/api/src/management/channel_models.rs`

Three access rules:
- Mutating a **platform-level** entry (owner_org_id = NULL) requires `platform_admin`.
- Mutating an **org-private** entry requires `members.role in [admin, owner]` for that org.
- Reading either kind is allowed for any member (visibility is filtered at the storage layer).

- [ ] **Step 1: Write failing test**

Append to `crates/org/src/access.rs` tests:

```rust
#[test_case(MemberRole::Owner,  Some(org_a), Some(org_a), true;  "owner of org A mutating org A's private entry")]
#[test_case(MemberRole::Admin,  Some(org_a), Some(org_a), true;  "admin of org A mutating org A's private entry")]
#[test_case(MemberRole::Member, Some(org_a), Some(org_a), false; "member of org A cannot mutate org A's private entry")]
#[test_case(MemberRole::Owner,  Some(org_a), Some(org_b), false; "owner of org A cannot mutate org B's private entry")]
#[test_case(MemberRole::Owner,  None,        None,        false; "owner cannot mutate platform-level entry without platform_admin")]
fn test_can_mutate_org_catalog(role: MemberRole, ctx_org: Option<&str>, entry_org: Option<&str>, expected: bool) {
    let ctx = OrgContext {
        user_id: "u".into(),
        org_id: ctx_org.unwrap_or("org_a").into(),
        member_role: role,
        platform_role: None,
        group_id: None,
    };
    let entry_org_id = entry_org.map(String::from);
    assert_eq!(can_mutate_catalog_entry(&ctx, entry_org_id.as_deref()), expected);
}

#[test]
fn platform_admin_can_mutate_any_entry() {
    let ctx = OrgContext {
        user_id: "admin-1".into(),
        org_id: "org_a".into(),
        member_role: MemberRole::Member,  // even as a member-role temp row
        platform_role: Some(PlatformRole::PlatformAdmin),
        group_id: None,
    };
    assert!(can_mutate_catalog_entry(&ctx, None));           // platform-level
    assert!(can_mutate_catalog_entry(&ctx, Some("org_a")));  // org A private
    assert!(can_mutate_catalog_entry(&ctx, Some("org_b")));  // org B private (cross-org!)
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Implement `can_mutate_catalog_entry`**

`crates/org/src/access.rs`:

```rust
/// Returns true if the user may write to a catalog entry with the given `owner_org_id`.
///
/// - Platform-level entry (None) → only platform admins.
/// - Org-private entry (Some(org_id)) → admin/owner of that org, or platform admin.
pub fn can_mutate_catalog_entry(ctx: &OrgContext, entry_owner_org_id: Option<&str>) -> bool {
    if ctx.is_platform_admin() {
        return true;
    }
    match entry_owner_org_id {
        None => false,
        Some(entry_org) => {
            entry_org == ctx.org_id
                && matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        }
    }
}
```

- [ ] **Step 4: Wire into handlers**

In `crates/api/src/management/providers.rs::update_provider`:

```rust
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<Provider>, ApiError> {
    let existing = state.storage.get_provider(&ctx.org_id, &id).await?
        .ok_or(ApiError::NotFound)?;

    if !llm_gateway_org::can_mutate_catalog_entry(&ctx, existing.owner_org_id.as_deref()) {
        return Err(ApiError::Forbidden);
    }

    let updated = state.storage.update_provider(&ctx.org_id, &id, req.into()).await?;
    Ok(Json(updated))
}
```

Apply the same pattern to `delete_provider`, `create_provider`, and the equivalent CUD methods on `models.rs`, `pricing_policies.rs`, `channel_models.rs`.

`create_provider` reads the new entry's `owner_org_id` from the request body. The rule:

```rust
let target_owner_org_id = req.owner_org_id.clone();
let is_platform_create = target_owner_org_id.is_none();
if is_platform_create && !ctx.is_platform_admin() {
    return Err(ApiError::Forbidden);
}
if !is_platform_create && target_owner_org_id.as_deref() != Some(&ctx.org_id) {
    return Err(ApiError::Forbidden);  // can't create entries for other orgs
}
if !llm_gateway_org::can_mutate_catalog_entry(&ctx, target_owner_org_id.as_deref()) {
    return Err(ApiError::Forbidden);
}
```

- [ ] **Step 5: Run tests — expect PASS**

- [ ] **Step 6: Commit**

```bash
git add crates/org/src/access.rs crates/api/src/management/{providers,models,pricing_policies,channel_models}.rs
git commit -m "feat(api,org): can_mutate_catalog_entry gates all catalog CUD operations"
```

---

### Task 8: Frontend — Catalog filter ("Platform" vs "Ours")

**Files:**
- Create: `web/src/components/CatalogFilter.tsx`
- Modify: `web/src/api/providers.ts`, `models.ts`, `pricing_policies.ts`
- Modify: `web/src/hooks/useProviders.ts`, etc.
- Modify: `web/src/pages/Providers.tsx`, `Models.tsx`, `PricingPolicies.tsx`

The filter is a segmented control at the top of catalog listing pages. Default is "All" (current behavior — both platform and org entries visible).

- [ ] **Step 1: Add `CatalogFilter` component**

```tsx
// web/src/components/CatalogFilter.tsx
import { cn } from '../lib/cn'

type Scope = 'all' | 'platform' | 'org'

interface Props {
  value: Scope
  onChange: (s: Scope) => void
  /** Hide the "Ours" option when the user isn't an org admin */
  showOrgOption: boolean
}

export function CatalogFilter({ value, onChange, showOrgOption }: Props) {
  return (
    <div className="inline-flex rounded-md border border-white/10">
      {(['all', 'platform', 'org'] as Scope[])
        .filter((s) => s !== 'org' || showOrgOption)
        .map((s) => (
          <button
            key={s}
            onClick={() => onChange(s)}
            className={cn(
              'px-3 py-1 text-sm capitalize',
              value === s ? 'bg-white/10' : 'hover:bg-white/5',
            )}
          >
            {s === 'all' ? 'All' : s === 'platform' ? 'Platform' : 'Ours'}
          </button>
        ))}
    </div>
  )
}
```

- [ ] **Step 2: Update API client to accept scope**

```typescript
// web/src/api/providers.ts
import { api, orgPrefix } from './client'
import type { Provider } from '../types'

export async function listProviders(scope: 'all' | 'platform' | 'org' = 'all'): Promise<Provider[]> {
  const { data } = await api.get(`${orgPrefix()}/admin/providers`, { params: { scope } })
  return data
}
```

Backend: in `providers::list_providers`, accept a `Query<ScopeParam>` and append to the SQL filter:

```rust
// In storage::list_providers
match scope {
    "platform" => "WHERE owner_org_id IS NULL",
    "org"      => "WHERE owner_org_id = $1",  // $1 = ctx.org_id
    _          => "WHERE owner_org_id IS NULL OR owner_org_id = $1",
}
```

- [ ] **Step 3: Update hook to take scope**

```typescript
// web/src/hooks/useProviders.ts
export function useProviders(scope: 'all' | 'platform' | 'org' = 'all') {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? ''
  return useQuery({
    queryKey: [slug, 'providers', scope],
    queryFn: () => listProviders(scope),
    enabled: !!slug,
  })
}
```

- [ ] **Step 4: Add filter to Providers page**

```tsx
// web/src/pages/Providers.tsx
import { useState } from 'react'
import { CatalogFilter } from '../components/CatalogFilter'
import { useAuthStore } from '../stores/authStore'

export default function Providers() {
  const [scope, setScope] = useState<'all' | 'platform' | 'org'>('all')
  const { currentOrg, user } = useAuthStore()
  const showOrgOption = ['admin', 'owner'].includes(currentOrg?.role ?? '')
    || user?.platform_role === 'platform_admin'
  const { data: providers } = useProviders(scope)

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Providers</h1>
        <CatalogFilter value={scope} onChange={setScope} showOrgOption={showOrgOption} />
      </div>
      {/* ... existing table */}
    </div>
  )
}
```

Repeat for `Models.tsx` and `PricingPolicies.tsx`.

- [ ] **Step 5: Run tests + build**

```bash
source ~/.nvm/nvm.sh && cd web && npm test && npm run build
```

- [ ] **Step 6: Commit**

```bash
git add web/src/
git commit -m "feat(web): catalog filter (Platform / Ours / All) on listing pages"
```

---

### Task 9: Frontend — Impersonation banner

**Files:**
- Create: `web/src/components/ImpersonationBanner.tsx`
- Modify: `web/src/components/Layout.tsx`
- Modify: `web/src/api/me.ts` (or `authStore`) to surface `isImpersonating`

When a platform_admin is operating in an org they don't own (temp member row), show a top-of-page banner so they (and any onlooker) know what context they're in.

- [ ] **Step 1: Determine "is impersonating" client-side**

The login/`/me` response already includes `current_org` and `orgs`. If `currentOrg` is not in `orgs` (or is in `orgs` but with `created_by === 'system'`... but that's a backend-only field), the user is impersonating.

Simplest signal: backend includes an `impersonating: bool` field in the `/me` response, set when the membership was a temp row.

`crates/api/src/management/auth.rs::me`:

```rust
let impersonating = member.created_by.as_deref() == Some("system");
// include in response
```

Update `MeResponse`:

```rust
pub struct MeResponse {
    // ... existing fields
    pub impersonating: bool,
}
```

- [ ] **Step 2: Frontend type + store**

```typescript
// web/src/types/index.ts
export interface MeResponse {
  // ...
  impersonating: boolean
}

// web/src/stores/authStore.ts
interface AuthState {
  // ...
  impersonating: boolean
}
```

Set `impersonating` in the `me`/`login`/`refresh` action handlers.

- [ ] **Step 3: Write the banner**

```tsx
// web/src/components/ImpersonationBanner.tsx
import { useAuthStore } from '../stores/authStore'

export function ImpersonationBanner() {
  const { impersonating, currentOrg, user } = useAuthStore()
  if (!impersonating || !currentOrg) return null

  return (
    <div className="bg-amber-500/10 border-b border-amber-500/30 px-4 py-2 text-sm text-amber-300">
      <strong>Platform admin mode.</strong> You are viewing org <strong>{currentOrg.name}</strong> as a temporary admin.
      Actions you take here are logged with your user ID ({user?.username}) and flagged in audit logs.
    </div>
  )
}
```

- [ ] **Step 4: Render above main content**

`web/src/components/Layout.tsx`:

```tsx
<div className="flex h-screen flex-col">
  <ImpersonationBanner />
  <div className="flex flex-1 overflow-hidden">
    <aside>{/* ... */}</aside>
    <main className="flex-1"><Outlet /></main>
  </div>
</div>
```

- [ ] **Step 5: Run tests + build**

- [ ] **Step 6: Commit**

```bash
git add web/src/ crates/api/src/management/auth.rs
git commit -m "feat(web): impersonation banner when platform_admin acts in non-owned org"
```

---

### Task 10: End-to-end verification

**Files:** (no file changes)

- [ ] **Step 1: Full backend tests**

```bash
cargo test --workspace
```

- [ ] **Step 2: Full frontend tests + build**

```bash
source ~/.nvm/nvm.sh && cd web && npm test && npm run build
```

- [ ] **Step 3: Manual smoke — platform-admin impersonation**

```bash
cargo run &
```

1. Log in as a platform_admin user
2. Create a second org via `POST /api/v1/orgs` (you become owner)
3. Create a third org from a different user's session (or via SQL)
4. As platform_admin, navigate to `/<third-org-slug>/dashboard`
5. Verify the impersonation banner appears at the top
6. Verify keys/channels/usage pages load (you have temp admin access)
7. Make a change (create a key)
8. Check the audit log: the row has `actor_is_platform_admin = true`
9. Wait 1+ hour (or temporarily shorten the janitor threshold) → verify temp row is removed

- [ ] **Step 4: Manual smoke — anti-shadowing**

```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"gpt-4","owner_org_id":"org_a",...}' \
  http://localhost:8080/api/v1/org-a/admin/models
# Expected: 400 with body mentioning "reserved by a platform-level entry"
```

- [ ] **Step 5: Manual smoke — catalog filter**

1. As org admin, navigate to `/<org>/admin/models`
2. Verify the filter shows "All / Platform / Ours"
3. Click "Platform" → only platform-level models visible
4. Click "Ours" → only org-private models visible (none if no org models exist yet)
5. Create an org-private model (use a unique name like `my-finetune-v1`)
6. Click "All" → both visible
7. As platform_admin, verify all three filters work across orgs

- [ ] **Step 6: Manual smoke — impersonation cleanup**

```bash
psql -U llm_gateway -d llm_gateway -c "
  SELECT user_id, org_id, last_seen FROM members WHERE created_by = 'system';
"
```

Verify rows exist after platform_admin activity. Wait (or speed up the janitor) and verify they're cleaned up.

- [ ] **Step 7: Commit any cleanup**

```bash
git status
git log --oneline -15
```

---

## Self-Review Notes

**Spec coverage:**

| Spec deliverable (Phase 2) | Task |
|---|---|
| Platform-admin impersonation via temp member row + janitor | Tasks 1, 2, 3, 4 |
| Audit log records `actor_is_platform_admin` | Task 5 |
| Anti-shadowing: storage trait rejects org-private creates matching platform names | Task 6 |
| `can_mutate_catalog_entry` check before catalog writes | Task 7 |
| UI surfaces "Platform" vs "Ours" filter | Task 8 |
| (Implicit from spec) "Viewing as org X" indicator | Task 9 — implemented as impersonation banner |

**Placeholder scan:** none. The `state.storage.pool` direct access in Task 4's tests assumes `pool` is pub; if encapsulated, swap to a `Storage` trait method or `pub fn pool() -> &PgPool` accessor. Flagged inline.

**Type consistency:** `CatalogError` is new in this plan, lives in `crates/storage/src/catalog.rs`. `OrgContext` is unchanged from Plan 2.1. `Scope` (`'all' | 'platform' | 'org'`) is consistent across backend query params and frontend types.

**Risks worth flagging:**

1. **`last_seen` write on every request** adds ~1ms of latency and one DB write per request. At 100 RPS that's 8.6M extra writes/day. Acceptable for a management API (low traffic). If this gateway ever serves high-RPS proxy traffic through the same middleware, the touch should be conditional on management-vs-proxy routing — but for now, `membership_layer` only runs on management routes, so this is fine.

2. **Janitor interval of 5 minutes is hardcoded.** If you want it configurable, add a `[impersonation] cleanup_interval_secs = 300` section to `config.toml`. Out of scope for Plan 2.3 unless ops asks.

3. **Anti-shadowing is in the storage trait, not the DB.** A direct SQL INSERT bypassing the trait could create a shadowing entry. The migration's partial UNIQUE indexes catch exact platform-vs-platform and org-vs-org duplicates, but org-vs-platform is enforced only at the trait level. Acceptable — direct SQL writes are already a "you broke it, you bought it" zone.

4. **`can_mutate_catalog_entry` returning true for platform_admin on ANY org entry is intentional.** It lets support staff debug customer issues by editing their catalog entries if needed. All such edits are audit-logged with `actor_is_platform_admin = true`. If your compliance team wants a stricter rule (e.g., platform_admin can read but not write), the function is the single chokepoint.

5. **The "is impersonating" signal relies on `member.created_by == 'system'`.** If a real user happens to have a UUID of `"system"` (extremely unlikely given UUID v4 format), they'd be miscategorized as impersonating. Either reserve the literal at the user-creation layer, or use a sentinel that's clearly not a UUID (e.g., `"system-impersonation"`).

6. **Catalog filter's "All" default preserves v2.1.0 behavior** — no breaking change. But once users start creating org-private catalog entries, they may notice entries appearing in lists they didn't expect. The filter makes this transparent; consider surfacing a small badge ("Ours: 2") on catalog pages once org entries exist.
