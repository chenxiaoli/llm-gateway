# User Groups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a single `groups` table referenced by both `users` and `channels`, with the access rule "a user can only use channels in the same group (or channels with no group)."

**Architecture:** Normalize the existing free-form `channels.group TEXT` column into a `channels.group_id` FK to a new `groups` table. Add `users.group_id` FK. Filter routing candidates in `proxy_inner` based on the requesting user's group. New admin `/admin/groups` CRUD endpoint and new `Groups` admin page.

**Tech Stack:** Rust (Axum, sqlx, Postgres), React + TypeScript + Vite, React Query, Zustand, Tailwind/DaisyUI, vitest + MSW.

**Spec:** `docs/superpowers/specs/2026-06-24-user-groups-design.md`

---

## File Structure

**Backend (Rust):**

| File | Action | Responsibility |
| ---- | ------ | -------------- |
| `crates/storage/migrations/postgres/20260624000001_user_groups.sql` | Create | New migration: groups table, channel.group refactor, user.group_id |
| `crates/storage/src/types.rs` | Modify | Add `Group`, `CreateGroup`, `UpdateGroup`; replace `Channel.group` with `group_id`; extend `User` with `group_id` |
| `crates/storage/src/lib.rs` | Modify | Extend `Storage` trait with group methods |
| `crates/storage/src/postgres.rs` | Modify | Implement group methods; update existing channel/user queries to use `group_id` and join `group_name` |
| `crates/api/src/management/groups.rs` | Create | New module: groups CRUD endpoints |
| `crates/api/src/management/mod.rs` | Modify | Register `groups` module + routes |
| `crates/api/src/management/users.rs` | Modify | Extend `update_user` and `UserResponse` with `group_id`/`group_name` |
| `crates/api/src/management/channels.rs` | Modify | Replace `group` with `group_id`; include `group_name` in response |
| `crates/api/src/proxy.rs` | Modify | Apply user-group filter in routing; add `group_id` to `ResolvedChannel` |
| `crates/api/tests/test_user_groups.rs` | Create | Group CRUD tests + routing filter tests |

**Frontend (TypeScript/React):**

| File | Action | Responsibility |
| ---- | ------ | -------------- |
| `web/src/types.ts` | Modify | Add `Group`, group-related request types; extend `User`/`Channel` |
| `web/src/api/groups.ts` | Create | Group API client functions |
| `web/src/api/users.ts` | Modify | (No change needed — uses generic `UpdateUserRequest` already) |
| `web/src/api/providers.ts` | Modify | Update `createChannel` signature to use `group_id` |
| `web/src/hooks/useGroups.ts` | Create | React Query hooks for groups |
| `web/src/pages/Groups.tsx` | Create | New admin Groups page (list + create/edit drawer) |
| `web/src/pages/Groups.test.tsx` | Create | Groups page tests |
| `web/src/pages/Users.tsx` | Modify | Add group selector in `UserDrawer` |
| `web/src/pages/Channels.tsx` | Modify | Replace free-form group input with `Select` |
| `web/src/pages/ChannelDetail.tsx` | Modify | Replace free-form group input with `Select` |
| `web/src/components/Layout.tsx` | Modify | Add "Groups" sidebar entry |
| `web/src/App.tsx` | Modify | Add `/admin/groups` route |
| `web/src/i18n/en.json`, `web/src/i18n/zh.json` | Modify | New strings for `groups.*` |

---

## Task 1: Set up feature branch and database migration

**Files:**
- Create: `crates/storage/migrations/postgres/20260624000001_user_groups.sql`

- [ ] **Step 1: Create feature branch from develop**

```bash
git checkout develop
git pull origin develop
git checkout -b feature/user-groups
```

- [ ] **Step 2: Write the migration SQL**

Create `crates/storage/migrations/postgres/20260624000001_user_groups.sql` with this exact content:

```sql
-- User Groups: add canonical groups table, normalize channels.group to FK, add users.group_id

CREATE TABLE groups (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Backfill groups from existing distinct channel.group values (idempotent on re-run)
INSERT INTO groups (id, name)
SELECT gen_random_uuid()::text, "group"
FROM (SELECT DISTINCT "group" FROM channels WHERE "group" IS NOT NULL) t
ON CONFLICT (name) DO NOTHING;

-- Add channels.group_id column
ALTER TABLE channels ADD COLUMN group_id TEXT REFERENCES groups(id) ON DELETE SET NULL;

-- Backfill channels.group_id by matching the legacy name
UPDATE channels c
SET group_id = g.id
FROM groups g
WHERE c."group" = g.name;

-- Verify all non-null legacy groups got backfilled (should be 0 rows)
DO $$
DECLARE
    unbackfilled_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO unbackfilled_count
    FROM channels WHERE "group" IS NOT NULL AND group_id IS NULL;
    IF unbackfilled_count > 0 THEN
        RAISE EXCEPTION 'Backfill failed: % channels had a group but no matching groups row', unbackfilled_count;
    END IF;
END$$;

-- Drop legacy column
ALTER TABLE channels DROP COLUMN "group";

-- Add users.group_id
ALTER TABLE users ADD COLUMN group_id TEXT REFERENCES groups(id) ON DELETE SET NULL;
```

- [ ] **Step 3: Run cargo check to verify SQLx migration is picked up**

Run: `cargo check --workspace`
Expected: Compiles without errors (the migration file is just SQL, picked up at test/run time).

- [ ] **Step 4: Verify migration runs on a clean test DB**

Run: `cargo test --workspace -p llm-gateway-storage --no-run`
Expected: Compiles successfully. (The actual migration runs inside `sqlx::test` at test time.)

- [ ] **Step 5: Commit**

```bash
git add crates/storage/migrations/postgres/20260624000001_user_groups.sql
git commit -m "feat(storage): add user_groups migration with channel.group refactor"
```

---

## Task 2: Add Group types to storage layer

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add Group struct and request types**

In `crates/storage/src/types.rs`, add a new section after the Users section (around line 715, after `UpdateUser`):

```rust
// --- Groups ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroup {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct DeleteGroupResult {
    pub cleared_users: i64,
    pub cleared_channels: i64,
}
```

- [ ] **Step 2: Replace `group` with `group_id` on Channel struct**

In `crates/storage/src/types.rs`, find the `Channel` struct (around line 159) and change:

```rust
    pub group: Option<String>,
```

to:

```rust
    pub group_id: Option<String>,
```

Do the same for `CreateChannel` (around line 196) and `UpdateChannel` (around line 213). The `UpdateChannel` field becomes:

```rust
    pub group_id: Option<Option<String>>,
```

- [ ] **Step 3: Add `group_id` to User struct**

