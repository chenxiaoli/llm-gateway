# SaaS Multi-Tenant Orgs — Phase 2, Plan 2.2: Members + Org Settings

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Prerequisite:** Plan 2.1 (URL migration foundation) shipped. Routes are at `/api/v1/{org_slug}/*`, `OrgContext` is injected by `membership_layer`, and `currentOrg` is in the frontend store.

**Goal:** Add the members and org-settings surfaces that turn the URL migration into a usable multi-tenant product. After this plan ships, an org admin can invite other users by username, change their roles, remove them (with the last-owner guard blocking accidental orphaning), rename the org, change its slug, or delete the org entirely (with password confirmation).

**Architecture:** Six new backend endpoints (all under `/api/v1/{org_slug}/...`) plus two frontend pages. The `count_owners` storage method from Phase 1 powers the last-owner guard. Slug rename reuses the same `^[a-z0-9-]{3,64}$` regex and the partial UNIQUE index from Phase 1's migration. No DB migration in this plan — everything needed (the `members` table, the `orgs` table, the `count_owners` query target) was added in Phase 1.

**Tech Stack:** Rust (Axum), sqlx, React + TypeScript + react-hook-form + zod.

**Spec reference:** `docs/superpowers/specs/2026-07-07-saas-multi-tenant-orgs-design.md` — Members endpoints (lines 700-703), Org CRUD (lines 696-698), Permission helpers (lines 720-733), Last-owner guard (lines 880, 918).

---

## File Structure

### Create

**Backend**
- `crates/api/src/management/members.rs` — members CRUD (list, invite, change-role, remove)
- `crates/api/src/management/orgs.rs` (extend) — `PATCH`/`DELETE` org

**Frontend**
- `web/src/pages/Members.tsx` — members table + invite modal
- `web/src/pages/OrgSettings.tsx` — name/slug edit + danger zone

### Modify

**Backend**
- `crates/api/src/management/mod.rs` — register new routes under `/{org_slug}/members` and `/{org_slug}`
- `crates/api/src/error.rs` — add `LastOwner` (400) and `PasswordRequired` (400) variants if not present
- `crates/storage/src/lib.rs` — verify `count_owners`, `update_member_role`, `delete_member` already exist from Phase 1; if any are missing, add them
- `crates/storage/src/postgres.rs` — implement any missing methods from above

**Frontend**
- `web/src/App.tsx` — add `Members` and `OrgSettings` routes under `/:orgSlug/`
- `web/src/components/Layout.tsx` — add `Members` and `OrgSettings` to the sidebar nav
- `web/src/api/members.ts` (new) — API client for members endpoints
- `web/src/api/orgs.ts` (new) — API client for org PATCH/DELETE
- `web/src/hooks/useMembers.ts` (new) — React Query hook
- `web/src/types/index.ts` — `Member` type

---

## Deployment Notes

**Not a breaking change at the URL level.** Plan 2.1 already moved routes to `/{org_slug}/*`; this plan only adds new endpoints under that prefix. Existing clients continue to work.

**Operational note on slug rename.** When an org's slug changes, every URL pointing at the old slug 404s. The frontend's `OrgSwitcher` updates `currentOrg` automatically, so in-app users are fine. External API clients using the old slug in URLs get a 404 from `org_resolve_layer` — they must update their integration. Document this in the CHANGELOG entry.

**No DB migration.** All schema work was done in Phase 1.

---

### Task 1: Backend — `GET /{org_slug}/members` list-members

