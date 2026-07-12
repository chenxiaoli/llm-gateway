# Users → Members Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the `Users` page and its concept; migrate `accounts` from per-user to per-membership; fold the orphaned capabilities (balance, recharge, usage, enable/disable) into the per-org `Members` page.

**Architecture:** Schema migration ships first (`accounts` gains `org_id`, UNIQUE shifts to `(user_id, org_id)`). Storage trait and postgres impl are updated to the new `(user_id, org_id)` account-lookup signature. The `members` handler expands its response shape and PATCH body to absorb balance/enable. The four account-action routes (`balance`/`recharge`/`adjust`/`threshold`) move from `/admin/users/{id}/...` to `/admin/members/{user_id}/...`. The `users` handler file and routes are deleted. Frontend `Members.tsx` absorbs the drawers and columns from `Users.tsx`; `Users.tsx`, `api/users.ts`, and `hooks/useUsers.ts` are deleted; `AdminDashboard.tsx` switches its count + link to `members`.

**Tech Stack:** Rust workspace (Axum, sqlx, thiserror); React 18 + TypeScript + Vite + React Query; vitest + MSW for frontend tests; cargo test for backend; PostgreSQL with sqlx migrations.

**Spec:** `docs/superpowers/specs/2026-07-12-users-to-members-refactor-design.md`

---

## File Structure

### Files to create
- `crates/storage/migrations/postgres/20260712000000_accounts_per_membership.sql` — schema migration
- `crates/api/tests/test_members_balance.rs` — new test file for account actions under new route

### Files to modify — backend
- `crates/storage/src/lib.rs` — trait method changes
- `crates/storage/src/postgres.rs` — implementation changes
- `crates/storage/src/types.rs` — `MemberWithDetails` type, remove `UserWithBalance` if no longer used elsewhere
- `crates/api/src/management/mod.rs` — route table: drop users routes, move account routes under /admin/members/, expand members routes
- `crates/api/src/management/members.rs` — expand `list_members` response; expand `change_member_role` PATCH to accept `enabled`
- `crates/api/src/management/accounts.rs` — account lookup by `(user_id, ctx.org_id)`
- `crates/api/tests/common/mod.rs` — update helpers if signatures change
- `crates/api/tests/test_members.rs` — new test coverage
- `crates/api/tests/test_accounts.rs` — URL updates (if file exists)

### Files to delete — backend
- `crates/api/src/management/users.rs` — handler file
- `crates/api/tests/test_users.rs` — test file (any useful tests migrate to test_members.rs)

### Files to modify — frontend
- `web/src/App.tsx` — route table
- `web/src/components/Layout.tsx` — sidebar items
- `web/src/components/Layout.test.tsx` — assertions
- `web/src/pages/Members.tsx` — expand
- `web/src/pages/Members.test.tsx` — new coverage
- `web/src/pages/AdminDashboard.tsx` — switch from useUsers to useMembers; update link
- `web/src/pages/AccountBalance.tsx` — no code change (route mount changes in App.tsx)
- `web/src/api/members.ts` — add balance/recharge/adjust/threshold functions
- `web/src/api/accounts.ts` — update URLs
- `web/src/hooks/useMembers.ts` — add new hooks
- `web/src/hooks/useAccounts.ts` — no code change (URL change is in api/accounts.ts)
- `web/src/types/index.ts` — add MemberWithDetails type; remove UserResponse if unused
- `web/src/i18n/en.json` — remove sidebar.users
- `web/src/i18n/zh.json` — remove sidebar.users
- `web/src/test/server.ts` — handler updates

### Files to delete — frontend
- `web/src/pages/Users.tsx`
- `web/src/pages/Users.test.tsx` (if exists)
- `web/src/api/users.ts`
- `web/src/hooks/useUsers.ts`

### Other
- `CHANGELOG.md` — entry

---

## Task 1: Schema cleanup — drop orphan accounts index

> **Revision note (2026-07-12):** The original Task 1 was a full per-membership migration (add `org_id`, backfill, constraints). Investigation during implementation revealed that `20260708000000_saas_orgs.sql` **already did this work** — `accounts` is already 1:1 with `(org_id, user_id)`, has `org_id NOT NULL`, a `UNIQUE (org_id, user_id)` constraint, and an FK to `orgs`. The only remaining schema cleanup is dropping the orphaned `idx_accounts_user_id` index. Task 2 below is also largely already done (account methods already take `org_id`); see its revision note.

**Files:**
- Create: `crates/storage/migrations/postgres/20260712000000_drop_orphan_accounts_user_id_index.sql`

- [ ] **Step 1: Write the migration SQL**

Create `crates/storage/migrations/postgres/20260712000000_drop_orphan_accounts_user_id_index.sql`:

```sql
-- idx_accounts_user_id was left behind when 20260708000000_saas_orgs.sql
-- added accounts_org_user_unique (the per-membership UNIQUE constraint).
-- The plain user_id index is now redundant — every account lookup goes
-- through (org_id, user_id) via the unique constraint's btree.
DROP INDEX IF EXISTS idx_accounts_user_id;
```