In `crates/storage/src/types.rs`, find the `User` struct (around line 680) and add `group_id` before `created_at`:

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

Also add to `UserWithBalance` (around line 692):

```rust
pub struct UserWithBalance {
    pub id: String,
    pub username: String,
    pub role: String,
    pub enabled: bool,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub balance: i64,
    pub threshold: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Add `group_id` to `UpdateUser`**

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub role: Option<String>,
    pub enabled: Option<bool>,
    pub group_id: Option<Option<String>>,
}
```

- [ ] **Step 5: Run `cargo check` to find all the broken call sites**

Run: `cargo check --workspace`
Expected: Errors in `crates/storage/src/postgres.rs` (channel SQL references `"group"` and struct construction sets `group:` field). Do NOT fix yet — Task 3 handles that.

- [ ] **Step 6: Commit (will not compile yet — expected)**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(storage): add Group types, refactor Channel/User group fields"
```

Note: The repo will not compile until Task 3 completes. That's intentional — keeping the type changes in their own commit makes review easier.

---

## Task 3: Update Postgres storage queries

**Files:**
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Add `group_id` and `group_name` to channel SELECT queries**

Search `crates/storage/src/postgres.rs` for `SELECT ... FROM channels` queries. Each one needs:
- Replace `"group"` with `group_id` in the column list
- Add a LEFT JOIN to groups for `group_name`

For the main `list_channels` and `get_channel` queries, the SELECT becomes:
```sql
SELECT c.id, c.provider_id, c.name, c.api_key, c.priority, c.pricing_policy_id,
       c.markup_ratio, c.rpm_limit, c.tpm_limit, c.balance, c.weight, c.enabled,
       c.disabled_until, c.available_hours, c.created_by, c.group_id,
       g.name AS group_name,
       c.created_at, c.updated_at
FROM channels c
LEFT JOIN groups g ON g.id = c.group_id
```

Update the `sqlx::query_as::<_, Channel>` row mapping to map `group_id: Option<String>` from the `group_id` column. The `group_name` field is only used by the management layer — it's a separate query if needed (see Task 5 for the API response).

Actually, simpler approach: the `Channel` struct doesn't have `group_name` (per Task 2 spec). Only `UserWithBalance` does. So `group_name` is fetched via a JOIN only for user/channel list responses in the API layer. The storage layer returns `group_id` only.

Simplify the channel SELECT to:
```sql
SELECT id, provider_id, name, api_key, priority, pricing_policy_id,
       markup_ratio, rpm_limit, tpm_limit, balance, weight, enabled,
       disabled_until, available_hours, created_by, group_id,
       created_at, updated_at
FROM channels
```

(The `LEFT JOIN groups` is not needed for the storage layer.)

- [ ] **Step 2: Update channel INSERT and UPDATE queries**

For `create_channel`:
- Add `group_id` to the column list and VALUES placeholder
- Bind `channel.group_id` (was `channel.group`)

For `update_channel`:
- Replace `"group" = $N` with `group_id = $N` in SET clause
- Bind `channel.group_id`

- [ ] **Step 3: Update user queries to include `group_id`**

In `list_users_paginated`, `get_user`, `create_user`, `update_user`, and anywhere `User` is constructed:
- Add `group_id` to SELECT column lists
- Add to INSERT/UPDATE column lists
- Map it in `query_as` row parsers

For `list_users_paginated` (returns `UserWithBalance`), also JOIN groups for `group_name`:
```sql
SELECT u.id, u.username, u.role, u.enabled, u.group_id, g.name AS group_name,
       COALESCE(a.balance, 0) AS balance, COALESCE(a.threshold, 0) AS threshold,
       u.created_at, u.updated_at
FROM users u
LEFT JOIN accounts a ON a.user_id = u.id
LEFT JOIN groups g ON g.id = u.group_id
```

- [ ] **Step 4: Run `cargo check --workspace` to verify the existing queries compile**

Run: `cargo check --workspace`
Expected: Compiles cleanly. New group methods (added in Task 4) are not yet called, so no missing-method errors.

- [ ] **Step 5: Run existing tests to verify no regressions**

Run: `cargo test --workspace`
Expected: All existing tests pass. (Each test spins up a fresh DB via `sqlx::test` and runs all migrations including the new one.)

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat(storage): migrate channel/user queries to group_id"
```

---

## Task 4: Add group storage methods

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Extend the `Storage` trait**

In `crates/storage/src/lib.rs`, find the `Storage` trait definition. Add these methods inside the trait (note: existing methods return `Result<_, sqlx::Error>`; we match that):

```rust
async fn list_groups(&self) -> Result<Vec<Group>, sqlx::Error>;
async fn get_group(&self, id: &str) -> Result<Option<Group>, sqlx::Error>;
async fn create_group(&self, input: &CreateGroup) -> Result<Group, sqlx::Error>;
async fn update_group(&self, id: &str, input: &UpdateGroup) -> Result<Group, sqlx::Error>;
async fn delete_group(&self, id: &str) -> Result<DeleteGroupResult, sqlx::Error>;
async fn get_user_group_id(&self, user_id: &str) -> Result<Option<String>, sqlx::Error>;
```

Also add `Group`, `CreateGroup`, `UpdateGroup`, `DeleteGroupResult` to the public exports (likely already done via `pub use types::*;` if the crate has that pattern — check the existing exports).

- [ ] **Step 2: Implement group methods in PostgresStorage**

In `crates/storage/src/postgres.rs`, add implementations (returning `sqlx::Error` directly — no custom error type):

