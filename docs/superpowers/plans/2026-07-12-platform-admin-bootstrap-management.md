# Platform Admin Bootstrap & Management — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three mechanisms to manage `platform_role` on users: a config knob (`auth.first_user_is_admin`) to disable the silent auto-promotion of the first registered user, a CLI subcommand (`cargo run -p llm-gateway -- grant-platform-admin`) for operator bootstrap/recovery, and a UI page (`/admin/platform-users`) for ongoing grant/revoke. Move platform-scoped routes from `/{slug}/admin/*` to top-level `/admin/*` with a dedicated `PlatformLayout` so URLs accurately reflect scope.

**Architecture:** Storage trait gains `set_user_platform_role` (with last-admin self-demote guard) plus two read methods (`list_platform_admins`, `search_user_candidates`). The `register` handler reads a new `auth.first_user_is_admin` config field (default `true`). A new `crates/gateway/src/cli.rs` parses a `clap`-derived subcommand that runs before the server starts. New `crates/api/src/management/admin_users.rs` exposes `GET /api/v1/admin/platform-users` and `PATCH /api/v1/admin/users/:id/platform-role`. Frontend gets a `PlatformLayout` chrome, a `PlatformUsers` page, and a route restructure that moves `/admin/settings` (the just-shipped platform-scoped settings page) to top-level with a client-side redirect from the old `/{slug}/admin/settings`.

**Tech Stack:** Rust (clap 4 for CLI, sqlx for storage, axum for handlers, thiserror for errors), React 18 + TypeScript + React Query + i18next + daisyUI (frontend).

**Branch:** Work on `feature/platform-admin-bootstrap` cut from `develop`. The previous task's branch (`feature/saas-phase8-budget-alerts`) already has the spec committed; cut a fresh branch from `develop` and re-apply the platform sidebar refactor (already in working tree as uncommitted changes) as the starting point for this work.

---

## File Structure

### New files
- `crates/gateway/src/cli.rs` — clap parser + `grant-platform-admin` handler
- `crates/api/src/management/admin_users.rs` — list + patch handlers
- `crates/api/tests/test_admin_users.rs` — integration tests
- `web/src/components/PlatformLayout.tsx` — chrome for `/admin/*`
- `web/src/components/PlatformLayout.test.tsx` — layout tests
- `web/src/pages/PlatformUsers.tsx` — grant/revoke UI
- `web/src/pages/PlatformUsers.test.tsx` — page tests
- `web/src/api/admin.ts` — endpoint wrappers
- `web/src/api/admin.test.ts` — MSW-based endpoint wrapper tests