**Files:**
- Create: `crates/api/src/management/members.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: Write failing test**

`crates/api/tests/test_members.rs`:

```rust
use crate::common::{make_client, make_admin_token, make_member_token};

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn list_members_returns_only_current_org_members(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .get("/api/v1/org-a/members")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    let usernames: Vec<&str> = body.iter().map(|m| m["username"].as_str().unwrap()).collect();
    assert!(usernames.contains(&"alice"));
    assert!(usernames.contains(&"bob"));
    assert!(!usernames.contains(&"carol")); // carol is in org_b
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn list_members_forbidden_for_plain_member(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_member_token("user-3", "org_a"); // role=member, not admin

    let resp = app
        .get("/api/v1/org-a/members")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cargo test -p llm-gateway-api --test test_members -- --nocapture
```

- [ ] **Step 3: Implement list_members**

`crates/api/src/management/members.rs`:

```rust
use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::error::ApiError;
use crate::AppState;
use llm_gateway_org::{OrgContext, access::can_administer};
use llm_gateway_storage::MemberRole;

#[derive(serde::Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub group_id: Option<String>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_members(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<MemberResponse>>, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let memberships = state
        .storage
        .list_members_with_user(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(
        memberships
            .into_iter()
            .map(|m| MemberResponse {
                user_id: m.user_id,
                username: m.username,
                role: format!("{:?}", m.role).to_lowercase(),
                group_id: m.group_id,
                joined_at: m.created_at,
            })
            .collect(),
    ))
}
```

`list_members_with_user` joins `members` to `users` to get usernames. If the Phase 1 storage trait only has `list_members` (returning members without usernames), add a `list_members_with_user` method:

`crates/storage/src/lib.rs`:

```rust
async fn list_members_with_user(&self, org_id: &str) -> Result<Vec<MemberWithUser>, Box<dyn std::error::Error + Send + Sync>>;
```

`crates/storage/src/types.rs`:

```rust
pub struct MemberWithUser {
    pub user_id: String,
    pub username: String,
    pub role: MemberRole,
    pub group_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

Implement the join in `crates/storage/src/postgres.rs`:

```rust
async fn list_members_with_user(&self, org_id: &str) -> Result<Vec<MemberWithUser>, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query_as!(
        MemberWithUser,
        r#"
        SELECT m.user_id, u.username, m.role as "role: MemberRole", m.group_id, m.created_at
        FROM members m
        JOIN users u ON u.id = m.user_id
        WHERE m.org_id = $1
        ORDER BY m.created_at
        "#,
        org_id
    )
    .fetch_all(&self.pool)
    .await
    .map_err(Into::into)
}
```

- [ ] **Step 4: Register route**

`crates/api/src/management/mod.rs` in `org_scoped_routes()`:

```rust
.route("/members", get(members::list_members).post(members::invite_member))
.route("/members/{user_id}", patch(members::change_member_role).delete(members::remove_member))
```

Add the module declaration at the top:

```rust
pub mod members;
```

- [ ] **Step 5: Run tests — expect PASS**

```bash
cargo test -p llm-gateway-api --test test_members -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/members.rs crates/api/src/management/mod.rs crates/storage/src/
git commit -m "feat(api): GET /{org_slug}/members with admin-only access"
```

---

### Task 2: Backend — `POST /{org_slug}/members` invite by username

**Files:**
- Modify: `crates/api/src/management/members.rs`

Phase 2 invite is restricted to existing users. Phase 3 will add invitation tokens for users who don't have accounts yet.

- [ ] **Step 1: Write failing test**

Append to `test_members.rs`:

```rust
#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn invite_member_adds_existing_user(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    // carol exists in users table but is only in org_b
    let resp = app
        .post("/api/v1/org-a/members")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "username": "carol",
            "role": "member"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["username"], "carol");
    assert_eq!(body["role"], "member");
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn invite_member_404_for_unknown_user(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .post("/api/v1/org-a/members")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "username": "ghost",
            "role": "member"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn invite_member_409_if_already_member(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    // bob is already a member of org_a
    let resp = app
        .post("/api/v1/org-a/members")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "username": "bob",
            "role": "member"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cargo test -p llm-gateway-api --test test_members -- --nocapture
```

- [ ] **Step 3: Implement invite_member**

```rust
#[derive(serde::Deserialize)]
pub struct InviteMemberRequest {
    pub username: String,
    pub role: String,
}

pub async fn invite_member(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(req): Json<InviteMemberRequest>,
) -> Result<(StatusCode, Json<MemberResponse>), ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let role = match req.role.as_str() {
        "owner" => MemberRole::Owner,
        "admin" => MemberRole::Admin,
        "member" => MemberRole::Member,
        _ => return Err(ApiError::BadRequest(format!("unknown role: {}", req.role))),
    };

    let user = state
        .storage
        .get_user_by_username(&req.username)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    // Check for existing membership
    if let Some(_existing) = state
        .storage
        .get_member(&user.id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        return Err(ApiError::Conflict);
    }

    state
        .storage
        .upsert_member(Member {
            user_id: user.id.clone(),
            org_id: ctx.org_id.clone(),
            role,
            group_id: None,
            created_by: Some(ctx.user_id.clone()),
            created_at: chrono::Utc::now(),
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(MemberResponse {
            user_id: user.id,
            username: user.username,
            role: format!("{:?}", role).to_lowercase(),
            group_id: None,
            joined_at: chrono::Utc::now(),
        }),
    ))
}
```

`Member`, `MemberRole` imports:

```rust
use llm_gateway_storage::{Member, MemberRole};
```

- [ ] **Step 4: Run tests — expect PASS**

```bash
cargo test -p llm-gateway-api --test test_members -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/members.rs
git commit -m "feat(api): POST /{org_slug}/members invite by username"
```

---

### Task 3: Backend — `PATCH /{org_slug}/members/{user_id}` change role

**Files:**
- Modify: `crates/api/src/management/members.rs`

Includes the first last-owner guard: demoting the only owner to admin/member is rejected with 400.

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn change_role_promotes_member_to_admin(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .patch("/api/v1/org-a/members/user-3") // bob is currently member
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "role": "admin" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<serde_json::Value>().await.unwrap()["role"], "admin");
}

#[tokio::test]
#[sqlx::test(fixtures("single_owner_seed"))]
async fn change_role_rejects_demoting_last_owner(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    // alice is the only owner of org_a
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .patch("/api/v1/org-a/members/user-1")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "role": "admin" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("last owner"));
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Implement change_member_role**

```rust
use axum::extract::Path;

#[derive(serde::Deserialize)]
pub struct ChangeRoleRequest {
    pub role: String,
}

pub async fn change_member_role(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path(user_id): Path<String>,
    Json(req): Json<ChangeRoleRequest>,
) -> Result<Json<MemberResponse>, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let new_role = match req.role.as_str() {
        "owner" => MemberRole::Owner,
        "admin" => MemberRole::Admin,
        "member" => MemberRole::Member,
        _ => return Err(ApiError::BadRequest(format!("unknown role: {}", req.role))),
    };

    let member = state
        .storage
        .get_member(&user_id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    // Last-owner guard
    if member.role == MemberRole::Owner && new_role != MemberRole::Owner {
        let owner_count = state
            .storage
            .count_owners(&ctx.org_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        if owner_count <= 1 {
            return Err(ApiError::LastOwner);
        }
    }

    state
        .storage
        .update_member_role(&user_id, &ctx.org_id, new_role)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let user = state
        .storage
        .get_user(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(MemberResponse {
        user_id: user.id,
        username: user.username,
        role: format!("{:?}", new_role).to_lowercase(),
        group_id: member.group_id,
        joined_at: member.created_at,
    }))
}
```

Add `LastOwner` variant to `ApiError`:

```rust
// crates/api/src/error.rs
#[error("cannot remove or demote the last owner of an org")]
LastOwner,
// maps to 400 BadRequest in IntoResponse impl
```

- [ ] **Step 4: Run tests — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/members.rs crates/api/src/error.rs
git commit -m "feat(api): PATCH /{org_slug}/members/{id} with last-owner guard"
```

---

### Task 4: Backend — `DELETE /{org_slug}/members/{user_id}` remove member

**Files:**
- Modify: `crates/api/src/management/members.rs`

Same last-owner guard applies. Members can remove themselves (so they can leave an org); admins can remove anyone except the last owner.

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn remove_member_succeeds_for_admin(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .delete("/api/v1/org-a/members/user-3")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);
}

#[tokio::test]
#[sqlx::test(fixtures("single_owner_seed"))]
async fn remove_member_rejects_last_owner(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .delete("/api/v1/org-a/members/user-1")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn remove_member_can_self_remove(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_member_token("user-3", "org_a"); // member can leave

    let resp = app
        .delete("/api/v1/org-a/members/user-3")
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Implement remove_member**

```rust
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Members can self-remove; admin+ can remove anyone.
    let is_self = ctx.user_id == user_id;
    if !is_self && !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let member = state
        .storage
        .get_member(&user_id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    if member.role == MemberRole::Owner {
        let owner_count = state
            .storage
            .count_owners(&ctx.org_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        if owner_count <= 1 {
            return Err(ApiError::LastOwner);
        }
    }

    state
        .storage
        .delete_member(&user_id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 4: Run tests — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/members.rs
git commit -m "feat(api): DELETE /{org_slug}/members/{id} with self-leave + last-owner guard"
```

---

### Task 5: Backend — `PATCH /{org_slug}` update org (name/slug)

**Files:**
- Modify: `crates/api/src/management/orgs.rs`

Slug rename is allowed but validated. Slug collision returns 409.

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn update_org_renames_name(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .patch("/api/v1/org-a")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "name": "Alpha Corp" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<serde_json::Value>().await.unwrap()["name"], "Alpha Corp");
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn update_org_changes_slug(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .patch("/api/v1/org-a")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "slug": "alpha" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<serde_json::Value>().await.unwrap()["slug"], "alpha");
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn update_org_rejects_duplicate_slug(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .patch("/api/v1/org-a")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "slug": "org-b" }))  // already taken
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn update_org_forbidden_for_member(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_member_token("user-3", "org_a");

    let resp = app
        .patch("/api/v1/org-a")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "name": "Hacked" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Implement update_org**

`crates/api/src/management/orgs.rs` (extend):

```rust
use regex::Regex;
use std::sync::LazyLock;
use llm_gateway_storage::UpdateOrg;

static SLUG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9-]{3,64}$").expect("valid regex")
});

#[derive(serde::Deserialize, Default)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

pub async fn update_org(
    State(state): State<Arc<AppState>>,
    ctx: llm_gateway_org::OrgContext,
    Json(req): Json<UpdateOrgRequest>,
) -> Result<Json<OrgSummary>, ApiError> {
    if !llm_gateway_org::can_manage_org_settings(&ctx) {
        return Err(ApiError::Forbidden);
    }

    if let Some(ref slug) = req.slug {
        if !SLUG_RE.is_match(slug) {
            return Err(ApiError::BadRequest("slug must match ^[a-z0-9-]{3,64}$".into()));
        }
    }
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(ApiError::BadRequest("name must not be empty".into()));
        }
    }

    let updated = state
        .storage
        .update_org(&ctx.org_id, UpdateOrg { name: req.name, slug: req.slug })
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("unique constraint") || msg.contains("duplicate key") {
                ApiError::Conflict
            } else {
                ApiError::Internal(msg)
            }
        })?;

    Ok(Json(OrgSummary {
        id: updated.id,
        slug: updated.slug,
        name: updated.name,
        role: format!("{:?}", ctx.member_role).to_lowercase(),
    }))
}
```

- [ ] **Step 4: Register the route**

`crates/api/src/management/mod.rs`:

```rust
// In org_scoped_routes()
.route("/", get(orgs::get_org).patch(orgs::update_org).delete(orgs::delete_org))
```

- [ ] **Step 5: Run tests — expect PASS**

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/orgs.rs crates/api/src/management/mod.rs
git commit -m "feat(api): PATCH /{org_slug} update name/slug (admin+)"
```