```rust
async fn list_groups(&self) -> Result<Vec<Group>, sqlx::Error> {
    let rows = sqlx::query_as!(
        Group,
        r#"SELECT id, name, description, created_at, updated_at FROM groups ORDER BY name"#
    )
    .fetch_all(&*self.pool)
    .await?;
    Ok(rows)
}

async fn get_group(&self, id: &str) -> Result<Option<Group>, sqlx::Error> {
    let row = sqlx::query_as!(
        Group,
        r#"SELECT id, name, description, created_at, updated_at FROM groups WHERE id = $1"#,
        id
    )
    .fetch_optional(&*self.pool)
    .await?;
    Ok(row)
}

async fn create_group(&self, input: &CreateGroup) -> Result<Group, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let row = sqlx::query_as!(
        Group,
        r#"INSERT INTO groups (id, name, description)
           VALUES ($1, $2, $3)
           RETURNING id, name, description, created_at, updated_at"#,
        id,
        input.name,
        input.description
    )
    .fetch_one(&*self.pool)
    .await?;
    Ok(row)
}

async fn update_group(&self, id: &str, input: &UpdateGroup) -> Result<Group, sqlx::Error> {
    let name = input.name.clone();
    let description = input.description.clone().flatten();
    let row = sqlx::query_as!(
        Group,
        r#"UPDATE groups
           SET name = COALESCE($2, name),
               description = CASE WHEN $3::text IS NULL THEN description ELSE $3 END,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, description, created_at, updated_at"#,
        id,
        name,
        description.as_deref()
    )
    .fetch_optional(&*self.pool)
    .await?
    .ok_or(sqlx::Error::RowNotFound)?;
    Ok(row)
}

async fn delete_group(&self, id: &str) -> Result<DeleteGroupResult, sqlx::Error> {
    let cleared_users = sqlx::query!(
        r#"UPDATE users SET group_id = NULL WHERE group_id = $1"#,
        id
    )
    .execute(&*self.pool)
    .await?
    .rows_affected() as i64;

    let cleared_channels = sqlx::query!(
        r#"UPDATE channels SET group_id = NULL WHERE group_id = $1"#,
        id
    )
    .execute(&*self.pool)
    .await?
    .rows_affected() as i64;

    sqlx::query!(r#"DELETE FROM groups WHERE id = $1"#, id)
        .execute(&*self.pool)
        .await?;

    Ok(DeleteGroupResult { cleared_users, cleared_channels })
}

async fn get_user_group_id(&self, user_id: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_scalar!(
        r#"SELECT group_id FROM users WHERE id = $1"#,
        user_id
    )
    .fetch_optional(&*self.pool)
    .await?;
    Ok(row.flatten())
}
```

Note: unique-violation detection on `create_group`/`update_group` happens in the API layer (Task 5) — match `sqlx::Error::Database(db) if db.is_unique_violation()` and map to `ApiError::Conflict`. This matches the existing pattern in this crate (no `StorageError` enum).

- [ ] **Step 3: Run cargo check**

Run: `cargo check --workspace`
Expected: Compiles cleanly.

- [ ] **Step 4: Run storage-level tests**

Run: `cargo test --workspace -p llm-gateway-storage`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/postgres.rs
git commit -m "feat(storage): add Group CRUD methods"
```

---

## Task 5: Add Groups management API endpoints

**Files:**
- Create: `crates/api/src/management/groups.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: Create the groups module**

Create `crates/api/src/management/groups.rs` with:

```rust
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use llm_gateway_storage::{CreateGroup, DeleteGroupResult, Group, UpdateGroup};
use serde::Serialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Group> for GroupResponse {
    fn from(g: Group) -> Self {
        GroupResponse {
            id: g.id,
            name: g.name,
            description: g.description,
            created_at: g.created_at.to_rfc3339(),
            updated_at: g.updated_at.to_rfc3339(),
        }
    }
}

pub async fn list_groups(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<GroupResponse>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let groups = state.storage.list_groups().await.map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(groups.into_iter().map(GroupResponse::from).collect()))
}

pub async fn get_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<GroupResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let group = state.storage.get_group(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Group '{}' not found", id)))?;
    Ok(Json(GroupResponse::from(group)))
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateGroup>,
) -> Result<Json<GroupResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let group = state.storage.create_group(&input).await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                ApiError::Conflict(format!("Group '{}' already exists", input.name))
            }
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(GroupResponse::from(group)))
}

pub async fn update_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateGroup>,
) -> Result<Json<GroupResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let group = state.storage.update_group(&id, &input).await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                ApiError::Conflict(format!("Group name '{}' already exists", input.name.clone().unwrap_or_default()))
            }
            sqlx::Error::RowNotFound => ApiError::NotFound(format!("Group '{}' not found", id)),
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(GroupResponse::from(group)))
}

pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DeleteGroupResult>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let result = state.storage.delete_group(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(result))
}
```

- [ ] **Step 2: Verify ApiError::Conflict exists**

Check `crates/api/src/error.rs` for `Conflict` variant. If missing, add it (with HTTP 409):

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    // ... existing variants
    #[error("{0}")]
    Conflict(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            // ... existing
            ApiError::Conflict(_) => (StatusCode::CONFLICT, "Conflict"),
            // ...
        };
        // ...
    }
}
```

- [ ] **Step 3: Register module and routes in management/mod.rs**

In `crates/api/src/management/mod.rs`, add `pub mod groups;` to the module declarations (after `pub mod channels;`).

In `management_router`, add after the channels routes:

```rust
// Groups (admin)
.route(
    "/api/v1/admin/groups",
    post(groups::create_group).get(groups::list_groups),
)
.route(
    "/api/v1/admin/groups/{id}",
    get(groups::get_group).patch(groups::update_group).delete(groups::delete_group),
)
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check --workspace`
Expected: Compiles cleanly.

- [ ] **Step 5: Write the failing test**

Create `crates/api/tests/test_user_groups.rs` with:

```rust
mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