### Modified files
- `Cargo.toml` — add `clap` to `[workspace.dependencies]`
- `crates/gateway/Cargo.toml` — add `clap` dep
- `crates/gateway/src/main.rs` — parse CLI before starting server
- `crates/storage/src/types.rs` — `AuthConfig.first_user_is_admin` + `SetPlatformRoleError` enum
- `crates/storage/src/lib.rs` — three new trait methods (`set_user_platform_role`, `list_platform_admins`, `search_user_candidates`)
- `crates/storage/src/postgres/mod.rs` — implementations
- `crates/api/src/auth.rs:register` — gate on config flag
- `crates/api/src/management/mod.rs` — register new routes; drop `/{org_slug}/admin/settings` route (it's now under `/admin/settings` global)
- `crates/api/src/error.rs` (or wherever `ApiError` lives) — add `LastPlatformAdmin` variant
- `web/src/App.tsx` — top-level `/admin/*` routes with `PlatformLayout` + `RequirePlatformAdmin`; client-side redirect from `/{slug}/admin/settings`
- `web/src/components/Layout.tsx` — Platform sidebar links point to top-level URLs
- `web/src/i18n/en.json`, `web/src/i18n/zh.json` — new strings

---

## Task 1: Add `set_user_platform_role` storage method + last-admin guard

**Files:**
- Modify: `crates/storage/src/types.rs:1287-1290` (AuthConfig) and a new `SetPlatformRoleError` enum near `PlatformRole` at `crates/storage/src/types.rs:32-50`
- Modify: `crates/storage/src/lib.rs` (add trait method)
- Modify: `crates/storage/src/postgres/mod.rs` (implement)
- Create: `crates/api/tests/test_platform_role_storage.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/api/tests/test_platform_role_storage.rs`:

```rust
mod common;

use llm_gateway_storage::Storage;
use llm_gateway_storage::types::SetPlatformRoleError;
use sqlx::PgPool;

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_grants_to_none(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    // seed a regular user with no platform_role
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-target', 'target', 'x', NULL, $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .unwrap();

    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("u-target", "admin-1", Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin), false)
        .await
        .expect("grant succeeds");

    let user = storage.get_user("u-target").await.unwrap().unwrap();
    assert_eq!(
        user.platform_role,
        Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin)
    );
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_404_for_missing_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let err = storage
        .set_user_platform_role("nonexistent", "admin-1", Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin), false)
        .await
        .unwrap_err();
    assert!(matches!(err, SetPlatformRoleError::UserNotFound));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_blocks_last_admin_self_demote(pool: PgPool) {
    // Only admin-1 exists as platform_admin.
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let err = storage
        .set_user_platform_role("admin-1", "admin-1", None, false)
        .await
        .unwrap_err();
    assert!(matches!(err, SetPlatformRoleError::LastPlatformAdmin));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_allows_last_admin_with_override(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("admin-1", "admin-1", None, true)
        .await
        .expect("override flag bypasses guard");
    let user = storage.get_user("admin-1").await.unwrap().unwrap();
    assert_eq!(user.platform_role, None);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_allows_demote_when_two_admins_exist(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    // Add a second platform_admin.
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-second', 'second', 'x', 'platform_admin', $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .unwrap();
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("u-second", "admin-1", None, false)
        .await
        .expect("two admins → demote succeeds");
    let user = storage.get_user("u-second").await.unwrap().unwrap();
    assert_eq!(user.platform_role, None);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_user_platform_role_idempotent_grant(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    storage
        .set_user_platform_role("admin-1", "admin-1", Some(llm_gateway_storage::types::PlatformRole::PlatformAdmin), false)
        .await
        .expect("re-grant is no-op");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /workspace/llm-gateway && cargo test --test test_platform_role_storage 2>&1 | tail -20`

Expected: FAIL — `set_user_platform_role` doesn't exist on the `Storage` trait.

- [ ] **Step 3: Add the error enum to types.rs**

In `crates/storage/src/types.rs`, immediately after the existing `PlatformRole` enum (around line 50), add:

```rust
use thiserror::Error;

/// Errors from `set_user_platform_role`. Surfaced to API handlers and the
/// CLI subcommand; mapped to HTTP 404 / 409 in the API and to exit code 2
/// in the CLI. Caller-supplied overrides do NOT change the typed error —
/// they just unblock the operation, so the CLI prints the override warning
/// after-the-fact rather than as an error variant.
#[derive(Debug, Error)]
pub enum SetPlatformRoleError {
    #[error("user not found")]
    UserNotFound,
    #[error("cannot demote the last platform admin")]
    LastPlatformAdmin,
}
```

(If `thiserror` isn't already in `crates/storage/Cargo.toml`, add it: `thiserror = { workspace = true }` and add `thiserror = "1"` to `[workspace.dependencies]` in the root `Cargo.toml`.)

- [ ] **Step 4: Add the trait method**

In `crates/storage/src/lib.rs`, after the existing `get_user_by_email` method (around line 278), add:

```rust
// ---- Platform admin management ----

/// Set or clear a user's `platform_role`. If `role = None` and the target is
/// the only platform_admin, returns `LastPlatformAdmin` (unless
/// `allow_last_admin_override` is set). All other writes are unconditional
/// inside a single transaction.
///
/// Idempotent: re-granting an already-platform_admin user is a no-op success.
/// Returns `UserNotFound` when no row matches `target_user_id`.
async fn set_user_platform_role(
    &self,
    target_user_id: &str,
    actor_user_id: &str,
    role: Option<PlatformRole>,
    allow_last_admin_override: bool,
) -> Result<(), SetPlatformRoleError>;
```

- [ ] **Step 5: Implement in `crates/storage/src/postgres/mod.rs`**

Add this method to the `impl Storage for PostgresStorage` block. The implementation runs the count + UPDATE in a single transaction:

```rust
async fn set_user_platform_role(
    &self,
    target_user_id: &str,
    _actor_user_id: &str,
    role: Option<PlatformRole>,
    allow_last_admin_override: bool,
) -> Result<(), SetPlatformRoleError> {
    let mut tx = self.pool.begin().await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Lock the target row so a concurrent grant/demote doesn't race the count.
    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE"
    )
    .bind(target_user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    if exists.is_none() {
        return Err(SetPlatformRoleError::UserNotFound);
    }

    // If demoting, check the count of remaining platform_admins.
    if role.is_none() && !allow_last_admin_override {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE platform_role = 'platform_admin'"
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        if count <= 1 {
            return Err(SetPlatformRoleError::LastPlatformAdmin);
        }
    }

    // Apply. `None` → NULL (column is TEXT NULL).
    let sql_role: Option<&str> = role.as_ref().map(|_| "platform_admin");
    sqlx::query(
        "UPDATE users SET platform_role = $1, updated_at = NOW() WHERE id = $2"
    )
    .bind(sql_role)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    tx.commit().await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(())
}
```

(`_actor_user_id` is unused for now — kept in the signature for future audit columns per spec D6.)

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cd /workspace/llm-gateway && cargo test --test test_platform_role_storage 2>&1 | tail -20`

Expected: 6/6 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/lib.rs crates/storage/src/postgres/mod.rs crates/api/tests/test_platform_role_storage.rs Cargo.toml crates/storage/Cargo.toml
git commit -m "feat(storage): add set_user_platform_role with last-admin guard"
```

---

## Task 2: Add `list_platform_admins` and `search_user_candidates` storage methods

**Files:**
- Modify: `crates/storage/src/lib.rs` (trait)
- Modify: `crates/storage/src/postgres/mod.rs` (impl)
- Modify: `crates/api/tests/test_platform_role_storage.rs` (extend)

- [ ] **Step 1: Add failing tests to `test_platform_role_storage.rs`**

Append to the existing test file:

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_platform_admins_returns_all_admins(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-second', 'second', 'x', 'platform_admin', $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool).await.unwrap();

    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let admins = storage.list_platform_admins().await.unwrap();
    assert_eq!(admins.len(), 2);
    let usernames: Vec<&str> = admins.iter().map(|u| u.username.as_str()).collect();
    assert!(usernames.contains(&"admin"));
    assert!(usernames.contains(&"second"));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn search_user_candidates_excludes_admins_and_matches_query(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    // Three regular users, two with "alice" in username/email.
    for (id, name, email) in [
        ("u-a1", "alice_one", "alice1@x.com"),
        ("u-a2", "bob_two",   "alice2@x.com"),
        ("u-b",  "charlie",   "charlie@x.com"),
    ] {
        sqlx::query(
            r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, email, created_at, updated_at)
               VALUES ($1, $2, 'x', NULL, $3, true, $4, NOW(), NOW())"#,
        )
        .bind(id).bind(name).bind(common::TEST_ORG).bind(email)
        .execute(&pool).await.unwrap();
    }

    let storage = llm_gateway_storage::postgres::PostgresStorage::from_pool(pool);
    let hits = storage.search_user_candidates("alice").await.unwrap();
    assert_eq!(hits.len(), 2);
    let names: Vec<&str> = hits.iter().map(|u| u.username.as_str()).collect();
    assert!(names.contains(&"alice_one"));
    assert!(names.contains(&"bob_two"));
    assert!(!names.contains(&"charlie"));
    assert!(!names.contains(&"admin")); // platform_admin excluded
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /workspace/llm-gateway && cargo test --test test_platform_role_storage list_platform_admins 2>&1 | tail -10`

Expected: FAIL — `list_platform_admins` doesn't exist on the trait.

- [ ] **Step 3: Add trait methods**

In `crates/storage/src/lib.rs`, immediately after the `set_user_platform_role` declaration:

```rust
/// Return every user that currently holds `platform_role = 'platform_admin'`.
/// Used by the `GET /api/v1/admin/platform-users` handler. No pagination —
/// the platform_admin set is expected to stay small (typically <10).
async fn list_platform_admins(&self) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>>;

/// Substring-search users whose `platform_role IS NULL` by username or email
/// (case-insensitive). Returns up to 20 results. Used by the
/// search-to-add affordance on the PlatformUsers page; the response must
/// exclude existing platform_admins.
async fn search_user_candidates(
    &self,
    query: &str,
) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 4: Implement in `postgres/mod.rs`**

Add inside the same `impl Storage for PostgresStorage` block:

```rust
async fn list_platform_admins(&self) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>> {
    let rows: Vec<User> = sqlx::query_as::<_, User>(
        "SELECT id, username, password, platform_role, current_org_id, enabled, \
                refresh_token, email, email_verified_at, requires_email_verification, \
                password_changed_at, created_at, updated_at \
         FROM users WHERE platform_role = 'platform_admin' \
         ORDER BY username ASC"
    )
    .fetch_all(&self.pool)
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(rows)
}

async fn search_user_candidates(
    &self,
    query: &str,
) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>> {
    let pattern = format!("%{}%", query.to_lowercase());
    let rows: Vec<User> = sqlx::query_as::<_, User>(
        "SELECT id, username, password, platform_role, current_org_id, enabled, \
                refresh_token, email, email_verified_at, requires_email_verification, \
                password_changed_at, created_at, updated_at \
         FROM users \
         WHERE platform_role IS NULL \
           AND (LOWER(username) LIKE $1 OR LOWER(COALESCE(email, '')) LIKE $1) \
         ORDER BY username ASC \
         LIMIT 20"
    )
    .bind(&pattern)
    .fetch_all(&self.pool)
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(rows)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /workspace/llm-gateway && cargo test --test test_platform_role_storage 2>&1 | tail -15`

Expected: 8/8 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/postgres/mod.rs crates/api/tests/test_platform_role_storage.rs
git commit -m "feat(storage): add list_platform_admins and search_user_candidates"
```

---

## Task 3: Add `auth.first_user_is_admin` config field + register gate

**Files:**
- Modify: `crates/storage/src/types.rs:1287-1290` (AuthConfig)
- Modify: `crates/gateway/src/main.rs:236-239` (default config template)
- Modify: `crates/api/src/auth.rs:register` (gate on flag)
- Create: `crates/api/tests/test_first_user_promotion.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/api/tests/test_first_user_promotion.rs`:

```rust
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use llm_gateway_api::{management, AppState};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

async fn register_first_user(state: Arc<AppState>) -> reqwest::Response {
    let app = management::management_router(state.clone()).with_state(state.clone());
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "username": "first",
                "password": "supersecret123",
                "email": "first@test.local"
            }).to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn first_user_is_admin_true_promotes(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let resp = register_first_user(state).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT platform_role FROM users WHERE username = 'first'"
    )
    .fetch_one(&pool).await.unwrap();
    assert_eq!(user.0, Some("platform_admin".to_string()));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn first_user_is_admin_false_skips_promotion(pool: PgPool) {
    // Step 7 below replaces this stub with the real test once
    // `make_state_with_auth` exists.
    let _ = pool;
}
```

(Step 4 below explains why the second test is wired differently. The test file as-written is the source of truth — Step 4 fills in the AppState plumbing.)

- [ ] **Step 2: Run the test, verify first one fails (the "true" path is fine; we want to see that the config field can be flipped)**

Run: `cd /workspace/llm-gateway && cargo test --test test_first_user_promotion 2>&1 | tail -10`

Expected: Compile error — `AppState` doesn't expose `first_user_is_admin`. That's the gate for Step 4.

- [ ] **Step 3: Add the config field**

In `crates/storage/src/types.rs:1287-1290`:

```rust
pub struct AuthConfig {
    pub jwt_secret: String,
    pub allow_registration: Option<bool>,
    /// When true (default), the first user to register against an empty DB
    /// is automatically promoted to `platform_admin`. Operators who want to
    /// bootstrap via the CLI subcommand instead should set this to false.
    /// Self-hosted deployments typically leave this on; SaaS deployments
    /// typically turn it off.
    #[serde(default = "default_first_user_is_admin")]
    pub first_user_is_admin: bool,
}

fn default_first_user_is_admin() -> bool { true }
```

- [ ] **Step 4: Plumb the config into AppState**

`AppState` (in `crates/api/src/lib.rs`) currently does not carry `AuthConfig`. Add it:

```rust
pub struct AppState {
    // ... existing fields ...
    pub auth_config: Arc<AuthConfig>,
}
```

In `crates/gateway/src/main.rs`, when building `AppState`, add `auth_config: Arc::new(config.auth.clone())`.

In `crates/api/tests/common/mod.rs`, in `make_state`, add:

```rust
let auth_config = Arc::new(llm_gateway_storage::AuthConfig {
    jwt_secret: TEST_JWT_SECRET.to_string(),
    allow_registration: Some(true),
    first_user_is_admin: true,
});
// then `auth_config,` in the AppState literal
```

To test the `false` case, extend `make_state` to accept an override:

```rust
pub fn make_state_with_auth(pool: PgPool, first_user_is_admin: bool) -> Arc<AppState> {
    // Same as make_state but with `first_user_is_admin: first_user_is_admin`
}
```

- [ ] **Step 5: Gate `register` on the flag**

In `crates/api/src/auth.rs`, around line 422, replace:

```rust
let platform_role = if is_first_user {
    Some(PlatformRole::PlatformAdmin)
} else {
    None
};
```

with:

```rust
let platform_role = if is_first_user && state.auth_config.first_user_is_admin {
    Some(PlatformRole::PlatformAdmin)
} else {
    None
};
```

- [ ] **Step 6: Update the default config template in `crates/gateway/src/main.rs:236-239`**

After:

```toml
[auth]
jwt_secret = "change-me-jwt-secret!"
allow_registration = true
```

Add the commented new line:

```toml
# first_user_is_admin = true  # uncomment + set false to disable silent first-user promotion
```

- [ ] **Step 7: Write the second test with the new helper**

Update `test_first_user_promotion.rs`:

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn first_user_is_admin_false_skips_promotion(pool: PgPool) {
    let state = common::make_state_with_auth(pool.clone(), false);
    let resp = register_first_user(state).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT platform_role FROM users WHERE username = 'first'"
    )
    .fetch_one(&pool).await.unwrap();
    assert_eq!(user.0, None);
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd /workspace/llm-gateway && cargo test --test test_first_user_promotion 2>&1 | tail -10`

Expected: 2/2 tests PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/storage/src/types.rs crates/api/src/lib.rs crates/api/src/auth.rs crates/gateway/src/main.rs crates/api/tests/common/mod.rs crates/api/tests/test_first_user_promotion.rs
git commit -m "feat(auth): add auth.first_user_is_admin config flag"
```

---

## Task 4: API handler — list_platform_users + patch_platform_role

**Files:**
- Modify: `crates/api/src/error.rs` (add `LastPlatformAdmin` variant mapping to 409)
- Create: `crates/api/src/management/admin_users.rs`
- Modify: `crates/api/src/management/mod.rs:117-120` (register global routes)

- [ ] **Step 1: Add ApiError variant**

In `crates/api/src/error.rs`, add (alongside the other variants):

```rust
#[error("cannot demote the last platform admin")]
LastPlatformAdmin,
```

Map it to `StatusCode::CONFLICT` in the `IntoResponse` impl, with body `{"error":"last_platform_admin","message":"..."}`.

- [ ] **Step 2: Write the failing tests**

Create `crates/api/tests/test_admin_users.rs`:

```rust
mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::{management, AppState};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn bearer(token: &str) -> String { format!("Bearer {}", token) }

fn build(state: Arc<AppState>) -> axum::Router {
    management::management_router(state.clone()).with_state(state)
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_platform_users_requires_platform_admin(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let user = common::make_user_token("non-admin-user");

    let resp = app.oneshot(
        Request::builder()
            .method("GET")
            .uri("/api/v1/admin/platform-users")
            .header("authorization", bearer(&user.token))
            .body(Body::empty()).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_platform_users_returns_admins(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("GET")
            .uri("/api/v1/admin/platform-users")
            .header("authorization", bearer(&admin.token))
            .body(Body::empty()).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let admins = body["admins"].as_array().expect("admins array");
    assert_eq!(admins.len(), 1);
    assert_eq!(admins[0]["username"], "admin");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_platform_role_grants_to_non_admin(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-target', 'target', 'x', NULL, $1, true, NOW(), NOW())"#,
    ).bind(common::TEST_ORG).execute(&pool).await.unwrap();

    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri("/api/v1/admin/users/u-target/platform-role")
            .header("authorization", bearer(&admin.token))
            .header("content-type", "application/json")
            .body(Body::from(json!({"platform_role": "platform_admin"}).to_string())).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_platform_role_returns_409_on_last_admin_self_demote(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri("/api/v1/admin/users/admin-1/platform-role")
            .header("authorization", bearer(&admin.token))
            .header("content-type", "application/json")
            .body(Body::from(json!({"platform_role": null}).to_string())).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["error"], "last_platform_admin");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn patch_platform_role_returns_404_for_missing_user(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let app = build(common::make_state(pool));
    let admin = common::make_admin_token();

    let resp = app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri("/api/v1/admin/users/nonexistent/platform-role")
            .header("authorization", bearer(&admin.token))
            .header("content-type", "application/json")
            .body(Body::from(json!({"platform_role": "platform_admin"}).to_string())).unwrap(),
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd /workspace/llm-gateway && cargo test --test test_admin_users 2>&1 | tail -10`

Expected: 404 — the `/api/v1/admin/platform-users` route doesn't exist yet.

- [ ] **Step 4: Implement the handler**

Create `crates/api/src/management/admin_users.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::require_platform_admin;
use crate::AppState;
use llm_gateway_storage::types::{PlatformRole, SetPlatformRoleError};

#[derive(Debug, Deserialize)]
pub struct PatchPlatformRoleBody {
    /// `"platform_admin"` to grant, `null` to revoke.
    pub platform_role: Option<String>,
}

pub async fn list_platform_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let _claims = require_platform_admin(&headers, &state.jwt_secret)?;
    let admins = state
        .storage
        .list_platform_admins()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    // Strip password + refresh_token before responding.
    let safe: Vec<_> = admins
        .into_iter()
        .map(|u| serde_json::json!({
            "id": u.id,
            "username": u.username,
            "email": u.email,
            "platform_role": u.platform_role,
        }))
        .collect();
    Ok(Json(serde_json::json!({ "admins": safe })))
}

pub async fn patch_platform_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PatchPlatformRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    let _claims = require_platform_admin(&headers, &state.jwt_secret)?;
    let role: Option<PlatformRole> = match body.platform_role.as_deref() {
        Some("platform_admin") => Some(PlatformRole::PlatformAdmin),
        None => None,
        Some(other) => {
            return Err(ApiError::BadRequest(format!(
                "unknown platform_role value: {other}"
            )));
        }
    };
    // Note: allow_last_admin_override is hard-coded false here. The CLI gets
    // the override; the API path does not, by design.
    state
        .storage
        .set_user_platform_role(&user_id, &_claims.sub, role, false)
        .await
        .map_err(|e| match e {
            SetPlatformRoleError::UserNotFound => ApiError::NotFound,
            SetPlatformRoleError::LastPlatformAdmin => ApiError::LastPlatformAdmin,
        })?;
    Ok(Json(serde_json::json!({"id": user_id, "platform_role": body.platform_role})))
}
```

- [ ] **Step 5: Register the module + routes**

In `crates/api/src/management/mod.rs`, add `pub mod admin_users;` at the top with the others, then in `management_router` add the global routes (alongside `/api/v1/admin/nats/status` around line 120):

```rust
.route("/api/v1/admin/platform-users", get(admin_users::list_platform_users))
.route(
    "/api/v1/admin/users/{id}/platform-role",
    axum::routing::patch(admin_users::patch_platform_role),
)
```

- [ ] **Step 6: Add `NotFound` variant if missing**

If `ApiError::NotFound` doesn't exist, add it with `StatusCode::NOT_FOUND` mapping.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cd /workspace/llm-gateway && cargo test --test test_admin_users 2>&1 | tail -15`

Expected: 5/5 tests PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/api/src/error.rs crates/api/src/management/admin_users.rs crates/api/src/management/mod.rs crates/api/tests/test_admin_users.rs
git commit -m "feat(api): add /admin/platform-users and PATCH /admin/users/:id/platform-role"
```

---

## Task 5: Move `/admin/settings` to top-level global route + drop the old org-scoped route

**Files:**
- Modify: `crates/api/src/management/mod.rs:154` (drop `/admin/settings` from `org_scoped_routes()`)
- Modify: `crates/api/src/management/mod.rs:120` (add `/api/v1/admin/settings` as a global route — it's already there; just verify the path)

- [ ] **Step 1: Inspect current routing**

Open `crates/api/src/management/mod.rs`:
- Line 300 in `org_scoped_routes()` has `"/admin/settings"`. The handler is `settings::get_settings` / `settings::update_settings`.
- This route is mounted under `/api/v1/{org_slug}/admin/settings`. That means the request path is `/api/v1/{anything}/admin/settings`.

- [ ] **Step 2: Move the route to global**

Move the `.route("/admin/settings", ...)` declaration from `org_scoped_routes()` (line 300) to the global routes section of `management_router()` (around line 120):

```rust
.route("/api/v1/admin/settings", get(settings::get_settings).patch(settings::update_settings))
```

Delete the line in `org_scoped_routes()`.

- [ ] **Step 3: Verify by running all management tests**

Run: `cd /workspace/llm-gateway && cargo test --test test_settings 2>&1 | tail -20`

Expected: 4/4 tests PASS — note that the test file calls `/api/v1/default/admin/settings` which already works because `default` is treated as `org_slug` today. After the move, that path stops matching and we need to update the tests.

- [ ] **Step 4: Update `crates/api/tests/test_settings.rs`**

Replace `"/api/v1/default/admin/settings"` with `"/api/v1/admin/settings"` in all 4 tests.

- [ ] **Step 5: Re-run tests**

Run: `cd /workspace/llm-gateway && cargo test --test test_settings 2>&1 | tail -10`

Expected: 4/4 tests PASS on the new path.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/mod.rs crates/api/tests/test_settings.rs
git commit -m "refactor(api): move /admin/settings from org-scoped to global route"
```

---

## Task 6: CLI subcommand `grant-platform-admin`

**Files:**
- Modify: `Cargo.toml` (workspace deps: add clap)
- Modify: `crates/gateway/Cargo.toml` (add clap)
- Create: `crates/gateway/src/cli.rs`
- Modify: `crates/gateway/src/main.rs` (parse CLI, dispatch subcommand)

- [ ] **Step 1: Add clap dep**

In `Cargo.toml` `[workspace.dependencies]` add:

```toml
clap = { version = "4", features = ["derive"] }
```

In `crates/gateway/Cargo.toml` `[dependencies]` add:

```toml
clap = { workspace = true }
```

- [ ] **Step 2: Write a smoke test for the CLI**

Create `crates/gateway/tests/cli_smoke.rs`:

```rust
// Spawns the `llm-gateway` binary as a subprocess with a custom DATABASE_URL
// and a temp config file. Asserts exit code + stdout/stderr.

use std::process::Command;

#[test]
fn help_flag_prints_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_llm-gateway"))
        .arg("--help")
        .output()
        .expect("spawn gateway binary");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("grant-platform-admin"), "help should list the subcommand");
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cd /workspace/llm-gateway && cargo test --test cli_smoke 2>&1 | tail -10`

Expected: FAIL — `--help` doesn't print the subcommand (no CLI parser wired yet).

- [ ] **Step 4: Implement the CLI parser**

Create `crates/gateway/src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "llm-gateway", about = "LLM Gateway server and admin CLI")]
pub struct Cli {
    /// Path to config.toml. Defaults to ./config.toml. Used by the CLI
    /// subcommand when present; ignored when running the server (the server
    /// reads from the working directory by convention).
    #[arg(long, global = true, default_value = "config.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Grant or revoke platform_admin for a user. Operator escape hatch for
    /// bootstrap when the first-user auto-promotion is disabled.
    GrantPlatformAdmin {
        /// Username to grant/revoke. Must already exist in the users table.
        #[arg(long)]
        username: String,
        /// Revoke instead of grant. Sets platform_role = NULL.
        #[arg(long, default_value_t = false)]
        revoke: bool,
        /// Override the last-admin guard when revoking. Required to demote
        /// the only platform_admin. Prints a warning when used.
        #[arg(long, default_value_t = false)]
        allow_last_admin: bool,
    },
}
```

- [ ] **Step 5: Wire CLI into main.rs**

In `crates/gateway/src/main.rs`, replace the `#[tokio::main]` body to parse CLI first:

```rust
use clap::Parser;
use llm_gateway_gateway::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        return run_cli_command(cmd).await;
    }

    // ... existing bootstrap + server startup ...
}

async fn run_cli_command(cmd: Commands) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use llm_gateway_storage::postgres::PostgresStorage;
    use llm_gateway_storage::Storage;
    use llm_gateway_storage::types::PlatformRole;

    // Resolve config path from the CLI's --config flag (defaults to
    // "config.toml"). The server startup path also reads this flag, so the
    // two paths share the same load-config step.
    let cli = Cli::parse();
    let config_path = &cli.config;

    match cmd {
        Commands::GrantPlatformAdmin { username, revoke, allow_last_admin } => {
            let config_str = std::fs::read_to_string(config_path)?;
            let config_str = shellexpand::env(&config_str)?.to_string();
            let config: AppConfig = toml::from_str(&config_str)?;
            if config.database.driver.as_str() != "postgres" {
                eprintln!("error: only 'postgres' driver is supported");
                std::process::exit(1);
            }
            let url = config.database.url.as_deref()
                .ok_or("database.url is required")?;
            let db = PostgresStorage::new(url).await?;

            let user = db.get_user_by_username(&username).await?
                .ok_or_else(|| {
                    eprintln!("error: user '{username}' not found");
                    "user not found"
                })?;
            let actor = &user.id;

            let role = if revoke { None } else { Some(PlatformRole::PlatformAdmin) };
            if revoke && allow_last_admin {
                eprintln!("warning: --allow-last-admin set; proceeding with demotion");
            }

            match db.set_user_platform_role(&user.id, actor, role.clone(), allow_last_admin).await {
                Ok(()) => {
                    if revoke {
                        println!("user '{username}' is no longer platform_admin");
                    } else if user.platform_role == Some(PlatformRole::PlatformAdmin) {
                        println!("user '{username}' is already platform_admin (no change)");
                    } else {
                        println!("user '{username}' is now platform_admin");
                    }
                    Ok(())
                }
                Err(llm_gateway_storage::types::SetPlatformRoleError::LastPlatformAdmin) => {
                    eprintln!("error: cannot demote last platform admin (pass --allow-last-admin to override)");
                    std::process::exit(2);
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        }
    }
}
```

- [ ] **Step 6: Export the cli module**

In `crates/gateway/src/lib.rs` (create it if missing):

```rust
pub mod cli;
```

- [ ] **Step 7: Run the smoke test**

Run: `cd /workspace/llm-gateway && cargo test --test cli_smoke 2>&1 | tail -10`

Expected: 1/1 PASS.

- [ ] **Step 8: Add the integration test that hits a real DB**

Append to `crates/gateway/tests/cli_smoke.rs`:

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn grant_platform_admin_promotes_user(pool: sqlx::PgPool) {
    // Seed a regular user.
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('u-cli-test', 'cli_test', 'x', NULL, NULL, true, NOW(), NOW())"#,
    ).execute(&pool).await.unwrap();

    // Write a temp config pointing at this test DB.
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/llm_gateway_test".into());
    let tmp = tempdir();
    std::fs::write(tmp.join("config.toml"), format!(
        "[server]\nhost=\"0.0.0.0\"\nport=0\nencryption_key=\"x\"\n\n[auth]\njwt_secret=\"x\"\nallow_registration=true\n\n[database]\ndriver=\"postgres\"\nurl=\"{url}\"\n"
    )).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_llm-gateway"))
        .arg("grant-platform-admin")
        .arg("--username").arg("cli_test")
        .env("DATABASE_URL_OVERRIDE_FOR_TEST", &url)  // see note below
        .current_dir(&tmp)
        .output()
        .expect("spawn");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let row: (Option<String>,) = sqlx::query_as(
        "SELECT platform_role FROM users WHERE username = 'cli_test'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, Some("platform_admin".into()));
}

fn tempdir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("cli-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&p).unwrap();
    p
}
```

(Note: the gateway binary currently reads `config.toml` directly. The cleanest integration-test pattern requires the CLI to accept a `--config <path>` flag. Add that flag to `Commands::GrantPlatformAdmin` if not already present; fall back to `config.toml` in CWD when absent. The test writes a temp config to a tempdir and runs the binary with `--config <tmp>/config.toml`.)

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml crates/gateway/Cargo.toml crates/gateway/src/cli.rs crates/gateway/src/lib.rs crates/gateway/src/main.rs crates/gateway/tests/cli_smoke.rs
git commit -m "feat(cli): add grant-platform-admin subcommand"
```

---

## Task 7: Frontend API wrappers (`api/admin.ts`)

**Files:**
- Create: `web/src/api/admin.ts`
- Create: `web/src/api/admin.test.ts`

- [ ] **Step 1: Look at the existing API client conventions**

Read `web/src/api/client.ts` to confirm:
- Base URL prefix is `/api/v1`
- Bearer token attached automatically
- Response shape: `{ data: T }` (axios unwrap)

(Use `apiClient.get`, `apiClient.patch` per existing endpoints.)

- [ ] **Step 2: Write failing tests**

Create `web/src/api/admin.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { listPlatformUsers, searchCandidates, setPlatformRole } from './admin';

describe('admin api wrappers', () => {
  it('listPlatformUsers calls GET /admin/platform-users', async () => {
    let called = false;
    server.use(
      http.get('/api/v1/admin/platform-users', () => {
        called = true;
        return HttpResponse.json({ admins: [], candidates: [] });
      }),
    );
    const r = await listPlatformUsers();
    expect(called).toBe(true);
    expect(r.admins).toEqual([]);
  });

  it('searchCandidates encodes the query', async () => {
    let url = '';
    server.use(
      http.get('/api/v1/admin/platform-users', (req) => {
        url = req.url.toString();
        return HttpResponse.json({ admins: [], candidates: [] });
      }),
    );
    await searchCandidates('alice');
    expect(url).toContain('q=alice');
  });

  it('setPlatformRole PATCHes with body', async () => {
    let body: any;
    server.use(
      http.patch('/api/v1/admin/users/u-1/platform-role', async ({ request }) => {
        body = await request.json();
        return HttpResponse.json({ id: 'u-1', platform_role: 'platform_admin' });
      }),
    );
    await setPlatformRole('u-1', 'platform_admin');
    expect(body).toEqual({ platform_role: 'platform_admin' });
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/api/admin.test.ts 2>&1 | tail -10`

Expected: FAIL — `./admin` module not found.

- [ ] **Step 4: Implement the wrappers**

Create `web/src/api/admin.ts`:

```ts
import { apiClient } from './client';

export interface PlatformUserBrief {
  id: string;
  username: string;
  email: string | null;
  platform_role: 'platform_admin' | null;
}

export interface PlatformUsersResponse {
  admins: PlatformUserBrief[];
  candidates?: PlatformUserBrief[];
}

export async function listPlatformUsers(query?: string): Promise<PlatformUsersResponse> {
  const params = query ? { params: { q: query } } : undefined;
  const r = await apiClient.get<PlatformUsersResponse>('/admin/platform-users', params);
  return r.data;
}

export async function searchCandidates(query: string): Promise<PlatformUserBrief[]> {
  const r = await apiClient.get<PlatformUsersResponse>('/admin/platform-users', {
    params: { q: query },
  });
  return r.data.candidates ?? [];
}

export async function setPlatformRole(
  userId: string,
  platformRole: 'platform_admin' | null,
): Promise<PlatformUserBrief> {
  const r = await apiClient.patch<PlatformUserBrief>(
    `/admin/users/${userId}/platform-role`,
    { platform_role: platformRole },
  );
  return r.data;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/api/admin.test.ts 2>&1 | tail -10`

Expected: 3/3 PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/api/admin.ts web/src/api/admin.test.ts
git commit -m "feat(web): add admin api wrappers for platform-users endpoints"
```

---

## Task 8: Frontend `PlatformLayout` chrome

**Files:**
- Create: `web/src/components/PlatformLayout.tsx`
- Create: `web/src/components/PlatformLayout.test.tsx`

- [ ] **Step 1: Write failing tests**

Create `web/src/components/PlatformLayout.test.tsx`:

```tsx
import { describe, it, expect, beforeEach } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen } from '@testing-library/react';
import { useAuthStore } from '../stores/authStore';
import type { User, OrgSummary } from '../types';
import PlatformLayout from './PlatformLayout';

const platformAdmin: User = {
  id: 'u-pa', username: 'pa', platform_role: 'platform_admin',
  email: 'pa@x.com', email_verified_at: '2026-07-12T00:00:00Z',
};
const org: OrgSummary = {
  id: 'org-1', slug: 'test-org', name: 'Test Org', role: 'admin', group_id: null,
};

describe('PlatformLayout', () => {
  beforeEach(() => {
    useAuthStore.setState({ user: platformAdmin, currentOrg: org });
  });

  it('renders Platform sidebar with Settings and Platform Users links', () => {
    renderWithProviders(<PlatformLayout />, { route: '/admin/settings' });
    expect(screen.getByText('Platform')).toBeInTheDocument();
    // The links are rendered as buttons inside the sidebar.
    const settingsLinks = screen.getAllByText('Settings');
    expect(settingsLinks.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Platform Users')).toBeInTheDocument();
  });

  it('shows back-to-org link when currentOrg is set', () => {
    renderWithProviders(<PlatformLayout />, { route: '/admin/settings' });
    expect(screen.getByText(/back to/i)).toBeInTheDocument();
    expect(screen.getByText('Test Org')).toBeInTheDocument();
  });

  it('hides back-to-org link when currentOrg is null', () => {
    useAuthStore.setState({ currentOrg: null });
    renderWithProviders(<PlatformLayout />, { route: '/admin/settings' });
    expect(screen.queryByText(/back to/i)).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/components/PlatformLayout.test.tsx 2>&1 | tail -10`

Expected: FAIL — `./PlatformLayout` not found.

- [ ] **Step 3: Implement PlatformLayout**

Create `web/src/components/PlatformLayout.tsx`:

```tsx
import { Outlet, useNavigate } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import { isPlatformAdmin } from '../lib/auth';
import { useTranslation } from 'react-i18next';
import { Settings, Users, PanelLeftClose } from 'lucide-react';

export default function PlatformLayout() {
  const navigate = useNavigate();
  const user = useAuthStore((s) => s.user);
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const { t } = useTranslation();

  // The route guard at App.tsx already enforces this, but render-time
  // double-check prevents flicker if a stale render slips through.
  if (!isPlatformAdmin(user)) return null;

  const navItem = (path: string, label: string, Icon: typeof Settings, active: boolean) => (
    <button
      key={path}
      className={`group/nav flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-base font-medium transition-all duration-150 whitespace-nowrap overflow-hidden select-none relative ${
        active
          ? 'bg-primary/10 text-primary'
          : 'text-base-content/50 hover:bg-base-200 hover:text-base-content/80'
      }`}
      onClick={() => navigate(path)}
    >
      {active && <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-4 rounded-r-full bg-primary" />}
      <Icon className="h-[17px] w-[17px] shrink-0" strokeWidth={active ? 2 : 1.5} />
      <span>{label}</span>
    </button>
  );

  const items = [
    { path: '/admin/settings', label: t('sidebar.settings'), Icon: Settings },
    { path: '/admin/platform-users', label: t('sidebar.platformUsers'), Icon: Users },
  ];

  return (
    <div className="flex min-h-screen bg-base-200">
      {/* Sidebar */}
      <aside className="w-[232px] fixed left-0 top-0 bottom-0 z-[100] flex flex-col bg-base-100 border-r border-base-300/60">
        <div className="flex h-14 items-center gap-3 border-b border-base-300/60 px-4">
          <div className="h-8 w-8 shrink-0 rounded-lg bg-primary flex items-center justify-center font-semibold text-md text-primary-content">TV</div>
          <span className="font-semibold text-lg">TokenVis</span>
        </div>
        <nav className="flex-1 overflow-y-auto px-3 py-4 flex flex-col gap-0.5">
          <div className="text-xs font-semibold uppercase tracking-[0.12em] text-base-content/30 px-3 pt-1 pb-2">
            {t('sidebar.platform')}
          </div>
          {items.map(({ path, label, Icon }) => navItem(path, label, Icon, location.pathname.startsWith(path)))}
        </nav>
      </aside>

      {/* Main */}
      <div className="flex min-h-screen flex-col md:ml-[232px]">
        <header className="fixed top-0 z-40 shrink-0 bg-base-100/80 backdrop-blur-md border-b border-base-300/60 h-12 left-0 md:left-[232px] right-0">
          <div className="flex h-12 items-center px-4 md:px-6 gap-3 w-full">
            {currentOrg && (
              <button
                onClick={() => navigate(`/${currentOrg.slug}/dashboard`)}
                className="text-sm text-base-content/60 hover:text-base-content transition-colors flex items-center gap-1.5"
              >
                <PanelLeftClose className="h-3.5 w-3.5" />
                {t('platformLayout.backTo')} {currentOrg.name}
              </button>
            )}
            <div className="ml-auto text-xs text-base-content/40">{user?.username}</div>
          </div>
        </header>
        <main className="flex-1 p-4 md:p-6 overflow-y-auto pt-16 pb-8">
          <div className="animate-fade-in-up" key={location.pathname}>
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Add i18n keys**

In `web/src/i18n/en.json` (sidebar section):

```json
"platformUsers": "Platform Users",
```

In `web/src/i18n/zh.json`:

```json
"platformUsers": "平台管理员",
```

In a new top-level `platformLayout` section (both files):

- en: `"backTo": "Back to"`
- zh: `"backTo": "返回"`

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/components/PlatformLayout.test.tsx 2>&1 | tail -10`

Expected: 3/3 PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/PlatformLayout.tsx web/src/components/PlatformLayout.test.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): add PlatformLayout chrome for /admin/* routes"
```

---

## Task 9: Frontend `PlatformUsers` page

**Files:**
- Create: `web/src/pages/PlatformUsers.tsx`
- Create: `web/src/pages/PlatformUsers.test.tsx`

- [ ] **Step 1: Write failing tests**

Create `web/src/pages/PlatformUsers.test.tsx`:

```tsx
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { setToken } from '../api/client';
import { useAuthStore } from '../stores/authStore';
import PlatformUsers from './PlatformUsers';

const adminUser = {
  id: 'u-pa', username: 'pa', platform_role: 'platform_admin' as const,
  email: 'pa@x.com', email_verified_at: '2026-07-12T00:00:00Z',
};

beforeEach(() => {
  setToken('test-token');
  useAuthStore.setState({ user: adminUser });
});

describe('PlatformUsers page', () => {
  it('renders current platform admins from the API', async () => {
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
            { id: 'u-2', username: 'second', email: 's@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => {
      expect(screen.getByText('pa')).toBeInTheDocument();
      expect(screen.getByText('second')).toBeInTheDocument();
    });
  });

  it('hides the revoke button when only one admin (self)', async () => {
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => expect(screen.getByText('pa')).toBeInTheDocument());
    expect(screen.queryByRole('button', { name: /revoke/i })).not.toBeInTheDocument();
  });

  it('shows revoke button when multiple admins exist', async () => {
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
            { id: 'u-2', username: 'second', email: 's@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => expect(screen.getByText('pa')).toBeInTheDocument());
    const revokeButtons = screen.getAllByRole('button', { name: /revoke/i });
    expect(revokeButtons.length).toBeGreaterThanOrEqual(1);
  });

  it('PATCHes platform_role when revoke is clicked', async () => {
    let patched: { url: string; body: any } | null = null;
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
            { id: 'u-2', username: 'second', email: 's@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
      http.patch('/api/v1/admin/users/u-2/platform-role', async ({ request }) => {
        patched = { url: request.url, body: await request.json() };
        return HttpResponse.json({ id: 'u-2', platform_role: null });
      }),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => expect(screen.getByText('second')).toBeInTheDocument());
    const user = userEvent.setup();
    await user.click(screen.getAllByRole('button', { name: /revoke/i })[0]);
    await waitFor(() => {
      expect(patched).not.toBeNull();
      expect(patched!.body).toEqual({ platform_role: null });
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/pages/PlatformUsers.test.tsx 2>&1 | tail -10`

Expected: FAIL — `./PlatformUsers` not found.

- [ ] **Step 3: Implement the page**

Create `web/src/pages/PlatformUsers.tsx`:

```tsx
import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Search } from 'lucide-react';
import {
  listPlatformUsers,
  searchCandidates,
  setPlatformRole,
  type PlatformUserBrief,
} from '../api/admin';
import { useAuthStore } from '../stores/authStore';

export default function PlatformUsers() {
  const { t } = useTranslation();
  const me = useAuthStore((s) => s.user);
  const qc = useQueryClient();
  const [query, setQuery] = useState('');

  const adminsQuery = useQuery({
    queryKey: ['platform-users'],
    queryFn: () => listPlatformUsers(),
  });

  const candidatesQuery = useQuery({
    queryKey: ['platform-users-candidates', query],
    queryFn: () => searchCandidates(query),
    enabled: query.length >= 1,
  });

  const grantMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: 'platform_admin' | null }) =>
      setPlatformRole(userId, role),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['platform-users'] }),
  });

  const admins = adminsQuery.data?.admins ?? [];
  const candidates = candidatesQuery.data ?? [];
  const isLastAdmin = admins.length <= 1;

  return (
    <div className="max-w-4xl">
      <h1 className="text-2xl font-semibold mb-2">{t('platformUsers.title')}</h1>
      <p className="text-sm text-base-content/50 mb-6">{t('platformUsers.subtitle')}</p>

      {/* Current admins */}
      <section className="mb-10">
        <h2 className="text-sm font-semibold uppercase tracking-[0.12em] text-base-content/40 mb-3">
          {t('platformUsers.currentAdmins')}
        </h2>
        <div className="rounded-lg border border-base-300/60 bg-base-100 overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-base-200/50 text-base-content/50 text-xs uppercase">
              <tr>
                <th className="text-left px-4 py-2.5">{t('common.username')}</th>
                <th className="text-left px-4 py-2.5">{t('common.email')}</th>
                <th className="text-right px-4 py-2.5">{t('common.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {admins.map((u) => (
                <tr key={u.id} className="border-t border-base-300/40">
                  <td className="px-4 py-3 font-medium">{u.username}</td>
                  <td className="px-4 py-3 text-base-content/60">{u.email ?? '—'}</td>
                  <td className="px-4 py-3 text-right">
                    {isLastAdmin ? (
                      <span className="text-xs text-base-content/40">—</span>
                    ) : (
                      <button
                        onClick={() => {
                          if (window.confirm(t('platformUsers.confirmRevoke', { username: u.username }))) {
                            grantMutation.mutate({ userId: u.id, role: null });
                          }
                        }}
                        className="text-xs px-2.5 py-1 rounded-md text-red-500/80 hover:bg-red-500/5"
                      >
                        {t('platformUsers.revoke')}
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {isLastAdmin && (
          <p className="text-xs text-base-content/40 mt-2">{t('platformUsers.lastAdminHint')}</p>
        )}
      </section>

      {/* Add admin: search + grant */}
      <section>
        <h2 className="text-sm font-semibold uppercase tracking-[0.12em] text-base-content/40 mb-3">
          {t('platformUsers.addAdmin')}
        </h2>
        <div className="relative mb-3">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/30" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('platformUsers.searchPlaceholder')}
            className="w-full pl-10 pr-4 py-2 rounded-lg border border-base-300/60 bg-base-100 text-sm focus:outline-none focus:border-primary"
          />
        </div>
        {query && (
          <div className="rounded-lg border border-base-300/60 bg-base-100 overflow-hidden">
            {candidates.length === 0 ? (
              <p className="px-4 py-3 text-sm text-base-content/50">{t('platformUsers.noCandidates')}</p>
            ) : (
              <ul>
                {candidates.map((u: PlatformUserBrief) => (
                  <li key={u.id} className="flex items-center justify-between px-4 py-2.5 border-t border-base-300/40 first:border-t-0">
                    <div>
                      <div className="font-medium text-sm">{u.username}</div>
                      <div className="text-xs text-base-content/50">{u.email ?? '—'}</div>
                    </div>
                    <button
                      onClick={() => grantMutation.mutate({ userId: u.id, role: 'platform_admin' })}
                      className="text-xs px-2.5 py-1 rounded-md bg-primary/10 text-primary hover:bg-primary/15"
                    >
                      {t('platformUsers.grant')}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </section>

      <p className="text-xs text-base-content/40 mt-8">{t('platformUsers.stalenessNote')}</p>
    </div>
  );
}
```

- [ ] **Step 4: Add i18n keys**

In `web/src/i18n/en.json`, add:

```json
"platformUsers": {
  "title": "Platform users",
  "subtitle": "Manage who has platform-level administrator access.",
  "currentAdmins": "Current platform admins",
  "addAdmin": "Add platform admin",
  "searchPlaceholder": "Search by username or email",
  "noCandidates": "No users found.",
  "grant": "Grant",
  "revoke": "Revoke",
  "confirmRevoke": "Revoke platform admin from {{username}}?",
  "lastAdminHint": "You are the only platform admin. Promote another user before revoking.",
  "stalenessNote": "Role changes take effect on the user's next login (access-token refresh)."
}
```

In `web/src/i18n/zh.json`:

```json
"platformUsers": {
  "title": "平台管理员",
  "subtitle": "管理具有平台级别管理员权限的用户。",
  "currentAdmins": "当前平台管理员",
  "addAdmin": "添加平台管理员",
  "searchPlaceholder": "按用户名或邮箱搜索",
  "noCandidates": "未找到用户。",
  "grant": "授予",
  "revoke": "撤销",
  "confirmRevoke": "确认撤销 {{username}} 的平台管理员权限？",
  "lastAdminHint": "您是唯一的平台管理员。请先添加其他管理员再撤销。",
  "stalenessNote": "权限变更将在用户下次登录（access-token 刷新）时生效。"
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/pages/PlatformUsers.test.tsx 2>&1 | tail -15`

Expected: 4/4 PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/PlatformUsers.tsx web/src/pages/PlatformUsers.test.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): add PlatformUsers page for ongoing grant/revoke"
```

---

## Task 10: Restructure App.tsx — top-level /admin/* routes + redirect from old URL

**Files:**
- Modify: `web/src/App.tsx`

- [ ] **Step 1: Write a routing test**

Append to `web/src/App.test.tsx` (or create it):

```tsx
import { describe, it, expect } from 'vitest';
import { renderWithProviders } from './test/render';
import { screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import App from './App';

describe('App routing', () => {
  it('renders PlatformLayout chrome on /admin/settings', () => {
    renderWithProviders(<App />, { route: '/admin/settings' });
    // The Platform sidebar shows the "Platform" heading.
    expect(screen.getByText('Platform')).toBeInTheDocument();
  });

  it('redirects /:slug/admin/settings to /admin/settings', async () => {
    renderWithProviders(<App />, { route: '/test-org/admin/settings' });
    // After Navigate fires, the URL changes to /admin/settings and the
    // PlatformLayout renders.
    expect(await screen.findByText('Platform')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/App.test.tsx 2>&1 | tail -10`

Expected: FAIL — routes not yet wired.

- [ ] **Step 3: Restructure App.tsx**

In `web/src/App.tsx`:

1. Add imports:
   ```tsx
   import PlatformLayout from './components/PlatformLayout';
   import PlatformUsers from './pages/PlatformUsers';
   ```

2. Remove the existing `RequirePlatformAdmin` block at lines 174-178 (which sits inside `/:orgSlug/` and wraps `admin/settings`):
   ```tsx
   {/* DELETE THIS BLOCK:
   <Route element={<RequirePlatformAdmin />}>
     <Route element={<OrgRouteGuard />}>
       <Route path="admin/settings" element={<Settings />} />
     </Route>
   </Route>
   */}
   ```

3. Add a new top-level route group outside `/:orgSlug` (after the closing `</Route>` of `/:orgSlug`, before the legacy redirects):
   ```tsx
   {/* Platform-scoped routes (top-level, no org prefix) */}
   <Route element={<RequirePlatformAdmin />}>
     <Route element={<PlatformLayout />}>
       <Route path="/admin/settings" element={<Settings />} />
       <Route path="/admin/platform-users" element={<PlatformUsers />} />
     </Route>
   </Route>
   ```

4. Add a client-side redirect from the old URL. Place this just before the catch-all `<Route path="*" element={<Navigate to="/" replace />} />`:
   ```tsx
   {/* Backward-compat: redirect just-shipped /{slug}/admin/settings to /admin/settings */}
   <Route path="/:orgSlug/admin/settings" element={<Navigate to="/admin/settings" replace />} />
   ```

   (This goes OUTSIDE `RequirePlatformAdmin` so an org admin who hits the old URL still gets redirected, not 403'd.)

- [ ] **Step 4: Run the routing tests**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/App.test.tsx 2>&1 | tail -10`

Expected: 2/2 PASS.

- [ ] **Step 5: Run the full frontend test suite to catch regressions**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run 2>&1 | tail -20`

Expected: All existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add web/src/App.tsx web/src/App.test.tsx
git commit -m "refactor(web): move /admin/settings to top-level route, add /admin/platform-users"
```

---

## Task 11: Update Layout.tsx sidebar links to top-level URLs

**Files:**
- Modify: `web/src/components/Layout.tsx`

- [ ] **Step 1: Find and update the two URLs**

In `Layout.tsx`:

1. The `platformItems` array (around line 90): change the `key` from `\`/${slug}/admin/settings\`` to `/admin/settings`.
2. After the existing `platformItems` definition, add a second entry:
   ```tsx
   const platformItems = [
     { key: '/admin/settings', icon: Settings, label: t('sidebar.settings') },
     { key: '/admin/platform-users', icon: Users, label: t('sidebar.platformUsers') },
   ];
   ```
3. Add `Users` to the lucide-react imports at the top of the file.

- [ ] **Step 2: Update the Layout.test.tsx assertions**

The existing test asserts:
```tsx
const settingsItems = screen.getAllByText('Settings');
expect(settingsItems.length).toBeGreaterThanOrEqual(1);
```

Add a new assertion for the Platform Users link:

```tsx
it('shows the Platform Users link under Platform group', () => {
  useAuthStore.setState({ user: platformAdminUser, currentOrg: adminOrg });
  renderWithProviders(<AppLayout />, { route: '/test-org/dashboard' });
  expect(screen.getByText('Platform Users')).toBeInTheDocument();
});
```

- [ ] **Step 3: Run tests**

Run: `cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run src/components/Layout.test.tsx 2>&1 | tail -10`

Expected: 4/4 PASS (3 original + 1 new).

- [ ] **Step 4: Commit**

```bash
git add web/src/components/Layout.tsx web/src/components/Layout.test.tsx
git commit -m "feat(web): add Platform Users link to sidebar; point Settings to top-level URL"
```

---

## Task 12: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add an "Unreleased" section if not present**

At the top of `CHANGELOG.md`, add:

```markdown
## [Unreleased]

### Added
- **Platform admin bootstrap & management.** Operators can now grant `platform_role = platform_admin` to users through a new top-level UI page (`/admin/platform-users`) without manual SQL. A new `cargo run -p llm-gateway -- grant-platform-admin --username <name>` CLI subcommand handles bootstrap when the first-user auto-promotion is disabled. A new `auth.first_user_is_admin` config flag (default `true`, preserves existing behavior) gates the silent first-user promotion. Platform-scoped routes moved from `/{org_slug}/admin/*` to top-level `/admin/*` with a dedicated `PlatformLayout` chrome and a client-side redirect from the just-shipped `/{slug}/admin/settings` URL.

### Changed
- `/{slug}/admin/settings` → `/admin/settings` (client-side redirect preserves bookmarks).
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add CHANGELOG entry for platform-admin bootstrap"
```

---

## Verification

After all tasks complete:

1. **Backend tests pass:**
   ```bash
   cd /workspace/llm-gateway && cargo test --test test_platform_role_storage --test test_first_user_promotion --test test_admin_users --test test_settings --test cli_smoke
   ```
   Expected: all green.

2. **Full Rust test suite passes:**
   ```bash
   cd /workspace/llm-gateway && cargo test --workspace 2>&1 | tail -10
   ```
   Expected: no regressions.

3. **Frontend tests pass:**
   ```bash
   cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm test -- --run 2>&1 | tail -10
   ```
   Expected: all green (existing + new).

4. **TypeScript + Vite build clean:**
   ```bash
   cd /workspace/llm-gateway/web && source ~/.nvm/nvm.sh && npm run build 2>&1 | tail -10
   ```
   Expected: build succeeds.

5. **Manual smoke test (4 scenarios):**
   1. Start gateway on empty DB (default config) → register user A via the UI → A is platform_admin → Platform sidebar visible → A sees Settings + Platform Users entries.
   2. From a second shell on the same DB: `cargo run -p llm-gateway -- grant-platform-admin --username A --revoke` → A's `platform_role` becomes NULL. A's existing JWT still works until expiry. After A logs out + logs back in, Platform sidebar disappears.
   3. Set `auth.first_user_is_admin = false`, drop the DB, restart gateway. Register user A → A has `platform_role = NULL`, no Platform sidebar. Run `grant-platform-admin --username A` → A is admin.
   4. As platform admin, log in → navigate to `/admin/platform-users` → see yourself in the table with no Revoke button (last-admin guard). From a separate browser session, demote yourself (fails because last admin). Promote a second user → both visible → revoke yourself succeeds.

## Out of Scope

- `granted_by` / `granted_at` audit columns (spec D6 — YAGNI).
- Multi-role platform hierarchy (e.g. `platform_moderator`).
- Immediate revocation via token versioning (≤15 min staleness accepted).
- Per-org "delegate admin" role (org_admin covers this).
- Audit event log for grant/revoke events (request audit captures the PATCH itself).