---

### Task 6: Backend — `DELETE /{org_slug}` delete org (password confirm)

**Files:**
- Modify: `crates/api/src/management/orgs.rs`

Hard delete with cascade (per Phase 1 migration). Password confirmation required; type-slug-to-confirm is a frontend concern, not enforced here.

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn delete_org_requires_owner(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-2", "org_a"); // admin, not owner

    let resp = app
        .delete("/api/v1/org-a")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "password": "admin-pass" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn delete_org_requires_password(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a"); // owner

    let resp = app
        .delete("/api/v1/org-a")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "password": "wrong-pass" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn delete_org_with_correct_password_cascades(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_admin_token("user-1", "org_a");

    let resp = app
        .delete("/api/v1/org-a")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "password": "owner-pass" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);

    // Verify cascade
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE org_id = 'org_a'")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

- [ ] **Step 3: Implement delete_org**

```rust
use llm_gateway_auth::verify_password;

#[derive(serde::Deserialize)]
pub struct DeleteOrgRequest {
    pub password: String,
}

pub async fn delete_org(
    State(state): State<Arc<AppState>>,
    ctx: llm_gateway_org::OrgContext,
    Json(req): Json<DeleteOrgRequest>,
) -> Result<StatusCode, ApiError> {
    if !llm_gateway_org::can_delete_org(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // Verify password against the user's stored hash
    let user = state
        .storage
        .get_user(&ctx.user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    let valid = verify_password(&req.password, &user.password_hash)
        .map_err(|_| ApiError::Unauthorized)?;
    if !valid {
        return Err(ApiError::Unauthorized);
    }

    state
        .storage
        .delete_org(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
```

The exact `verify_password` signature depends on what's in `crates/auth`. If the auth crate uses a different function name or argument shape (e.g., `verify_password(password, hash) -> Result<bool>`), adjust accordingly.

- [ ] **Step 4: Run tests — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/orgs.rs
git commit -m "feat(api): DELETE /{org_slug} requires owner + password"
```

---

### Task 7: Frontend — Members page

**Files:**
- Create: `web/src/api/members.ts`
- Create: `web/src/hooks/useMembers.ts`
- Create: `web/src/pages/Members.tsx`
- Modify: `web/src/App.tsx` (add route)
- Modify: `web/src/components/Layout.tsx` (add nav item)
- Modify: `web/src/types/index.ts` (add Member type)

- [ ] **Step 1: Add `Member` type**

```typescript
// web/src/types/index.ts
export interface Member {
  user_id: string
  username: string
  role: 'owner' | 'admin' | 'member'
  group_id: string | null
  joined_at: string
}
```

- [ ] **Step 2: Write API client**

```typescript
// web/src/api/members.ts
import { api, orgPrefix } from './client'
import type { Member } from '../types'

export async function listMembers(): Promise<Member[]> {
  const { data } = await api.get(`${orgPrefix()}/members`)
  return data
}

export async function inviteMember(req: { username: string; role: Member['role'] }): Promise<Member> {
  const { data } = await api.post(`${orgPrefix()}/members`, req)
  return data
}

export async function changeMemberRole(userId: string, role: Member['role']): Promise<Member> {
  const { data } = await api.patch(`${orgPrefix()}/members/${userId}`, { role })
  return data
}

export async function removeMember(userId: string): Promise<void> {
  await api.delete(`${orgPrefix()}/members/${userId}`)
}
```

- [ ] **Step 3: Write React Query hook**

```typescript
// web/src/hooks/useMembers.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useAuthStore } from '../stores/authStore'
import { listMembers, inviteMember, changeMemberRole, removeMember } from '../api/members'

export function useMembers() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? ''
  return useQuery({
    queryKey: [slug, 'members'],
    queryFn: listMembers,
    enabled: !!slug,
  })
}

export function useInviteMember() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? ''
  const qc = useQueryClient()
  return useMutation({
    mutationFn: inviteMember,
    onSuccess: () => qc.invalidateQueries({ queryKey: [slug, 'members'] }),
  })
}