async fn register_admin(app: &axum::Router) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "admin", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    body["token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_create_group_succeeds(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let token = register_admin(&app).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["name"], "engineering");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_duplicate_group_name_returns_409(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let token = register_admin(&app).await;

    // Create first
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Create duplicate
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_list_groups_returns_all(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let token = register_admin(&app).await;

    for name in ["engineering", "marketing", "data-team"] {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/groups")
                    .header("authorization", bearer_token(&token))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"name": name}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_delete_group_clears_user_channel_references(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let token = register_admin(&app).await;

    // Create a group
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let group_id = body["id"].as_str().unwrap();

    // Delete
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/admin/groups/{}", group_id))
                .header("authorization", bearer_token(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["cleared_users"], 0);
    assert_eq!(body["cleared_channels"], 0);
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test --workspace -p llm-gateway-api --test test_user_groups`
Expected: All 4 tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/management/groups.rs crates/api/src/management/mod.rs crates/api/src/error.rs crates/api/tests/test_user_groups.rs
git commit -m "feat(api): add /admin/groups CRUD endpoints with tests"
```

---

## Task 6: Wire group_id into User and Channel PATCH endpoints

**Files:**
- Modify: `crates/api/src/management/users.rs`
- Modify: `crates/api/src/management/channels.rs`

- [ ] **Step 1: Extend UserResponse with group_id and group_name**

In `crates/api/src/management/users.rs`, update `UserResponse` (line 13):

```rust
#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub enabled: bool,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub balance: f64,
    pub threshold: f64,
    pub created_at: String,
    pub updated_at: String,
}
```

Update both `From<UserWithBalance>` and `From<&User>` impls to populate `group_id` and `group_name`. For `From<&User>`, `group_name` will be `None` (since `User` doesn't carry it).

- [ ] **Step 2: Apply group_id in update_user handler**

In `crates/api/src/management/users.rs` `update_user`, after the role/enabled blocks (around line 101), add:

```rust
if let Some(group_id) = input.group_id.clone() {
    if let Some(ref gid) = group_id {
        // Validate group exists
        let exists = state.storage.get_group(gid).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        if exists.is_none() {
            return Err(ApiError::BadRequest(format!("Group '{}' not found", gid)));
        }
    }
    user.group_id = group_id;
}
```

- [ ] **Step 3: Replace group with group_id in channel response and update**

In `crates/api/src/management/channels.rs`:
- Update `ChannelResponse` to replace `pub group: Option<String>` with `pub group_id: Option<String>` and add `pub group_name: Option<String>`.
- Update `From<Channel>` to populate `group_id`. The `group_name` requires a JOIN or a separate lookup — easiest is to do a single lookup at the handler level. Simpler approach: skip `group_name` in the channel response for now (frontend can look it up from `useGroups`). Document this as a small spec deviation.

Actually, to keep parity with the spec, do the lookup in the response builder:

In `From<Channel>`:
```rust
group_id: c.group_id.clone(),
group_name: None, // populated by handler if needed
```

In `get_channel`, `list_all_channels`, etc., after fetching the channel(s), do a single `list_groups()` lookup and join client-side. This avoids N+1.

- [ ] **Step 4: Replace group with group_id in update_channel**

In `crates/api/src/management/channels.rs` `update_channel` (around line 398-400), change:

```rust
if let Some(group) = input.group {
    channel.group = group;
}
```

to:

```rust
if let Some(group_id) = input.group_id.clone() {
    if let Some(ref gid) = group_id {
        let exists = state.storage.get_group(gid).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        if exists.is_none() {
            return Err(ApiError::BadRequest(format!("Group '{}' not found", gid)));
        }
    }
    channel.group_id = group_id;
}
```

Also update `create_channel` to set `channel.group_id = input.group_id;` instead of `channel.group = input.group;`.

- [ ] **Step 5: Run cargo check and fix any remaining field references**

Run: `cargo check --workspace`
Expected: Compiles cleanly.

- [ ] **Step 6: Add user-channel group_id integration tests**

Append to `crates/api/tests/test_user_groups.rs`:

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_group_id_assigns_group(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let token = register_admin(&app).await;

    // Register a regular user
    let user_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "regular", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let user_body: Value = serde_json::from_slice(&to_bytes(user_resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let user_id = user_body["user"]["id"].as_str().unwrap();

    // Create a group
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let group_id = body["id"].as_str().unwrap();

    // Assign user to group
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"group_id": group_id}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["group_id"], group_id);
    assert_eq!(body["group_name"], "engineering");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_group_id_nonexistent_returns_400(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let token = register_admin(&app).await;

    // Register a regular user
    let user_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "regular", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let user_body: Value = serde_json::from_slice(&to_bytes(user_resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let user_id = user_body["user"]["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"group_id": "nonexistent-group-id"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_clear_group_id_with_null(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let token = register_admin(&app).await;

    // Register a regular user
    let user_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "regular", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let user_body: Value = serde_json::from_slice(&to_bytes(user_resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let user_id = user_body["user"]["id"].as_str().unwrap();

    // Clear group_id (no prior assignment)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(&token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"group_id": null}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(body["group_id"].is_null());
}
```

- [ ] **Step 7: Run the new tests**

Run: `cargo test --workspace -p llm-gateway-api --test test_user_groups`
Expected: All tests pass (the 4 from Task 5 plus the 3 new ones here).

- [ ] **Step 8: Run the full backend test suite**

Run: `cargo test --workspace`
Expected: All tests pass. Fix any regressions in tests that still reference the old `group` field.

- [ ] **Step 9: Commit**

```bash
git add crates/api/src/management/users.rs crates/api/src/management/channels.rs crates/api/tests/test_user_groups.rs
git commit -m "feat(api): wire group_id into user/channel PATCH + responses"
```

---

## Task 7: Apply user-group routing filter

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: Add `group_id` field to `ResolvedChannel`**

In `crates/api/src/proxy.rs`, find the `ResolvedChannel` struct definition and add `pub group_id: Option<String>`. Update the construction site in `do_reload` (around line 227) to populate it:

```rust
let resolved = ResolvedChannel {
    // ... existing fields
    group_id: channel.group_id.clone(),
    // ...
};
```

Also update the `resolved_channel_carries_all_fields` test in the `#[cfg(test)] mod tests` block (around line 1590) to include `group_id: None,` in the struct literal.

- [ ] **Step 2: Plumb user_id and is_admin into the routing section**

In `proxy_inner` (around line 811), after the balance check, before model parsing, add:

```rust
let request_user_id = api_key.created_by.clone();
let request_is_admin = if let Some(ref uid) = request_user_id {
    match state.storage.get_user(uid).await {
        Ok(Some(u)) => u.role == "admin",
        Ok(None) => false,
        Err(e) => {
            tracing::warn!("[PROXY] Failed to look up user role for {}: {}", uid, e);
            false  // Fail-safe: treat as non-admin if lookup fails
        }
    }
} else {
    false  // No user_id (legacy admin-created keys) — no filter applied since user_id is None
};
```

- [ ] **Step 3: Apply the filter on the cached-path candidates**

Find the cache-hit path where `candidates: Vec<(ResolvedChannel, ChannelModel)>` is built (around line 940, the path that uses `state.registry.resolve_by_model`). After candidates are populated, add:

```rust
if let Some(ref user_id) = request_user_id {
    if !request_is_admin {
        match state.storage.get_user_group_id(user_id).await {
            Ok(Some(allowed_group_id)) => {
                candidates.retain(|(rc, _)| {
                    rc.group_id.is_none() || rc.group_id.as_deref() == Some(&allowed_group_id)
                });
            }
            Ok(None) => { /* User has no group — unrestricted */ }
            Err(e) => {
                tracing::warn!("[PROXY] Failed to look up group for user {}: {}", user_id, e);
                /* Fail-open: don't filter */
            }
        }
    }
}
```

- [ ] **Step 4: Apply the same filter on the cache-miss path**

Find the cache-miss path that builds `available_channels: Vec<(&ChannelModel, &Channel)>` (around line 961). After it's populated (before the "no available channels" check at line 975), add:

```rust
if let Some(ref user_id) = request_user_id {
    if !request_is_admin {
        if let Ok(Some(allowed_group_id)) = state.storage.get_user_group_id(user_id).await {
            available_channels.retain(|(_, ch)| {
                ch.group_id.is_none() || ch.group_id.as_deref() == Some(&allowed_group_id)
            });
        }
    }
}
```

- [ ] **Step 5: Run cargo check**

Run: `cargo check --workspace`
Expected: Compiles cleanly.

- [ ] **Step 6: Run the full test suite**

The routing filter logic operates inside `proxy_inner` which requires a full HTTP upstream to test end-to-end. Routing behavior is verified by the manual smoke test in Task 12 Step 3 (scenarios: user in group X hits only X-channel, admin sees all, etc.).

The unit-level invariant — that `get_user_group_id` returns the right value — is implicitly tested by the user PATCH tests in Task 6.

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(api): apply user-group routing filter in proxy_inner"
```

---

## Task 8: Frontend types, API, hooks

**Files:**
- Modify: `web/src/types.ts`
- Create: `web/src/api/groups.ts`
- Create: `web/src/hooks/useGroups.ts`

- [ ] **Step 1: Update TypeScript types**

In `web/src/types.ts`, add:

```typescript
// --- Groups ---

export interface Group {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateGroupRequest {
  name: string;
  description?: string;
}

export interface UpdateGroupRequest {
  name?: string;
  description?: string | null;
}

export interface DeleteGroupResult {
  cleared_users: number;
  cleared_channels: number;
}
```

Update `User`:

```typescript
export interface User {
  id: string;
  username: string;
  role: string;
  enabled: boolean;
  group_id: string | null;
  group_name: string | null;
  created_at: string;
  updated_at: string;
}
```

Update `UpdateUserRequest`:

```typescript
export interface UpdateUserRequest {
  role?: string;
  enabled?: boolean;
  group_id?: string | null;
}
```

Update `Channel` — replace `group?: string` with `group_id?: string | null`. Also update `CreateChannelRequest` and `UpdateChannelRequest`.

- [ ] **Step 2: Create API client for groups**

Create `web/src/api/groups.ts`:

```typescript
import { adminApiClient } from './client';
import type { Group, CreateGroupRequest, UpdateGroupRequest, DeleteGroupResult } from '../types';

export async function listGroups(): Promise<Group[]> {
  const { data } = await adminApiClient.get<Group[]>('/groups');
  return data;
}

export async function getGroup(id: string): Promise<Group> {
  const { data } = await adminApiClient.get<Group>(`/groups/${id}`);
  return data;
}

export async function createGroup(input: CreateGroupRequest): Promise<Group> {
  const { data } = await adminApiClient.post<Group>('/groups', input);
  return data;
}

export async function updateGroup(id: string, input: UpdateGroupRequest): Promise<Group> {
  const { data } = await adminApiClient.patch<Group>(`/groups/${id}`, input);
  return data;
}

export async function deleteGroup(id: string): Promise<DeleteGroupResult> {
  const { data } = await adminApiClient.delete<DeleteGroupResult>(`/groups/${id}`);
  return data;
}
```

- [ ] **Step 3: Create React Query hooks**

Create `web/src/hooks/useGroups.ts`:

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listGroups, createGroup, updateGroup, deleteGroup } from '../api/groups';
import type { CreateGroupRequest, UpdateGroupRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function useGroups() {
  return useQuery({ queryKey: ['groups'], queryFn: listGroups });
}

export function useCreateGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateGroupRequest) => createGroup(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      toast.success(i18n.t('toasts.groupCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.groupCreateFailed'))); },
  });
}

export function useUpdateGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateGroupRequest }) => updateGroup(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      toast.success(i18n.t('toasts.groupUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.groupUpdateFailed'))); },
  });
}

export function useDeleteGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteGroup(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      toast.success(i18n.t('toasts.groupDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.groupDeleteFailed'))); },
  });
}
```

- [ ] **Step 4: Run TypeScript check**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add web/src/types.ts web/src/api/groups.ts web/src/hooks/useGroups.ts
git commit -m "feat(web): add Group types, API client, and React Query hooks"
```

---

## Task 9: Build Groups admin page

**Files:**
- Create: `web/src/pages/Groups.tsx`
- Create: `web/src/pages/Groups.test.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/components/Layout.tsx`

- [ ] **Step 1: Add i18n strings**

In `web/src/i18n/en.json`, add under top-level:

```json
"groups": {
  "title": "Groups",
  "description": "Manage user and channel groups for access control",
  "addGroup": "Add Group",
  "table": {
    "name": "Name",
    "description": "Description",
    "users": "Users",
    "channels": "Channels",
    "actions": "Actions"
  },
  "createModal": {
    "title": "Create Group",
    "name": "Name",
    "namePlaceholder": "e.g. engineering",
    "description": "Description (optional)",
    "descriptionPlaceholder": "Brief description of this group's purpose",
    "createGroup": "Create Group"
  },
  "editModal": {
    "title": "Edit Group",
    "usedBy": "Used by {{users}} users and {{channels}} channels",
    "saveChanges": "Save Changes"
  },
  "deleteConfirm": {
    "title": "Delete group",
    "message": "This will remove the group from {{users}} users and {{channels}} channels. Continue?",
    "confirm": "Delete"
  },
  "noGroups": "No groups yet. Create your first group."
}
```

In `web/src/i18n/zh.json`, mirror these keys with Chinese translations:

```json
"groups": {
  "title": "分组",
  "description": "管理用户和渠道分组以控制访问权限",
  "addGroup": "添加分组",
  "table": {
    "name": "名称",
    "description": "描述",
    "users": "用户",
    "channels": "渠道",
    "actions": "操作"
  },
  "createModal": {
    "title": "创建分组",
    "name": "名称",
    "namePlaceholder": "例如 engineering",
    "description": "描述(可选)",
    "descriptionPlaceholder": "简述此分组的用途",
    "createGroup": "创建分组"
  },
  "editModal": {
    "title": "编辑分组",
    "usedBy": "被 {{users}} 个用户和 {{channels}} 个渠道使用",
    "saveChanges": "保存修改"
  },
  "deleteConfirm": {
    "title": "删除分组",
    "message": "此操作将从此分组移除 {{users}} 个用户和 {{channels}} 个渠道。继续吗?",
    "confirm": "删除"
  },
  "noGroups": "暂无分组。创建第一个分组吧。"
}
```

Also add toast strings in both files:

```json
"toasts": {
  "groupCreated": "Group created",
  "groupCreateFailed": "Failed to create group",
  "groupUpdated": "Group updated",
  "groupUpdateFailed": "Failed to update group",
  "groupDeleted": "Group deleted",
  "groupDeleteFailed": "Failed to delete group"
}
```

(Chinese equivalents: `"分组已创建"`, `"创建分组失败"`, `"分组已更新"`, `"更新分组失败"`, `"分组已删除"`, `"删除分组失败"`.)

- [ ] **Step 2: Create Groups page**

Create `web/src/pages/Groups.tsx`:

```tsx
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Plus, Pencil, Trash2 } from 'lucide-react';
import { useGroups, useCreateGroup, useUpdateGroup, useDeleteGroup } from '../hooks/useGroups';
import { Button } from '../components/ui/Button';
import { Drawer } from '../components/ui/Drawer';
import { Modal } from '../components/ui/Modal';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { toast } from 'sonner';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function Groups() {
  const { t } = useTranslation();
  const { data: groups, isLoading } = useGroups();
  const createMutation = useCreateGroup();
  const updateMutation = useUpdateGroup();
  const deleteMutation = useDeleteGroup();

  const [isAdding, setIsAdding] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState({ name: '', description: '' });

  const openCreate = () => {
    setForm({ name: '', description: '' });
    setIsAdding(true);
  };

  const openEdit = (id: string, name: string, description: string | null) => {
    setForm({ name, description: description ?? '' });
    setEditingId(id);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const input = { name: form.name, description: form.description || undefined };
    try {
      if (editingId) {
        await updateMutation.mutateAsync({ id: editingId, input });
      } else {
        await createMutation.mutateAsync(input);
      }
      setIsAdding(false);
      setEditingId(null);
    } catch {
      // onError handles toast
    }
  };

  return (
    <div className="px-6 pb-8">
      <motion.div
        initial={{ opacity: 0, y: -8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="mb-6 flex items-start justify-between pt-8"
      >
        <div>
          <h1 className="text-xl font-bold tracking-tight text-base-content">{t('groups.title')}</h1>
          <p className="text-md text-base-content/35">{t('groups.description')}</p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} size="sm" onClick={openCreate}>
          {t('groups.addGroup')}
        </Button>
      </motion.div>

      {isLoading ? (
        <div className="flex items-center justify-center py-24">
          <div className="w-8 h-8 rounded-full border-2 border-accent/30 border-t-accent animate-spin" />
        </div>
      ) : groups && groups.length > 0 ? (
        <div className="overflow-x-auto rounded-2xl border border-base-300/40 bg-base-100">
          <table className="table table-sm">
            <thead>
              <tr className="border-b border-base-300/40">
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('groups.table.name')}</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('groups.table.description')}</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('groups.table.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {groups.map((g) => (
                <tr key={g.id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors">
                  <td className="font-mono text-md font-semibold">{g.name}</td>
                  <td className="text-base text-base-content/55">{g.description ?? '-'}</td>
                  <td>
                    <div className="flex items-center gap-1">
                      <Button variant="ghost" size="sm" onClick={() => openEdit(g.id, g.name, g.description)}>
                        <Pencil className="h-3 w-3" /> {t('common.edit')}
                      </Button>
                      <ConfirmDialog
                        title={t('groups.deleteConfirm.title')}
                        onConfirm={() => deleteMutation.mutate(g.id)}
                        okText={t('groups.deleteConfirm.confirm')}
                      >
                        <Button variant="danger" size="sm">
                          <Trash2 className="h-3 w-3" /> {t('common.delete')}
                        </Button>
                      </ConfirmDialog>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div className="text-center py-24 text-base-content/40 text-sm">{t('groups.noGroups')}</div>
      )}

      <Drawer
        open={isAdding || editingId !== null}
        onClose={() => { setIsAdding(false); setEditingId(null); }}
        title={editingId ? t('groups.editModal.title') : t('groups.createModal.title')}
        width={440}
      >
        <form onSubmit={handleSubmit} className="space-y-6">
          <div className="space-y-1.5">
            <label className="text-base font-semibold uppercase tracking-wider text-base-content/50">
              {t('groups.createModal.name')}
            </label>
            <input
              type="text"
              required
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              placeholder={t('groups.createModal.namePlaceholder')}
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-md text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20"
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-base font-semibold uppercase tracking-wider text-base-content/50">
              {t('groups.createModal.description')}
            </label>
            <input
              type="text"
              value={form.description}
              onChange={(e) => setForm({ ...form, description: e.target.value })}
              placeholder={t('groups.createModal.descriptionPlaceholder')}
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-md text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20"
            />
          </div>
          <div className="flex items-center gap-2 pt-2">
            <Button type="submit" variant="primary" loading={createMutation.isPending || updateMutation.isPending} className="flex-1">
              {editingId ? t('groups.editModal.saveChanges') : t('groups.createModal.createGroup')}
            </Button>
            <Button type="button" variant="ghost" onClick={() => { setIsAdding(false); setEditingId(null); }}>
              {t('common.cancel')}
            </Button>
          </div>
        </form>
      </Drawer>
    </div>
  );
}
```

- [ ] **Step 3: Add the route in App.tsx**

In `web/src/App.tsx`, find the admin routes section. Add an import for Groups at the top:

```tsx
import Groups from './pages/Groups';
```

Add a route (after Users or wherever the admin routes are):

```tsx
<Route path="/admin/groups" element={<Groups />} />
```

- [ ] **Step 4: Add sidebar entry in Layout.tsx**

In `web/src/components/Layout.tsx`, find the admin sidebar items (likely an array or list). Add a new entry for Groups with a `Users`-style icon (e.g. `Users` or `Group` from lucide-react):

```tsx
{ path: '/admin/groups', icon: <GroupIcon className="h-4 w-4" />, label: t('groups.title') }
```

(Use whatever icon import the file already uses — check the existing pattern.)

- [ ] **Step 5: Run TypeScript check**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 6: Write the test**

Create `web/src/pages/Groups.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Groups from './Groups';

describe('Groups page', () => {
  it('renders the empty state when no groups', async () => {
    server.use(http.get('*/api/v1/admin/groups', () => HttpResponse.json([])));

    renderWithProviders(<Groups />, { route: '/admin/groups' });

    await waitFor(() => {
      expect(screen.getByText('No groups yet. Create your first group.')).toBeInTheDocument();
    });
  });

  it('renders groups in the table', async () => {
    server.use(
      http.get('*/api/v1/admin/groups', () =>
        HttpResponse.json([
          { id: 'g1', name: 'engineering', description: 'Engineering team', created_at: '', updated_at: '' },
          { id: 'g2', name: 'marketing', description: null, created_at: '', updated_at: '' },
        ]),
      ),
    );

    renderWithProviders(<Groups />, { route: '/admin/groups' });

    await waitFor(() => {
      expect(screen.getByText('engineering')).toBeInTheDocument();
      expect(screen.getByText('marketing')).toBeInTheDocument();
    });
  });

  it('creates a group via the drawer', async () => {
    let capturedBody: unknown = null;
    server.use(
      http.get('*/api/v1/admin/groups', () => HttpResponse.json([])),
      http.post('*/api/v1/admin/groups', async ({ request }) => {
        capturedBody = await request.json();
        return HttpResponse.json({ id: 'g1', name: 'engineering', description: null, created_at: '', updated_at: '' });
      }),
    );

    renderWithProviders(<Groups />, { route: '/admin/groups' });

    await userEvent.click(await screen.findByText('Add Group'));
    await userEvent.type(screen.getByPlaceholderText('e.g. engineering'), 'engineering');
    await userEvent.click(screen.getByRole('button', { name: 'Create Group' }));

    await waitFor(() => {
      expect(capturedBody).toEqual({ name: 'engineering' });
    });
  });

  it('shows backend error on duplicate name', async () => {
    server.use(
      http.get('*/api/v1/admin/groups', () => HttpResponse.json([])),
      http.post('*/api/v1/admin/groups', () =>
        HttpResponse.json({ error: { message: "Group 'engineering' already exists" } }, { status: 409 }),
      ),
    );

    renderWithProviders(<Groups />, { route: '/admin/groups' });

    await userEvent.click(await screen.findByText('Add Group'));
    await userEvent.type(screen.getByPlaceholderText('e.g. engineering'), 'engineering');
    await userEvent.click(screen.getByRole('button', { name: 'Create Group' }));

    await waitFor(() => {
      expect(screen.getByText("Group 'engineering' already exists")).toBeInTheDocument();
    });
  });
});
```

- [ ] **Step 7: Run the tests**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- src/pages/Groups.test.tsx`
Expected: All 4 tests pass.

- [ ] **Step 8: Commit**

```bash
git add web/src/pages/Groups.tsx web/src/pages/Groups.test.tsx web/src/App.tsx web/src/components/Layout.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): add Groups admin page with CRUD + sidebar entry"
```

---

## Task 10: Integrate group selector into Users page

**Files:**
- Modify: `web/src/pages/Users.tsx`

- [ ] **Step 1: Refactor UserDrawer to take the full user object**

In `web/src/pages/Users.tsx`, the `UserDrawer` currently receives only a `userId`. Change the props to receive the full `User` object so we have access to `group_id`/`group_name` without an extra fetch.

Change the function signature (around line 86) from:

```tsx
function UserDrawer({ userId, onClose }: { userId: string | null; onClose: () => void }) {
```

to:

```tsx
function UserDrawer({ user, onClose }: { user: User | null; onClose: () => void }) {
  const userId = user?.id ?? null;
```

Inside the body, replace any reference to the `userId` prop with the local `const userId`. The existing balance-fetching logic continues to use this local `userId`.

In the parent `Users` component (around line 451), change state from `useState<string | null>(null)` to `useState<User | null>(null)`. Change the setter call from `setDrawerUserId(user.id)` (around line 549) to `setDrawerUser(user)`.

Add the import for the `User` type at the top:

```tsx
import type { User } from '../types';
```

- [ ] **Step 2: Add the group selector JSX**

In `web/src/pages/Users.tsx` `UserDrawer`, import the groups hook and add the group `<Select>` inside the JSX. Place it inside the `space-y-6` div, after the account balance card's closing `</div>` (around line 171) and before the `<div className="flex gap-2">` recharge/adjust buttons block.

Add the imports at the top of the file:

```tsx
import { useGroups } from '../hooks/useGroups';
import { Select } from '../components/ui/Select';
```

(The file already imports `Select` - verify by checking the existing imports.)

Inside `UserDrawer`, add the hook:

```tsx
const { data: groups } = useGroups();
```

Then add this JSX block after the account balance card:

```tsx
{user && (
  <div className="rounded-2xl border border-base-300/40 bg-base-100 p-5">
    <div className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-3">
      {t('users.drawer.group')}
    </div>
    <Select
      value={user.group_id ?? ''}
      onChange={(value) => updateMutation.mutate({ id: user.id, input: { group_id: value || null } })}
      options={[
        { value: '', label: t('users.drawer.noGroup') },
        ...(groups ?? []).map((g) => ({ value: g.id, label: g.name })),
      ]}
      size="sm"
    />
  </div>
)}
```

- [ ] **Step 3: Add i18n strings for users.drawer.group and noGroup**

In `web/src/i18n/en.json` under `users.drawer`:

```json
"group": "Group",
"noGroup": "None"
```

In `web/src/i18n/zh.json`:

```json
"group": "分组",
"noGroup": "无"
```

- [ ] **Step 4: Run TypeScript check**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 5: Add a test for the group selector**

Append to `web/src/pages/Users.test.tsx` (or create the file if it doesn't exist):

```tsx
import { describe, it, expect, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Users from './Users';

const { mockToastSuccess } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
}));
vi.mock('sonner', () => ({ toast: { success: mockToastSuccess, error: vi.fn() } }));

describe('Users page', () => {
  it('shows group selector in user drawer and updates on change', async () => {
    let patchBody: unknown = null;
    server.use(
      http.get('*/api/v1/admin/groups', () =>
        HttpResponse.json([
          { id: 'g1', name: 'engineering', description: null, created_at: '', updated_at: '' },
          { id: 'g2', name: 'marketing', description: null, created_at: '', updated_at: '' },
        ]),
      ),
      http.get('*/api/v1/admin/users', () =>
        HttpResponse.json({
          items: [
            {
              id: 'u1',
              username: 'alice',
              role: 'user',
              enabled: true,
              group_id: null,
              group_name: null,
              balance: 100,
              threshold: 10,
              created_at: '2026-06-01T00:00:00Z',
              updated_at: '2026-06-01T00:00:00Z',
            },
          ],
          total: 1,
          page: 1,
          page_size: 20,
        }),
      ),
      http.get('*/api/v1/admin/users/u1/balance', () =>
        HttpResponse.json({
          account: { id: 'a1', user_id: 'u1', balance: 100, threshold: 10, created_at: '', updated_at: '' },
          transactions: { items: [], total: 0, page: 1, page_size: 10 },
        }),
      ),
      http.patch('*/api/v1/admin/users/u1', async ({ request }) => {
        patchBody = await request.json();
        return HttpResponse.json({
          id: 'u1', username: 'alice', role: 'user', enabled: true,
          group_id: 'g1', group_name: 'engineering',
          balance: 100, threshold: 10, created_at: '', updated_at: '',
        });
      }),
    );

    renderWithProviders(<Users />, { route: '/admin/users' });

    await userEvent.click(await screen.findByText('alice'));
    await waitFor(() => {
      expect(screen.getByText('Group')).toBeInTheDocument();
    });

    // Open the group <select> and choose 'engineering'
    const select = screen.getByRole('combobox');
    await userEvent.selectOptions(select, 'g1');

    await waitFor(() => {
      expect(patchBody).toEqual({ group_id: 'g1' });
    });
  });
});
```

- [ ] **Step 6: Run the tests**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- src/pages/Users.test.tsx`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/Users.tsx web/src/pages/Users.test.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): add group selector to UserDrawer"
```

---

## Task 11: Replace free-form group input on Channels page

**Files:**
- Modify: `web/src/pages/Channels.tsx`
- Modify: `web/src/pages/ChannelDetail.tsx`

- [ ] **Step 1: Update Channels.tsx AddChannelDrawer**

In `web/src/pages/Channels.tsx`, find the AddChannelDrawer (around line 128). Replace the existing `<input>` for `group` (lines 320-329) with a `Select`:

```tsx
import { useGroups } from '../hooks/useGroups';
import { Select } from '../components/ui/Select';

// Inside AddChannelDrawer component:
const { data: groups } = useGroups();
const [groupId, setGroupId] = useState('');

// In the form state reset:
setGroupId('');

// In the submit input object:
input.group_id = groupId || undefined,  // was: group: group || undefined
```

Replace the JSX:

```tsx
<div>
  <label className="label"><span className="label-text font-medium">{t('channels.addDrawer.group')}</span></label>
  <Select
    value={groupId}
    onChange={setGroupId}
    options={[
      { value: '', label: t('channels.addDrawer.noGroup') },
      ...(groups ?? []).map((g) => ({ value: g.id, label: g.name })),
    ]}
  />
</div>
```

- [ ] **Step 2: Update the badge display on ChannelRow**

Around line 448-454 in `Channels.tsx`, update:

```tsx
{channel.group_id && (
  <div className="shrink-0">
    <span className="inline-flex items-center px-2 py-1 rounded bg-info/10 text-info text-xs font-medium">
      {channel.group_name ?? channel.group_id}
    </span>
  </div>
)}
```

- [ ] **Step 3: Update ChannelDetail.tsx edit modal**

In `web/src/pages/ChannelDetail.tsx`, find the group edit input (around line 321-326) and replace with the same `Select` pattern.

- [ ] **Step 4: Update i18n**

In `web/src/i18n/en.json` under `channels.addDrawer`:

```json
"group": "Group",
"noGroup": "None"
```

In `web/src/i18n/zh.json`:

```json
"group": "分组",
"noGroup": "无"
```

- [ ] **Step 5: Run TypeScript check**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 6: Run the frontend test suite**

Run: `source ~/.nvm/nvm.sh && cd web && npm test`
Expected: All tests pass. Fix any tests that reference the old `group` field on channel objects.

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/Channels.tsx web/src/pages/ChannelDetail.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): replace free-form channel group input with Select"
```

---

## Task 12: Final integration testing and release

**Files:**
- Modify: `CHANGELOG.md`
- Modify: all `Cargo.toml` files in the workspace (version bump)

- [ ] **Step 1: Run the full backend test suite**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 2: Run the full frontend test suite**

Run: `source ~/.nvm/nvm.sh && cd web && npm test`
Expected: All tests pass.

- [ ] **Step 3: Manual end-to-end smoke test**

Start the backend (`cargo run`) and frontend (`cd web && npm run dev`), then:
1. As admin, create two groups ("engineering" and "marketing")
2. Create a user "alice" and assign her to "engineering"
3. Create two channels, assign one to "engineering" group, one to "marketing" group, leave one ungrouped
4. Create an API key as alice
5. As alice (via API key), request a model that's available on all three channels
6. Verify the request lands on either the engineering channel or the ungrouped channel (never the marketing one)
7. Move alice to "marketing" and re-test — should now hit marketing or ungrouped
8. Move alice to no group — should hit any of the three

- [ ] **Step 4: Verify migration on production DB copy (if available)**

If there's a production DB dump available locally:
```bash
DATABASE_URL=postgres://... cargo run --bin llm-gateway -- --migrate-only
```
(If the binary doesn't support `--migrate-only`, just start the server and watch logs for migration completion.)

Verify the legacy `channels."group"` column is gone and `group_id` is populated.

- [ ] **Step 5: Bump versions for release**

In all workspace `Cargo.toml` files (workspace root + each crate), bump `version = "1.6.2"` to `version = "1.7.0"`.

In `web/package.json`, bump `version` to `0.16.0`.

- [ ] **Step 6: Update CHANGELOG.md**

Add a new section at the top:

```markdown
## [1.7.0] - 2026-06-24

### Added
- User and channel groups for access control. Admins can create groups and assign users and channels. A user in group X can only access channels in group X (or channels with no group). Users with no group remain unrestricted.
- New `Groups` admin page and `/api/v1/admin/groups` CRUD endpoints.

### Changed
- `channels.group` column refactored from free-form TEXT to `channels.group_id` foreign key to a new `groups` table. Existing channel-group values are migrated automatically.
- `Channel` API responses use `group_id` and `group_name` (replaces `group`).
- `User` API responses include `group_id` and `group_name`.
- Routing now filters candidate channels by the requesting user's group (admin role bypasses).
```

- [ ] **Step 7: Commit the release prep**

```bash
git add Cargo.toml crates/*/Cargo.toml web/package.json CHANGELOG.md
git commit -m "chore: bump to 1.7.0 / 0.16.0 for user-groups feature"
```

- [ ] **Step 8: Create release branch and merge per git flow**

```bash
git checkout develop
git checkout -b release/1.7.0
# (if changes are needed on release branch, do them here)
git checkout main
git merge --no-ff release/1.7.0
git tag v1.7.0
git checkout develop
git merge --no-ff release/1.7.0
git branch -d release/1.7.0
git push origin main develop --tags
git push origin :feature/user-groups  # optional, if it was pushed
```

---

## Self-Review Checklist (run after the plan is complete, before execution)

- [ ] **Spec coverage**: every section of `docs/superpowers/specs/2026-06-24-user-groups-design.md` has at least one task implementing it
- [ ] **Placeholder scan**: no "TBD", "TODO", "implement later", or unspecified steps
- [ ] **Type consistency**: method names (`useGroups`, `createGroup`, `get_user_group_id`, etc.) match across tasks
- [ ] **Migration safety**: idempotent via `ON CONFLICT (name) DO NOTHING`, verification step inside the migration
- [ ] **Backward compatibility**: existing users with no `group_id` and existing channels with no `group_id` both fall through as unrestricted