- [ ] **Step 2: Apply the migration locally and verify**

Apply the SQL directly to the test DB:

```bash
PGPASSWORD=postgres psql -h localhost -U postgres -d postgres \
  -f crates/storage/migrations/postgres/20260712000000_drop_orphan_accounts_user_id_index.sql
```

Then verify the index is gone and the per-membership constraint is intact:

```bash
PGPASSWORD=postgres psql -h localhost -U postgres -d postgres -c "\d accounts"
```

Expected: `idx_accounts_user_id` is no longer listed; `accounts_org_user_unique UNIQUE (org_id, user_id)` remains; `org_id` is `NOT NULL`.

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/postgres/20260712000000_drop_orphan_accounts_user_id_index.sql
git commit -m "chore(storage): drop redundant idx_accounts_user_id (subsumed by UNIQUE (org_id, user_id))"
```

---

## Task 2: Storage trait — account methods (VERIFICATION ONLY)

> **Revision note (2026-07-12):** Investigation during Task 1 implementation revealed that all account-related storage methods already take `org_id` as their first parameter. This task is now a verification step, not a code change. If the verification passes, mark complete and move on. If it fails (some method doesn't take `org_id`), escalate before proceeding — that means the prior SaaS migration left an inconsistency.

- [ ] **Step 1: Verify account methods take `org_id`**

Inspect the storage trait in `crates/storage/src/lib.rs`. Confirm the following methods all take `org_id` as a parameter (typically first):

- `create_account(org_id, account)`
- `get_account(org_id, id)`
- `get_account_by_user_id(org_id, user_id)`
- `update_account(org_id, account)`
- `list_transactions(org_id, account_id, page, page_size)`
- `get_transaction_by_reference(org_id, account_id, reference_id)`
- Any `recharge` / `adjust` / `set_threshold` methods (if they exist as separate methods)

Verify with a grep:

```bash
grep -nE "fn (create_account|get_account|update_account|list_transactions|recharge|adjust|set_threshold)" crates/storage/src/lib.rs
```

Expected: every match includes `org_id` in its parameter list.

- [ ] **Step 2: Verify account creation on membership creation**

Confirm that wherever a new membership row is created (typically `invite_member`, `accept_invitation`, or `register` storage methods), the matching `accounts` row is also inserted. Check `crates/storage/src/postgres.rs`:

```bash
grep -nB2 -A8 "INSERT INTO members" crates/storage/src/postgres.rs | head -40
grep -nB2 -A8 "INSERT INTO accounts" crates/storage/src/postgres.rs | head -40
```

Expected: every code path that INSERTs into `members` also INSERTs into `accounts` with the same `(user_id, org_id)`. (If this is partially true — e.g., `invite_member` does it but `accept_invitation` doesn't — note which paths are missing the account creation. That's a real bug worth fixing in this task; add the missing INSERTs.)

- [ ] **Step 3: If everything passes, no commit needed**

Mark Task 2 complete in the task tracker. No code changes means no commit.

If Step 1 or Step 2 found gaps, fix them with a focused commit:

```bash
git add crates/storage/
git commit -m "fix(storage): account methods/creation aligned with per-membership model"
```

---

## Task 3: Storage — expand list_members response shape

**Files:**
- Modify: `crates/storage/src/types.rs` — add `MemberWithDetails` type
- Modify: `crates/storage/src/lib.rs` — change `list_members` return type
- Modify: `crates/storage/src/postgres.rs` — update SQL SELECT and row mapping

- [ ] **Step 1: Define the new return type**

In `crates/storage/src/types.rs`, add a struct that joins `members`, `users`, and `accounts`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MemberWithDetails {
    pub user_id: String,
    pub org_id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,             // members.role
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub enabled: bool,            // users.enabled
    pub balance: i64,             // accounts.balance (subunits)
    pub threshold: i64,           // accounts.threshold (subunits)
    pub created_at: chrono::DateTime<chrono::Utc>,  // members.created_at
}
```

- [ ] **Step 2: Change the trait signature**

In `crates/storage/src/lib.rs`, find `list_members`. Before:

```rust
async fn list_members(&self, org_id: &str) -> Result<Vec<Member>, Box<dyn std::error::Error + Send + Sync>>;
```

After:

```rust
async fn list_members(&self, org_id: &str) -> Result<Vec<MemberWithDetails>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 3: Update the SQL and row mapping in postgres.rs**

In `crates/storage/src/postgres.rs`, find `list_members` impl. Update the SELECT to JOIN `users` and `accounts`:

```rust
async fn list_members(&self, org_id: &str) -> Result<Vec<MemberWithDetails>, Box<dyn std::error::Error + Send + Sync>> {
    let rows = sqlx::query_as!(
        MemberWithDetails,
        r#"
        SELECT
            m.user_id,
            m.org_id,
            u.username,
            u.email,
            m.role,
            m.group_id,
            g.name AS group_name,
            u.enabled,
            COALESCE(a.balance, 0) AS balance,
            COALESCE(a.threshold, 100000000) AS threshold,
            m.created_at
        FROM members m
        JOIN users u ON u.id = m.user_id
        LEFT JOIN groups g ON g.id = m.group_id
        LEFT JOIN accounts a ON a.user_id = m.user_id AND a.org_id = m.org_id
        WHERE m.org_id = $1
        ORDER BY m.created_at ASC
        "#,
        org_id
    )
    .fetch_all(&self.pool)
    .await?;
    Ok(rows)
}
```

Adjust `query_as!` vs `query` + manual `FromRow` to match the project's pattern (check sibling methods). The key requirement: all 11 columns must be selected with the right types.

- [ ] **Step 4: Update storage tests**

If any test asserts the old `Vec<Member>` shape, update it. Run:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p llm-gateway-storage 2>&1 | tail -10
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storage/
git commit -m "feat(storage): list_members returns MemberWithDetails (balance, enabled, etc)"
```

---

## Task 4: Storage — remove list_users_paginated and delete_user

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Remove trait methods**

In `crates/storage/src/lib.rs`, delete the lines:

```rust
async fn list_users_paginated(&self, org_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<UserWithBalance>, Box<dyn std::error::Error + Send + Sync>>;
async fn delete_user(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

(Also remove `UserWithBalance` from `types.rs` if it has no other callers — confirm with a grep first.)

- [ ] **Step 2: Remove postgres impls**

In `crates/storage/src/postgres.rs`, delete the `list_users_paginated` and `delete_user` method bodies.

- [ ] **Step 3: Verify nothing else references them**

```bash
grep -rn "list_users_paginated\|delete_user" crates/ web/
```

Expected: no matches outside this commit diff.

- [ ] **Step 4: Run cargo check**

```bash
cargo check --workspace 2>&1 | tail -20
```

Expected: clean. (If `crates/api/src/management/users.rs` still calls these, you'll get errors — that's expected and will be fixed when we delete that file in Task 7. To keep this commit buildable, do Tasks 4 and 7 together as one commit if `cargo check` fails.)

- [ ] **Step 5: Commit**

```bash
git add crates/storage/
git commit -m "refactor(storage): remove list_users_paginated and delete_user"
```

---

## Task 5: Backend — expand members handler response and PATCH body

**Files:**
- Modify: `crates/api/src/management/members.rs`
- Modify: `crates/api/tests/test_members.rs`

- [ ] **Step 1: Write the failing test for expanded list_members response**

In `crates/api/tests/test_members.rs`, add a test that hits `GET /api/v1/{slug}/admin/members` and asserts the response items include `balance`, `enabled`, `email`, and `username`:

```rust
#[tokio::test]
async fn list_members_returns_enriched_shape() {
    let (state, slug, _user_id) = common::setup_state_with_member().await;
    let app = common::build_test_app(state);

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/api/v1/{slug}/admin/members"))
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let first = v["items"][0].as_object().unwrap();
    assert!(first.contains_key("username"));
    assert!(first.contains_key("email"));
    assert!(first.contains_key("balance"));
    assert!(first.contains_key("threshold"));
    assert!(first.contains_key("enabled"));
    assert!(first.contains_key("role"));
    assert!(first.contains_key("group_id"));
    assert!(first.contains_key("group_name"));
    assert!(first.contains_key("created_at"));
}
```

(Adapt `common::setup_state_with_member` to whatever the existing test pattern uses.)

- [ ] **Step 2: Run test to verify it fails**

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p llm-gateway-api --test test_members list_members_returns_enriched_shape 2>&1 | tail -10
```

Expected: FAIL with assertion error (missing keys).

- [ ] **Step 3: Update list_members handler**

In `crates/api/src/management/members.rs`, find `list_members`. Change the response shape from the old `Member`/`MemberResponse` to the new `MemberWithDetails`. The handler becomes:

```rust
pub async fn list_members(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<MemberResponse>>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let members = state.storage.list_members(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(members.into_iter().map(MemberResponse::from).collect()))
}
```

Add/update `MemberResponse` (the API DTO) in the same file to match `MemberWithDetails` plus any field renaming (e.g., convert `balance` from subunits to USD using `units_to_usd`):