export function useChangeMemberRole() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? ''
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: 'owner' | 'admin' | 'member' }) =>
      changeMemberRole(userId, role),
    onSuccess: () => qc.invalidateQueries({ queryKey: [slug, 'members'] }),
  })
}

export function useRemoveMember() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? ''
  const qc = useQueryClient()
  return useMutation({
    mutationFn: removeMember,
    onSuccess: () => qc.invalidateQueries({ queryKey: [slug, 'members'] }),
  })
}
```

- [ ] **Step 4: Write Members page**

`web/src/pages/Members.tsx`:

```tsx
import { useState } from 'react'
import { useMembers, useInviteMember, useChangeMemberRole, useRemoveMember } from '../hooks/useMembers'
import { Modal } from '../components/ui/Modal'
import { Button } from '../components/ui/Button'
import { Select } from '../components/ui/Select'
import { Input } from '../components/ui/Input'
import { toast } from 'sonner'

export default function Members() {
  const { data: members, isLoading } = useMembers()
  const invite = useInviteMember()
  const changeRole = useChangeMemberRole()
  const remove = useRemoveMember()

  const [inviteOpen, setInviteOpen] = useState(false)
  const [inviteForm, setInviteForm] = useState({ username: '', role: 'member' as const })

  if (isLoading) return <div>Loading...</div>

  async function handleInvite() {
    try {
      await invite.mutateAsync(inviteForm)
      toast.success(`Invited ${inviteForm.username}`)
      setInviteOpen(false)
      setInviteForm({ username: '', role: 'member' })
    } catch (e: any) {
      toast.error(e.response?.data?.error ?? 'Failed to invite')
    }
  }

  async function handleRoleChange(userId: string, role: 'owner' | 'admin' | 'member') {
    try {
      await changeRole.mutateAsync({ userId, role })
      toast.success('Role updated')
    } catch (e: any) {
      toast.error(e.response?.data?.error ?? 'Failed to change role')
    }
  }

  async function handleRemove(userId: string, username: string) {
    if (!confirm(`Remove ${username} from this org?`)) return
    try {
      await remove.mutateAsync(userId)
      toast.success(`${username} removed`)
    } catch (e: any) {
      toast.error(e.response?.data?.error ?? 'Failed to remove')
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Members</h1>
        <Button onClick={() => setInviteOpen(true)}>Invite member</Button>
      </div>

      <table className="w-full text-sm">
        <thead className="text-left text-zinc-500">
          <tr>
            <th className="py-2">Username</th>
            <th>Role</th>
            <th>Joined</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {members?.map((m) => (
            <tr key={m.user_id} className="border-t border-white/5">
              <td className="py-3">{m.username}</td>
              <td>
                <Select
                  value={m.role}
                  onChange={(e) => handleRoleChange(m.user_id, e.target.value as any)}
                >
                  <option value="member">member</option>
                  <option value="admin">admin</option>
                  <option value="owner">owner</option>
                </Select>
              </td>
              <td>{new Date(m.joined_at).toLocaleDateString()}</td>
              <td className="text-right">
                <button
                  onClick={() => handleRemove(m.user_id, m.username)}
                  className="text-red-400 hover:text-red-300"
                >
                  Remove
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <Modal open={inviteOpen} onClose={() => setInviteOpen(false)} title="Invite member">
        <div className="space-y-3">
          <Input
            placeholder="Username"
            value={inviteForm.username}
            onChange={(e) => setInviteForm({ ...inviteForm, username: e.target.value })}
          />
          <Select
            value={inviteForm.role}
            onChange={(e) => setInviteForm({ ...inviteForm, role: e.target.value as any })}
          >
            <option value="member">member</option>
            <option value="admin">admin</option>
          </Select>
          <Button onClick={handleInvite} disabled={invite.isPending}>
            Invite
          </Button>
        </div>
      </Modal>
    </div>
  )
}
```

- [ ] **Step 5: Register route and nav item**

`web/src/App.tsx`:

```tsx
import Members from './pages/Members'
// Inside the RequireAuth > OrgRouteGuard block:
<Route path="members" element={<Members />} />
```

`web/src/components/Layout.tsx`:

```tsx
<NavLink to={`/${currentOrg?.slug}/members`}>Members</NavLink>
```

(Only show if `currentOrg.role in ['admin', 'owner']` or `user.platform_role === 'platform_admin'`.)

- [ ] **Step 6: Run frontend tests + build**

```bash
source ~/.nvm/nvm.sh && cd web && npm test && npm run build
```

- [ ] **Step 7: Commit**

```bash
git add web/src/
git commit -m "feat(web): Members page with invite + role-change + remove"
```

---

### Task 8: Frontend — Org Settings page

**Files:**
- Create: `web/src/api/orgs.ts`
- Create: `web/src/pages/OrgSettings.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/components/Layout.tsx`

- [ ] **Step 1: Write API client**

```typescript
// web/src/api/orgs.ts
import { api, orgPrefix } from './client'

export async function updateOrg(req: { name?: string; slug?: string }): Promise<void> {
  await api.patch(orgPrefix(), req)
  // On slug change, the caller (page) handles navigation/reload
}

export async function deleteOrg(password: string): Promise<void> {
  await api.delete(orgPrefix(), { data: { password } })
}
```

- [ ] **Step 2: Write OrgSettings page**

`web/src/pages/OrgSettings.tsx`:

```tsx
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'
import { updateOrg, deleteOrg } from '../api/orgs'
import { Button } from '../components/ui/Button'
import { Input } from '../components/ui/Input'
import { toast } from 'sonner'

export default function OrgSettings() {
  const { currentOrg, user, setCurrentOrg, logout } = useAuthStore()
  const navigate = useNavigate()
  const isOwner = currentOrg?.role === 'owner'
  const isPlatformAdmin = user?.platform_role === 'platform_admin'

  const [name, setName] = useState(currentOrg?.name ?? '')
  const [slug, setSlug] = useState(currentOrg?.slug ?? '')
  const [deleteConfirm, setDeleteConfirm] = useState('')
  const [deletePassword, setDeletePassword] = useState('')

  async function handleSave() {
    if (!currentOrg) return
    try {
      await updateOrg({ name, slug })
      toast.success('Org updated')
      // If slug changed, navigate to new path
      if (slug !== currentOrg.slug) {
        await setCurrentOrg({ ...currentOrg, name, slug })
        navigate(`/${slug}/settings`)
      } else {
        await setCurrentOrg({ ...currentOrg, name })
      }
    } catch (e: any) {
      toast.error(e.response?.data?.error ?? 'Failed to update org')
    }
  }

  async function handleDelete() {
    if (!currentOrg) return
    if (deleteConfirm !== currentOrg.slug) {
      toast.error(`Type ${currentOrg.slug} to confirm`)
      return
    }
    try {
      await deleteOrg(deletePassword)
      toast.success('Org deleted')
      logout()
      navigate('/')
    } catch (e: any) {
      toast.error(e.response?.data?.error ?? 'Failed to delete org')
    }
  }

  const canEdit = isOwner || isPlatformAdmin

  return (
    <div className="max-w-2xl space-y-8">
      <h1 className="text-xl font-semibold">Org settings</h1>

      <section className="space-y-3">
        <h2 className="text-sm font-medium text-zinc-400">General</h2>
        <label className="block">
          <span className="text-sm">Name</span>
          <Input value={name} onChange={(e) => setName(e.target.value)} disabled={!canEdit} />
        </label>
        <label className="block">
          <span className="text-sm">Slug</span>
          <Input value={slug} onChange={(e) => setSlug(e.target.value)} disabled={!canEdit} />
          <span className="text-xs text-zinc-500">Lowercase letters, digits, and hyphens. 3-64 chars.</span>
        </label>
        <Button onClick={handleSave} disabled={!canEdit}>Save</Button>
      </section>

      {isOwner && (
        <section className="space-y-3 border border-red-500/30 rounded-md p-4">
          <h2 className="text-sm font-medium text-red-400">Danger zone</h2>
          <p className="text-sm text-zinc-400">
            Deleting this org permanently removes all keys, channels, usage history, and audit logs.
            This cannot be undone.
          </p>
          <label className="block">
            <span className="text-sm">Type the org slug to confirm</span>
            <Input
              value={deleteConfirm}
              onChange={(e) => setDeleteConfirm(e.target.value)}
              placeholder={currentOrg?.slug}
            />
          </label>
          <label className="block">
            <span className="text-sm">Your password</span>
            <Input
              type="password"
              value={deletePassword}
              onChange={(e) => setDeletePassword(e.target.value)}
            />
          </label>
          <Button variant="danger" onClick={handleDelete} disabled={deleteConfirm !== currentOrg?.slug}>
            Delete org
          </Button>
        </section>
      )}
    </div>
  )
}
```

- [ ] **Step 3: Register route + nav**

`web/src/App.tsx`:

```tsx
import OrgSettings from './pages/OrgSettings'
// In the RequireAuth > OrgRouteGuard block:
<Route path="settings" element={<OrgSettings />} />
```

`web/src/components/Layout.tsx`:

```tsx
<NavLink to={`/${currentOrg?.slug}/settings`}>Settings</NavLink>
```

- [ ] **Step 4: Run tests + build**

```bash
npm test && npm run build
```

- [ ] **Step 5: Commit**

```bash
git add web/src/
git commit -m "feat(web): Org Settings page with rename + danger-zone delete"
```

---

### Task 9: End-to-end verification

**Files:** (no file changes)

- [ ] **Step 1: Full backend tests**

```bash
cargo test --workspace
```

- [ ] **Step 2: Full frontend tests + build**

```bash
source ~/.nvm/nvm.sh && cd web && npm test && npm run build
```

- [ ] **Step 3: Manual smoke — members flow**

```bash
cargo run &
cd web && npm run dev
```

In browser:
1. Log in as admin of "default" org → `/default/dashboard`
2. Navigate to `/default/members` → see yourself listed
3. Click "Invite member" → enter a username that exists → submit → toast "Invited"
4. Verify new member appears in the table
5. Change their role via the dropdown → toast "Role updated"
6. Click "Remove" → confirm → toast "removed"
7. Try to demote yourself when you're the only owner → toast "cannot remove or demote the last owner"

- [ ] **Step 4: Manual smoke — org settings flow**

1. Navigate to `/default/settings`
2. Change name → Save → toast "Org updated"; sidebar reflects new name
3. Change slug → Save → URL navigates to `/<new-slug>/settings`
4. Try invalid slug (uppercase, spaces) → 400 error toast
5. Try duplicate slug (if a second org exists) → 409 error toast
6. In danger zone: type wrong slug → Delete button stays disabled
7. Type correct slug + your password → org deleted → logged out

- [ ] **Step 5: Cross-org isolation regression**

1. Switch between two orgs and confirm members list is scoped to the current org
2. After a slug rename, verify old URLs return 404 (not 200 with stale data)

- [ ] **Step 6: Commit any cleanup**

```bash
git status
git log --oneline -15
```

---

## Self-Review Notes

**Spec coverage:**

| Spec deliverable | Task |
|---|---|
| `GET /{org_slug}/members` | Task 1 |
| `POST /{org_slug}/members` invite (by username) | Task 2 |
| `PATCH /{org_slug}/members/{user_id}` change role | Task 3 |
| `DELETE /{org_slug}/members/{user_id}` remove | Task 4 |
| `PATCH /{org_slug}` update name/slug | Task 5 |
| `DELETE /{org_slug}` delete org (owner + password) | Task 6 |
| Members page | Task 7 |
| Org Settings page | Task 8 |
| Last-owner guard | Tasks 3 + 4 |

**Placeholder scan:** none. The `verify_password` import in Task 6 may need adjusting to the actual function signature in `crates/auth` — flagged inline.

**Type consistency:** `Member` type matches between backend (`MemberResponse`) and frontend (`web/src/types/index.ts`). `OrgSummary` from Plan 2.1 is reused.

**Risks worth flagging:**

1. **Last-owner guard is in the handler, not the DB.** A direct SQL UPDATE could violate the invariant. Acceptable for now (no DB-level constraint can express "exactly one owner" cleanly); flag in operations docs that direct DB writes to `members.role` are dangerous.

2. **Slug rename breaks external integrations.** Any client using the old slug gets 404s. CHANGELOG entry must call this out.

3. **`delete_org` cascades to all tenant data** (keys, channels, usage_records, audit_logs, accounts, transactions). Phase 1's migration set `ON DELETE CASCADE` on every FK. This is by design (hard delete), but the UI confirmation ("type slug to confirm" + password) is the only thing standing between a careless admin and data loss. Don't soften it.

4. **Invitations are username-only in Plan 2.2.** A user must already exist. Phase 3 adds invitation tokens for new users via email or in-app notification.

5. **Members page exposes usernames.** No PII leak risk currently (usernames are public-ish), but if Phase 3 adds email-based invitations, the page should mask emails or restrict visibility.