```rust
#[derive(Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub enabled: bool,
    pub balance: f64,           // USD, converted from subunits
    pub threshold: f64,         // USD
    pub created_at: String,     // RFC3339
}

impl From<llm_gateway_storage::MemberWithDetails> for MemberResponse {
    fn from(m: llm_gateway_storage::MemberWithDetails) -> Self {
        MemberResponse {
            user_id: m.user_id,
            username: m.username,
            email: m.email,
            role: m.role,
            group_id: m.group_id,
            group_name: m.group_name,
            enabled: m.enabled,
            balance: llm_gateway_storage::units_to_usd(m.balance),
            threshold: llm_gateway_storage::units_to_usd(m.threshold),
            created_at: m.created_at.to_rfc3339(),
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p llm-gateway-api --test test_members list_members_returns_enriched_shape 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 5: Write the failing test for expanded PATCH (accept `enabled`)**

Add to `test_members.rs`:

```rust
#[tokio::test]
async fn patch_member_can_toggle_enabled() {
    let (state, slug, user_id) = common::setup_state_with_member().await;
    let app = common::build_test_app(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/{slug}/admin/members/{user_id}"))
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(r#"{"enabled": false}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    // Verify the underlying user row was updated
    let user = sqlx::query_scalar::<_, bool>("SELECT enabled FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_one(&common::test_pool())
        .await
        .unwrap();
    assert!(!user);
}
```

- [ ] **Step 6: Run test to verify it fails**

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p llm-gateway-api --test test_members patch_member_can_toggle_enabled 2>&1 | tail -10
```

Expected: FAIL (handler ignores `enabled` field).

- [ ] **Step 7: Expand change_member_role handler**

In `crates/api/src/management/members.rs`, find `change_member_role` (rename to `update_member` if cleaner). The signature's input body grows from `{ role }` to `{ role?, enabled?, group_id? }`. Handler logic:

```rust
#[derive(Deserialize)]
pub struct UpdateMemberBody {
    pub role: Option<String>,
    pub enabled: Option<bool>,
    pub group_id: Option<Option<String>>,  // null = clear, Some(id) = set
}

pub async fn update_member(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path(user_id): Path<String>,
    Json(body): Json<UpdateMemberBody>,
) -> Result<Json<MemberResponse>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // Update user-row fields
    if let Some(enabled) = body.enabled {
        let mut user = state.storage.get_user(&user_id).await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("User '{user_id}' not found")))?;
        user.enabled = enabled;
        user.updated_at = chrono::Utc::now();
        state.storage.update_user(&user).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Update member-row fields (role, group_id)
    if body.role.is_some() || body.group_id.is_some() {
        let mut member = state.storage.get_member(&user_id, &ctx.org_id).await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound("Member not found".into()))?;
        if let Some(role_str) = body.role {
            if let Some(parsed) = llm_gateway_storage::MemberRole::parse(&role_str) {
                member.role = parsed;
            }
        }
        if let Some(gid_opt) = body.group_id {
            member.group_id = gid_opt;
        }
        state.storage.upsert_member(member).await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Re-fetch the enriched view for the response
    let members = state.storage.list_members(&ctx.org_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let updated = members.into_iter().find(|m| m.user_id == user_id)
        .ok_or_else(|| ApiError::NotFound("Member not found after update".into()))?;
    Ok(Json(MemberResponse::from(updated)))
}
```

Update the route in `crates/api/src/management/mod.rs` if you renamed the handler:

```rust
// Before:
//   "/members/{user_id}" => patch(change_member_role)
// After:
"/members/{user_id}" => patch(members::update_member).delete(members::remove_member),
```

- [ ] **Step 8: Run test to verify it passes**

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p llm-gateway-api --test test_members 2>&1 | tail -10
```

Expected: all member tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/api/src/management/members.rs crates/api/src/management/mod.rs crates/api/tests/test_members.rs
git commit -m "feat(api): expand members handler with enabled/balance/group_name"
```

---

## Task 6: Backend — move account routes under /admin/members/

**Files:**
- Modify: `crates/api/src/management/accounts.rs`
- Modify: `crates/api/src/management/mod.rs`
- Modify: `crates/api/tests/test_accounts.rs` (if exists) or `test_members.rs`

- [ ] **Step 1: Update mod.rs route table**

In `crates/api/src/management/mod.rs`, find the `/admin/users/{id}/balance|recharge|adjust|threshold` route block. Move it under `/admin/members/`:

Before:
```rust
.route(
    "/admin/users/{id}/balance",
    get(accounts::get_balance).post(accounts::recharge),
)
.route("/admin/users/{id}/recharge", post(accounts::recharge))
.route("/admin/users/{id}/adjust", post(accounts::adjust))
.route("/admin/users/{id}/threshold", patch(accounts::set_threshold))
```

After:
```rust
.route(
    "/admin/members/{user_id}/balance",
    get(accounts::get_balance),
)
.route("/admin/members/{user_id}/recharge", post(accounts::recharge))
.route("/admin/members/{user_id}/adjust", post(accounts::adjust))
.route("/admin/members/{user_id}/threshold", patch(accounts::set_threshold))
```

(Use `{user_id}` for path-param name consistency with `/members/{user_id}` above. The axum `Path<String>` extractor picks it up by position, so the name is documentation.)

- [ ] **Step 2: Update account handler signatures**

In `crates/api/src/management/accounts.rs`, every handler that does an account lookup now needs `(user_id, ctx.org_id)` instead of `user_id` alone. For each of `get_balance`, `recharge`, `adjust`, `set_threshold`:

Before:
```rust
pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path(user_id): Path<String>,
) -> Result<Json<BalanceResponse>, ApiError> {
    let account = state.storage.get_account(&user_id).await...
```

After:
```rust
pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path(user_id): Path<String>,
) -> Result<Json<BalanceResponse>, ApiError> {
    let account = state.storage.get_account(&user_id, &ctx.org_id).await...
```

(Same change inside `recharge`, `adjust`, `set_threshold`.)

- [ ] **Step 3: Update or write tests**

If `crates/api/tests/test_accounts.rs` exists, update every URL from `/api/v1/{slug}/admin/users/{id}/...` to `/api/v1/{slug}/admin/members/{user_id}/...`. If it doesn't exist, add a basic test to `test_members.rs` covering one route (e.g., recharge) to verify the new prefix works:

```rust
#[tokio::test]
async fn recharge_member_under_new_route() {
    let (state, slug, user_id) = common::setup_state_with_member().await;
    let app = common::build_test_app(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/v1/{slug}/admin/members/{user_id}/recharge"))
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"type":"credit","amount":10.0,"description":"test"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}
```

- [ ] **Step 4: Run the test**

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p llm-gateway-api --test test_members recharge_member_under_new_route 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/accounts.rs crates/api/src/management/mod.rs crates/api/tests/
git commit -m "refactor(api): move account routes from /admin/users/* to /admin/members/*"
```

---

## Task 7: Backend — delete users handler and routes

**Files:**
- Delete: `crates/api/src/management/users.rs`
- Delete: `crates/api/tests/test_users.rs`
- Modify: `crates/api/src/management/mod.rs`
- Modify: `crates/api/src/management/mod.rs` (module declarations)

- [ ] **Step 1: Remove users routes from mod.rs**

In `crates/api/src/management/mod.rs`, find and delete the lines:

```rust
.route("/admin/users", get(users::list_users))
.route(
    "/admin/users/{id}",
    patch(users::update_user).delete(users::delete_user),
)
```

- [ ] **Step 2: Remove the users module declaration**

In the same file (or wherever the `pub mod users;` declaration lives — likely the top), delete:

```rust
pub mod users;
```

And remove `users::` from any `use` statement that imports from it.

- [ ] **Step 3: Delete the source files**

```bash
git rm crates/api/src/management/users.rs
git rm crates/api/tests/test_users.rs
```

- [ ] **Step 4: Add legacy-gone stubs for old URLs (optional but consistent with existing pattern)**

In `crates/api/src/management/mod.rs`, the existing `legacy_gone` handler returns 410 for removed routes. Add the removed paths to it (find the existing list and append):

```rust
// Inside the legacy_gone router, add:
.route("/admin/users", legacy_gone)
.route("/admin/users/{id}", legacy_gone)
.route("/admin/users/{id}/balance", legacy_gone)
.route("/admin/users/{id}/recharge", legacy_gone)
.route("/admin/users/{id}/adjust", legacy_gone)
.route("/admin/users/{id}/threshold", legacy_gone)
```

This gives a clear error to any stale clients (curl scripts, old CLI versions) hitting the old endpoints, rather than a generic 404.

- [ ] **Step 5: Verify cargo build + cargo test**

```bash
cargo build --workspace 2>&1 | tail -10
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p llm-gateway-api 2>&1 | tail -10
```

Expected: clean build, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(api): remove /admin/users handlers and routes (folded into /members)"
```

---

## Task 8: Frontend — expand api/members.ts and hooks/useMembers.ts

**Files:**
- Modify: `web/src/api/members.ts`
- Modify: `web/src/hooks/useMembers.ts`
- Modify: `web/src/api/accounts.ts`
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Add the new MemberWithDetails type**

In `web/src/types/index.ts`, replace or augment the existing Member type. Add:

```typescript
export interface MemberWithDetails {
  user_id: string;
  username: string;
  email: string | null;
  role: MemberRole;
  group_id: string | null;
  group_name: string | null;
  enabled: boolean;
  balance: number;
  threshold: number;
  created_at: string;
}
```

(Keep the existing `Member` type if it's used elsewhere; otherwise replace it.)

- [ ] **Step 2: Add account-action functions to api/members.ts**

In `web/src/api/members.ts`, add:

```typescript
import { adminApiClient } from './client';  // add to existing imports if not present

// recharge/adjust/threshold operate on the current org's membership for this user
export async function rechargeMember(userId: string, data: { type: string; amount: number; description?: string }) {
  const { data: resp } = await adminApiClient.post(`/members/${userId}/recharge`, data);
  return resp;
}

export async function adjustMember(userId: string, data: { type: 'credit_adjustment' | 'debit_refund'; amount: number; description?: string }) {
  const { data: resp } = await adminApiClient.post(`/members/${userId}/adjust`, data);
  return resp;
}

export async function setMemberThreshold(userId: string, threshold: number) {
  const { data: resp } = await adminApiClient.patch(`/members/${userId}/threshold`, { threshold });
  return resp;
}

export async function getMemberBalance(userId: string, page: number, pageSize: number) {
  const { data } = await adminApiClient.get(`/members/${userId}/balance`, { params: { page, page_size: pageSize } });
  return data;
}
```

Use the org-aware client (`apiClient` with `orgPrefix()`) or `adminApiClient` depending on whether the routes are org-scoped or top-level. They are org-scoped (`/{slug}/admin/members/...`), so use `apiClient` with `orgPrefix()` matching the pattern in the existing file. (Read `api/members.ts` first to see which client the existing code uses.)

- [ ] **Step 3: Update api/accounts.ts URLs**

In `web/src/api/accounts.ts`, replace every occurrence of `/admin/users/` with `/admin/members/`. For example:

Before:
```typescript
const { data } = await apiClient.get(`${orgPrefix()}/admin/users/${userId}/balance`, ...);
```

After:
```typescript
const { data } = await apiClient.get(`${orgPrefix()}/admin/members/${userId}/balance`, ...);
```

(If `accounts.ts` becomes empty after the move to `members.ts`, just delete it and update imports.)

- [ ] **Step 4: Add hooks for the new operations**

In `web/src/hooks/useMembers.ts`, add:

```typescript
export function useRechargeMember() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ userId, data }: { userId: string; data: { type: string; amount: number; description?: string } }) =>
      rechargeMember(userId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      queryClient.invalidateQueries({ queryKey: [slug, 'member-balance'] });
    },
    onError: (err) => toast.error(getErrorMessage(err, i18n.t('toasts.rechargeFailed'))),
  });
}

// Similarly: useAdjustMember, useSetMemberThreshold
```

- [ ] **Step 5: Verify TypeScript compiles**

```bash
source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -10
```

Expected: clean build (or pre-existing errors only — no new errors).

- [ ] **Step 6: Commit**

```bash
git add web/src/api/ web/src/hooks/ web/src/types/
git commit -m "feat(web): add member balance/recharge/adjust/threshold API + hooks"
```

---

## Task 9: Frontend — expand Members.tsx with balance column, status toggle, drawers

**Files:**
- Modify: `web/src/pages/Members.tsx`
- Modify: `web/src/pages/Members.test.tsx` (if exists; otherwise create)

- [ ] **Step 1: Read current Members.tsx and the source Users.tsx**

Read both files in full. The drawers in `Users.tsx` (`UserDrawer` for balance/recharge/adjust/transactions, `UsageDrawer` for per-user usage) need to move to `Members.tsx` with minor adaptations (the data type becomes `MemberWithDetails` instead of `UserResponse`; URL paths change).

- [ ] **Step 2: Move UserDrawer into Members.tsx**

Copy the `DrawerShell`, `UserDrawer`, and `UsageDrawer` components from `web/src/pages/Users.tsx` into `web/src/pages/Members.tsx` (above the default export). Adapt:
- Replace `UserResponse` with `MemberWithDetails` in prop types
- Replace `user.id` references with `member.user_id`
- Replace `user.username` with `member.username`
- Replace `user.enabled` with `member.enabled`
- Replace `user.group_id` / `user.group_name` with `member.group_id` / `member.group_name`
- The role `<Select>` in UserDrawer (admin/user) is removed — role editing stays on the existing row-level `<Select>` in the main table

- [ ] **Step 3: Add columns to the Members table**

In the main table body of `Members.tsx`, add columns for **Status** and **Balance**. The columns become:

| Username | Role | Status | Balance | Created | Actions |

- **Status**: shows `<Badge variant={member.enabled ? 'green' : 'red'}>`, clicking toggles via `updateMember({ enabled: !member.enabled })`
- **Balance**: shows `formatCurrency(member.balance, symbol, 2)`, click opens `UserDrawer`

- [ ] **Step 4: Add the drawer open state**

In `Members()`, add:

```typescript
const [drawerMember, setDrawerMember] = useState<MemberWithDetails | null>(null);
const [usageUserId, setUsageUserId] = useState<string | null>(null);
const rechargeMutation = useRechargeMember();
const adjustMutation = useAdjustMember();
```

Wire up the row's "Detail" button to `setDrawerMember(member)`, and "Usage" button to `setUsageUserId(member.user_id)`.

- [ ] **Step 5: Render the drawers at the bottom of the page**

```tsx
<UserDrawer
  member={drawerMember}
  onClose={() => setDrawerMember(null)}
/>
<UsageDrawer
  userId={usageUserId}
  onClose={() => setUsageUserId(null)}
/>
```

- [ ] **Step 6: Update or write the test**

If `web/src/pages/Members.test.tsx` doesn't exist, create it. Cover at minimum:
- Renders the members table with all columns
- Click on a row's "Detail" button opens the drawer
- Click on the status toggle calls `PATCH /admin/members/:user_id` with `{enabled: false}`
- Recharge form submission calls `POST /admin/members/:user_id/recharge`

Follow the existing test pattern in `web/src/test/render.tsx` and the MSW handlers in `web/src/test/server.ts` (which Task 11 will update).

- [ ] **Step 7: Run the test**

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run src/pages/Members.test.tsx 2>&1 | tail -15
```

Expected: all assertions pass.

- [ ] **Step 8: Commit**

```bash
git add web/src/pages/Members.tsx web/src/pages/Members.test.tsx
git commit -m "feat(web): expand Members page with balance/status/usage (absorbs Users.tsx)"
```

---

## Task 10: Frontend — migrate AdminDashboard.tsx

**Files:**
- Modify: `web/src/pages/AdminDashboard.tsx`
- Modify: `web/src/pages/AdminDashboard.test.tsx` (if exists)

- [ ] **Step 1: Switch the import**

In `web/src/pages/AdminDashboard.tsx`, change:

Before:
```typescript
import { useUsers } from '../hooks/useUsers';
```

After:
```typescript
import { useMembers } from '../hooks/useMembers';
```

- [ ] **Step 2: Switch the hook call**

Line ~83, change:

Before:
```typescript
const { data: users } = useUsers(1, 1);
```

After:
```typescript
const { data: users } = useMembers();
const userCount = users?.length ?? 0;
```

(`useMembers` returns an array, not paginated. If you need the count differently, adjust.)

- [ ] **Step 3: Update the link target**

Line ~158, change:

Before:
```typescript
navigate(slug ? `/${slug}/admin/users` : '/login')
```

After:
```typescript
navigate(slug ? `/${slug}/members` : '/login')
```

- [ ] **Step 4: Update the i18n key if it references users**

If the text uses `t('adminDashboard.usersCount', ...)`, rename to `t('adminDashboard.membersCount', ...)` and update `web/src/i18n/en.json` + `zh.json` accordingly. If you keep the key name, no i18n change needed.

- [ ] **Step 5: Verify build + test**

```bash
source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -5
npm test -- --run src/pages/AdminDashboard.test.tsx 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/AdminDashboard.tsx web/src/pages/AdminDashboard.test.tsx web/src/i18n/
git commit -m "refactor(web): AdminDashboard uses useMembers instead of useUsers"
```

---

## Task 11: Frontend — delete Users.tsx, api/users.ts, hooks/useUsers.ts

**Files:**
- Delete: `web/src/pages/Users.tsx`
- Delete: `web/src/pages/Users.test.tsx` (if exists)
- Delete: `web/src/api/users.ts`
- Delete: `web/src/hooks/useUsers.ts`
- Modify: `web/src/types/index.ts` — remove `UserResponse`, `UpdateUserRequest` if no other importer

- [ ] **Step 1: Verify no remaining importers**

```bash
grep -rn "from.*api/users\|from.*hooks/useUsers\|UserResponse" web/src/
```

Expected: no matches other than the files we're about to delete.

- [ ] **Step 2: Delete the files**

```bash
git rm web/src/pages/Users.tsx
git rm web/src/pages/Users.test.tsx 2>/dev/null || true
git rm web/src/api/users.ts
git rm web/src/hooks/useUsers.ts
```

- [ ] **Step 3: Remove orphaned types**

In `web/src/types/index.ts`, remove `UserResponse` and `UpdateUserRequest` if grep from Step 1 shows they're only used in deleted files. Keep them if anything still imports them.

- [ ] **Step 4: Verify build**

```bash
source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -10
```

Expected: clean (no "Module not found" errors).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(web): delete Users page, api/users, hooks/useUsers"
```

---

## Task 12: Frontend — update Layout.tsx and App.tsx

**Files:**
- Modify: `web/src/components/Layout.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/components/Layout.test.tsx`

- [ ] **Step 1: Remove `users` from adminItems in Layout.tsx**

In `web/src/components/Layout.tsx` (around line 79), delete:

```typescript
{ key: `/${slug}/admin/users`, icon: Users, label: t('sidebar.users') },
```

Also remove the now-unused `Users` import from `lucide-react` if nothing else uses it.

- [ ] **Step 2: Remove `admin/users` routes from App.tsx**

In `web/src/App.tsx`, find and delete:

```tsx
<Route path="admin/users" element={<Users />} />
<Route path="admin/users/:userId/balance" element={<AccountBalance />} />
```

Also delete the now-unused `import Users from './pages/Users';` line.

- [ ] **Step 3: Add the AccountBalance route under /admin/members/**

In App.tsx, add inside the `<Route element={<RequireAdmin />}>` block:

```tsx
<Route path="admin/members/:userId/balance" element={<AccountBalance />} />
```

- [ ] **Step 4: Add a redirect from the old URL**

Add a `<Route>` for the old URL that redirects:

```tsx
{/* Backward-compat: /{slug}/admin/users → /{slug}/members */}
<Route path="/:orgSlug/admin/users" element={<Navigate to="/:orgSlug/members" replace />} />
<Route path="/:orgSlug/admin/users/:userId/balance" element={<AccountBalanceRedirect />} />
```

Where `AccountBalanceRedirect` reads `useParams` and navigates to the new path. (If this is overkill for the redirect, drop the balance URL redirect — it's only used from inside Members drawer anyway.)

- [ ] **Step 5: Update Layout.test.tsx assertions**

In `web/src/components/Layout.test.tsx`, find any assertion that `Users` is visible. Either remove that assertion (the menu no longer exists) or replace with an assertion that `Members` is visible. Run:

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run src/components/Layout.test.tsx 2>&1 | tail -10
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Layout.tsx web/src/components/Layout.test.tsx web/src/App.tsx
git commit -m "refactor(web): remove /admin/users routes; redirect to /members"
```

---

## Task 13: Frontend — i18n, MSW handlers, test/server.ts

**Files:**
- Modify: `web/src/i18n/en.json`
- Modify: `web/src/i18n/zh.json`
- Modify: `web/src/test/server.ts`

- [ ] **Step 1: Remove the sidebar.users i18n key**

In `web/src/i18n/en.json`, delete `"users": "Users"` from the `sidebar` section. Same for `zh.json` (`"users": "用户"`).

- [ ] **Step 2: Update MSW handlers in test/server.ts**

In `web/src/test/server.ts`, find handlers that match `/api/v1/.../admin/users` and `/api/v1/.../admin/users/{id}/(balance|recharge|adjust|threshold)`. Replace each with the new `/admin/members/...` path:

Before:
```typescript
http.get('*/api/v1/test-org/admin/users', () => { ... }),
http.post('*/api/v1/test-org/admin/users/:userId/recharge', () => { ... }),
```

After:
```typescript
http.get('*/api/v1/test-org/admin/members', () => {
  return HttpResponse.json([
    {
      user_id: 'user-1',
      username: 'admin',
      email: 'admin@example.com',
      role: 'owner',
      group_id: null,
      group_name: null,
      enabled: true,
      balance: 100.0,
      threshold: 10.0,
      created_at: '2026-01-01T00:00:00Z',
    },
  ]);
}),
http.post('*/api/v1/test-org/admin/members/:userId/recharge', () => {
  return HttpResponse.json({ ok: true });
}),
// similarly for /balance, /adjust, /threshold
```

Also expand the existing `/admin/members` GET handler to return the new array shape (with balance, enabled, etc.) — tests in `Members.test.tsx` will rely on this.

- [ ] **Step 3: Run the full frontend test suite**

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/ web/src/test/server.ts
git commit -m "refactor(web): drop sidebar.users i18n key; update MSW handlers"
```

---

## Task 14: CHANGELOG + final verification

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add CHANGELOG entry**

In `CHANGELOG.md`, under the appropriate version section (or "Unreleased" if present), add entries under "Changed", "Removed", and "Added" (for the migration). Follow the existing entry format.

Example:

```markdown
## [Unreleased]

### Changed
- `accounts` table migrated from per-user (1:1 with `users`) to per-membership (1:1 with `(user_id, org_id)`). Balance, recharge, threshold, and transactions are now scoped per-org per-user. Multi-org users: the oldest membership inherits the prior balance; manual reconciliation may be required after upgrade.
- Balance management UI moved from the deleted Users page to the Members page (per-org).

### Removed
- `Users` page and the `/api/v1/{slug}/admin/users*` routes. Use `/api/v1/{slug}/admin/members*` instead.

### Added
- `Members` page expanded with balance column, recharge/adjust/threshold actions, per-member usage drawer, and enable/disable toggle.
- Runtime guard on server start: refuses to boot if any `accounts` row has NULL `org_id`.
```

(Adjust to match the actual CHANGELOG format in the repo.)

- [ ] **Step 2: Run full backend test suite**

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test --workspace 2>&1 | tail -20
```

Expected: all tests pass. Pay attention to any test that depended on the old `accounts` shape — they should have been updated in earlier tasks.

- [ ] **Step 3: Run full frontend test suite**

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 4: Run frontend build**

```bash
source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 5: Manual smoke test**

- Boot backend (`cargo run`) and frontend (`npm run dev`).
- Log in as an org admin.
- Verify the sidebar shows `Members` and no `Users`.
- Click Members → verify the table shows balance and status columns.
- Click a member's Detail → verify the recharge/adjust drawer works.
- Toggle a member's enabled status → verify the badge updates.
- Visit `/test-org/admin/users` in the URL bar → verify it redirects to `/test-org/members`.

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): users → members refactor"
```

---

## Verification (whole-plan)

After all tasks land:

1. `DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test --workspace` — all green.
2. `source ~/.nvm/nvm.sh && cd web && npm run build && npm test -- --run` — all green.
3. `grep -rn "list_users_paginated\|delete_user\|api/users\|hooks/useUsers\|admin/users\|sidebar.users\|UserResponse" crates/ web/src/` — only matches in CHANGELOG, migration files, and `legacy_gone` route stubs.
4. Manual smoke test (Step 5 of Task 14).

## Out of Scope

- "Delete user account entirely" (destructive global user-row deletion) — not in this plan; the membership can be removed, the user row persists.
- Per-org currency (still USD-only).
- `users.role` legacy column cleanup (separate task).
- Mobile UX redesign — only the new columns/actions need to render on mobile, no visual redesign.
- Renaming the `users` or `accounts` tables.
