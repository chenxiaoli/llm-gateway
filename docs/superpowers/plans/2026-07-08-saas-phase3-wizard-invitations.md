# SaaS Multi-Tenant Phase 3 — Wizard-Gated Signup + Invitations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Phase 3 of the SaaS multi-tenant migration — wizard-gated signup (no auto-org), generic single-use magic-link invitations, and an invite-aware `/accept-invite` landing page.

**Architecture:** A new `invitations` table backs a token lifecycle (mint → share → accept → revoke). Signup stops auto-assigning a default org; brand-new users land at `/onboarding` and must create or join an org. JWT is reissued on org-create and on invite-accept. Public preview endpoint renders invite metadata without auth.

**Tech Stack:** Rust (Axum, sqlx, Postgres), React + TypeScript (React Router, Zustand, React Query, MSW, Vitest, Playwright).

**Spec:** [docs/superpowers/specs/2026-07-08-saas-phase3-invitations-design.md](../specs/2026-07-08-saas-phase3-invitations-design.md). Read it before starting.

**Spec refinement discovered during planning:** `auth.rs:729-738` shows Phase 2.2's `create_org` *already* auto-sets `current_org_id` unconditionally — the spec's claim that it "did not auto-switch" was wrong. Phase 3 adds two real deltas: (a) only auto-switch when the caller was in limbo (`current_org_id IS NULL`); (b) reissue JWT and return `AuthResponse` (not `OrgSummary`) so the client receives a fresh token. No frontend callers exist today, so the response-shape change is non-breaking.

**Conventions established by earlier phases (carry forward):**
- All storage trait methods take `org_id: &str` first. Catalog methods take `viewer_org_id`. New methods in this plan follow that.
- All new endpoints live under `/api/v1/{org_slug}/...` for org-scoped, or `/api/v1/...` for global. The middleware chain (`auth_layer → org_resolve_layer → membership_layer`) handles org-scoped; global routes do their own `require_auth`.
- Error type is `ApiError` (`BadRequest`, `Forbidden`, `NotFound`, `Conflict`, `Internal`, `Unauthorized`). Storage errors are `Box<dyn Error>` and string-sniffed for Postgres constraint names when 409 mapping is needed.
- Frontend auth state lives in `useAuthStore` (Zustand). Server state via React Query. React Router for routing.

---

## Task 1: Schema migration — `invitations` table

**Files:**
- Create: `crates/storage/migrations/postgres/20260710000000_invitations.sql`
- Create: `crates/storage/migrations/postgres/20260710000000_invitations.down.sql`

- [ ] **Step 1: Write the up migration**

`crates/storage/migrations/postgres/20260710000000_invitations.sql`:

```sql
-- Phase 3: invitations table for magic-link org invitations.
--
-- One row per minted invitation. `token` is the lookup key (opaque 32-byte
-- random, base64url). `role` excludes 'owner' at the DB level — owner is a
-- self-promotion flow inside an org, not assignable by invitation.
--
-- Lifecycle:
--   mint     → row inserted, accepted_at + revoked_at NULL
--   accept   → accepted_at + accepted_by set (single-transaction with members insert)
--   revoke   → revoked_at set (admin action, irreversible)
--
-- Cleanup: rows are retained indefinitely for audit. A future janitor can
-- prune >1-year-old rows; not in Phase 3.

CREATE TABLE invitations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token           TEXT NOT NULL UNIQUE,
    org_id          UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('member','admin')),
    created_by      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,
    accepted_at     TIMESTAMPTZ,
    accepted_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    revoked_at      TIMESTAMPTZ,
    CONSTRAINT invitations_expires_after_created CHECK (expires_at > created_at)
);

-- Speed up the admin "pending invitations" list. Partial index keeps it small
-- even after the table accumulates accepted/expired history.
CREATE INDEX invitations_org_pending_idx
    ON invitations (org_id)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;

-- Token lookups during accept/preview go through the UNIQUE index on `token`
-- automatically; no separate index needed.
```

- [ ] **Step 2: Write the down migration**

`crates/storage/migrations/postgres/20260710000000_invitations.down.sql`:

```sql
DROP TABLE IF EXISTS invitations;
```

- [ ] **Step 3: Verify migrations apply on a clean database**

Run:
```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test -p llm-gateway-storage -- --nocapture
```

Expected: existing storage tests pass (the new migration is picked up automatically by sqlx's `MIGRATOR`).

- [ ] **Step 4: Verify the table shape with a manual query**

```bash
psql 'postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' -c '\d invitations'
```

Expected output should show all columns from the migration, the UNIQUE constraint on `token`, the CHECK on `role`, and the partial index `invitations_org_pending_idx`.

- [ ] **Step 5: Commit**

```bash
git add crates/storage/migrations/postgres/20260710000000_invitations.sql \
        crates/storage/migrations/postgres/20260710000000_invitations.down.sql
git commit -m "feat(storage): invitations table with role CHECK + pending index"
```

---

## Task 2: Storage types — `Invitation`, request/preview types

**Files:**
- Modify: `crates/storage/src/types.rs` (add types after the `MembershipSummary` block around line 98)

- [ ] **Step 1: Add the new types**

In `crates/storage/src/types.rs`, append after `MembershipSummary` (around line 98):

```rust
// --- Invitations (Phase 3) ---

/// One row of the `invitations` table. Used internally by the storage trait.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Invitation {
    pub id: String,
    pub token: String,
    pub org_id: String,
    pub role: MemberRole,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Request body for `POST /api/v1/{org_slug}/invitations`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateInvitationRequest {
    pub role: String, // "member" | "admin"
}

/// Response for invitation mint/list endpoints. The URL is constructed
/// server-side so the frontend doesn't need to know the public base URL.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InvitationResponse {
    pub id: String,
    pub token: String,
    pub url: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Request body for `POST /api/v1/invitations/accept`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AcceptInvitationRequest {
    pub token: String,
}

/// Response for `GET /api/v1/invitations/preview?token=...`. Public — does
/// NOT include the token itself (the caller already has it) or any user-id
/// data; just enough for the landing page to render.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InvitationPreview {
    pub org_name: String,
    pub org_slug: String,
    pub role: String,
    pub inviter_username: String,
    pub expires_at: DateTime<Utc>,
    pub already_member: bool,
}
```

- [ ] **Step 2: Verify the crate still compiles**

```bash
cargo build -p llm-gateway-storage
```

Expected: clean build, no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(storage): Invitation + request/preview types"
```

---

## Task 3: Storage trait + Postgres impl — invitation lifecycle

**Files:**
- Modify: `crates/storage/src/lib.rs` (add 5 trait methods after the members section)
- Modify: `crates/storage/src/postgres.rs` (add the 5 impls)

- [ ] **Step 1: Write failing tests first**

Add `crates/storage/src/postgres.rs` test module entries (extend the existing `#[cfg(test)] mod tests` block at the bottom of the file):

```rust
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn invitation_lifecycle_round_trip(pool: sqlx::PgPool) {
    let storage = super::PostgresStorage::new(pool);
    // Setup: org + user (the inviter).
    let org = make_test_org(&storage, "acme", "Acme").await;
    let inviter = make_test_user(&storage, "alice").await;
    let now = chrono::Utc::now();

    // Mint.
    let invitation = storage
        .create_invitation(
            &org.id,
            &llm_gateway_storage::MemberRole::Admin,
            &inviter.id,
            now + chrono::Duration::days(7),
        )
        .await
        .expect("mint");
    assert!(invitation.accepted_at.is_none());
    assert!(invitation.revoked_at.is_none());

    // Lookup by token.
    let fetched = storage
        .get_invitation_by_token(&invitation.token)
        .await
        .expect("get_by_token")
        .expect("invitation present");
    assert_eq!(fetched.id, invitation.id);

    // List pending for the org (should contain our invitation).
    let pending = storage
        .list_invitations_for_org(&org.id)
        .await
        .expect("list");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, invitation.id);

    // Revoke.
    storage
        .revoke_invitation(&org.id, &invitation.id)
        .await
        .expect("revoke");
    let revoked = storage
        .get_invitation_by_token(&invitation.token)
        .await
        .expect("get")
        .expect("present");
    assert!(revoked.revoked_at.is_some());
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn invitation_accept_creates_membership_and_marks_consumed(pool: sqlx::PgPool) {
    let storage = super::PostgresStorage::new(pool);
    let org = make_test_org(&storage, "acme", "Acme").await;
    let inviter = make_test_user(&storage, "alice").await;
    let invitee = make_test_user(&storage, "bob").await;
    let now = chrono::Utc::now();

    let invitation = storage
        .create_invitation(&org.id, &llm_gateway_storage::MemberRole::Member, &inviter.id, now + chrono::Duration::days(7))
        .await
        .expect("mint");

    let member = storage
        .accept_invitation(&invitation.token, &invitee.id)
        .await
        .expect("accept")
        .expect("invitation was consumable");
    assert_eq!(member.user_id, invitee.id);
    assert_eq!(member.org_id, org.id);
    assert_eq!(member.role, llm_gateway_storage::MemberRole::Member);

    // Second accept returns None (already consumed).
    let second = storage
        .accept_invitation(&invitation.token, &invitee.id)
        .await
        .expect("no db error");
    assert!(second.is_none(), "second accept should be no-op");
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn invitation_token_entropy_is_unique(pool: sqlx::PgPool) {
    // Sanity: 1000 mints produce 1000 distinct tokens. The actual entropy
    // guarantee is 256 bits — this test catches gross bugs in the generator
    // (e.g., a constant or a fixed-prefix mistake).
    let storage = super::PostgresStorage::new(pool);
    let org = make_test_org(&storage, "acme", "Acme").await;
    let inviter = make_test_user(&storage, "alice").await;
    let now = chrono::Utc::now();

    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        let inv = storage
            .create_invitation(&org.id, &llm_gateway_storage::MemberRole::Member, &inviter.id, now + chrono::Duration::days(7))
            .await
            .expect("mint");
        assert!(seen.insert(inv.token), "duplicate token generated");
    }
}
```

The helpers `make_test_org` and `make_test_user` may already exist in the test module; if not, add them as small fns that insert a row directly via `sqlx::query` (mirror existing patterns in the file).

- [ ] **Step 2: Run the tests to verify they fail**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test -p llm-gateway-storage -- --nocapture invitation_
```

Expected: compile errors (the new methods don't exist yet).

- [ ] **Step 3: Add trait method signatures**

In `crates/storage/src/lib.rs`, after the members section (search for `upsert_member` and add after the next blank line):

```rust
// --- Invitations (Phase 3) ---

/// Mint a new invitation token. The storage layer generates the token and
/// returns the inserted row. Expiry is provided by the caller.
async fn create_invitation(
    &self,
    org_id: &str,
    role: &MemberRole,
    created_by: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<Invitation, Box<dyn std::error::Error + Send + Sync>>;

/// Fetch by token. Returns None if no row matches.
async fn get_invitation_by_token(
    &self,
    token: &str,
) -> Result<Option<Invitation>, Box<dyn std::error::Error + Send + Sync>>;

/// List all invitations for an org (both pending and recently-accepted;
/// the handler decides what to surface).
async fn list_invitations_for_org(
    &self,
    org_id: &str,
) -> Result<Vec<Invitation>, Box<dyn std::error::Error + Send + Sync>>;

/// Mark an invitation revoked. No-op if already revoked or not found.
/// `org_id` is required so an admin in org A cannot revoke org B's invitations
/// via this method.
async fn revoke_invitation(
    &self,
    org_id: &str,
    invitation_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Accept an invitation in a single transaction. Validates token +
/// not-yet-accepted + not-revoked + not-expired, inserts a `members` row,
/// and sets `accepted_at` + `accepted_by`. Returns the new Member on success,
/// or None if the invitation was not consumable (expired/revoked/already-used).
///
/// Concurrent calls for the same token serialize via SELECT FOR UPDATE;
/// exactly one succeeds and the others get None.
async fn accept_invitation(
    &self,
    token: &str,
    accepting_user_id: &str,
) -> Result<Option<Member>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 4: Implement in `postgres.rs`**

In `crates/storage/src/postgres.rs`, add the impls (place near the members impl block):

```rust
use rand::{RngCore, rngs::OsRng};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

fn generate_invitation_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

async fn create_invitation(
    &self,
    org_id: &str,
    role: &MemberRole,
    created_by: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<llm_gateway_storage::Invitation, Box<dyn std::error::Error + Send + Sync>> {
    let id = uuid::Uuid::new_v4().to_string();
    let token = generate_invitation_token();
    let now = chrono::Utc::now();

    let row = sqlx::query!(
        r#"
        INSERT INTO invitations (id, token, org_id, role, created_by, created_at, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, token, org_id, role, created_by, created_at, expires_at, accepted_at, accepted_by, revoked_at
        "#,
        id,
        token,
        uuid::Uuid::parse_str(org_id).map_err(|e| format!("bad org_id: {e}"))?,
        role.as_str(),
        uuid::Uuid::parse_str(created_by).map_err(|e| format!("bad created_by: {e}"))?,
        now,
        expires_at,
    )
    .fetch_one(&self.pool)
    .await?;

    Ok(row_to_invitation(row)?)
}

async fn get_invitation_by_token(
    &self,
    token: &str,
) -> Result<Option<llm_gateway_storage::Invitation>, Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query!(
        r#"
        SELECT id, token, org_id, role, created_by, created_at, expires_at, accepted_at, accepted_by, revoked_at
        FROM invitations WHERE token = $1
        "#,
        token
    )
    .fetch_optional(&self.pool)
    .await?;

    row.map(row_to_invitation).transpose()
}
```

Continue with `list_invitations_for_org`, `revoke_invitation`, `accept_invitation`. The accept method is the most involved — wrap in a transaction:

```rust
async fn accept_invitation(
    &self,
    token: &str,
    accepting_user_id: &str,
) -> Result<Option<llm_gateway_storage::Member>, Box<dyn std::error::Error + Send + Sync>> {
    let mut tx = self.pool.begin().await?;

    // SELECT ... FOR UPDATE locks the row so concurrent accepts serialize.
    let row = sqlx::query!(
        r#"
        SELECT id, org_id, role, expires_at, accepted_at, revoked_at
        FROM invitations
        WHERE token = $1
        FOR UPDATE
        "#,
        token
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(inv) = row else {
        tx.rollback().await?;
        return Ok(None);
    };

    let now = chrono::Utc::now();
    let already_consumed = inv.accepted_at.is_some() || inv.revoked_at.is_some();
    let expired = inv.expires_at < now;
    if already_consumed || expired {
        tx.rollback().await?;
        return Ok(None);
    }

    let role = llm_gateway_storage::MemberRole::parse(&inv.role)
        .ok_or_else(|| format!("invalid role in invitations row: {}", inv.role))?;

    // Insert membership.
    let member_id = uuid::Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO members (user_id, org_id, role, created_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, org_id) DO UPDATE SET role = EXCLUDED.role
        "#,
        uuid::Uuid::parse_str(accepting_user_id).map_err(|e| format!("bad user id: {e}"))?,
        inv.org_id,
        role.as_str(),
        now,
    )
    .execute(&mut *tx)
    .await?;
    let _ = member_id; // members table uses (user_id, org_id) as the key

    // Mark invitation consumed.
    sqlx::query!(
        r#"
        UPDATE invitations
        SET accepted_at = $2, accepted_by = $3
        WHERE id = $1
        "#,
        inv.id,
        now,
        uuid::Uuid::parse_str(accepting_user_id).map_err(|e| format!("bad user id: {e}"))?,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Some(llm_gateway_storage::Member {
        user_id: accepting_user_id.to_string(),
        org_id: inv.org_id.to_string(),
        role,
        group_id: None,
        created_by: Some(accepting_user_id.to_string()),
        created_at: now,
    }))
}
```

Add `row_to_invitation` helper near other row-to-type helpers in the same file:

```rust
fn row_to_invitation(
    row: sqlx::postgres::PgRow, // actual type returned by query! — adjust to the specific struct sqlx macros generate
) -> Result<llm_gateway_storage::Invitation, Box<dyn std::error::Error + Send + Sync>> {
    // Hand-roll this if sqlx::query! returns an anonymous struct; the field
    // names match the SELECT columns.
    let role = llm_gateway_storage::MemberRole::parse(&row.role)
        .ok_or_else(|| format!("invalid role in DB: {}", row.role))?;
    Ok(llm_gateway_storage::Invitation {
        id: row.id.to_string(),
        token: row.token,
        org_id: row.org_id.to_string(),
        role,
        created_by: row.created_by.to_string(),
        created_at: row.created_at,
        expires_at: row.expires_at,
        accepted_at: row.accepted_at,
        accepted_by: row.accepted_by.map(|u| u.to_string()),
        revoked_at: row.revoked_at,
    })
}
```

(The exact field-access shape depends on what sqlx::query! emits; if the type-inference dance is awkward, fall back to `sqlx::query_as!` with a derived row struct. The implementer should pick whichever matches the file's existing pattern — most handlers in this file use `query!` with a conversion function.)

- [ ] **Step 5: Run the tests to verify they pass**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test -p llm-gateway-storage -- --nocapture invitation_
```

Expected: 3 passing tests.

- [ ] **Step 6: Run the full storage test suite to confirm no regressions**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test -p llm-gateway-storage
```

Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/postgres.rs
git commit -m "feat(storage): invitation lifecycle (create/get/list/revoke/accept)"
```

---

## Task 4: Backend — mint / list / revoke invitation endpoints

**Files:**
- Create: `crates/api/src/management/invitations.rs`
- Modify: `crates/api/src/management/mod.rs` (add module + wire routes)

- [ ] **Step 1: Write failing tests**

Create `crates/api/src/management/invitations.rs` with a `#[cfg(test)] mod tests` block at the bottom. The integration tests use the existing `axum_test` or `tokio` + `reqwest` pattern from `members.rs` — mirror what Phase 2.2 did.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*; // existing test helpers

    #[tokio::test]
    async fn admin_mints_invitation_member_cannot() {
        // Setup: org, admin user (owner), regular member.
        // POST /api/v1/{slug}/invitations as admin → 201 with token + url.
        // POST as member → 403.
    }

    #[tokio::test]
    async fn owner_role_rejected_at_mint() {
        // POST with role='owner' → 400 (bad request).
    }

    #[tokio::test]
    async fn list_returns_pending_and_recently_accepted() {
        // Mint 2, accept 1, mint 1 more. GET should return 3 (the accepted
        // one within the 30-day window + 2 pending).
    }

    #[tokio::test]
    async fn revoke_sets_revoked_at() {
        // Mint → DELETE → GET shows revoked_at set.
    }

    #[tokio::test]
    async fn admin_in_other_org_cannot_list_or_revoke() {
        // Admin in Org B hitting Org A's endpoints → 403 (membership_layer).
    }
}
```

(Fill in the bodies using the existing helpers in `crates/api/src/test_utils.rs`. If `test_utils` doesn't exist, look at how `members.rs` tests bootstrap — there's a setup helper somewhere.)

- [ ] **Step 2: Implement the handlers**

`crates/api/src/management/invitations.rs`:

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_org::{can_administer, OrgContext};
use llm_gateway_storage::{InvitationResponse, MemberRole};

use crate::error::ApiError;
use crate::AppState;

const INVITATION_TTL_DAYS: i64 = 7;

#[derive(Debug, Deserialize)]
pub struct CreateInvitationBody {
    pub role: String,
}

fn parse_invitation_role(s: &str) -> Result<MemberRole, ApiError> {
    match s {
        "member" => Ok(MemberRole::Member),
        "admin" => Ok(MemberRole::Admin),
        // 'owner' is explicitly forbidden via invitation.
        "owner" => Err(ApiError::BadRequest(
            "owner role cannot be assigned via invitation".into(),
        )),
        other => Err(ApiError::BadRequest(format!(
            "unknown role '{other}'; expected one of: member, admin"
        ))),
    }
}

/// Build the URL the admin will share. The base comes from the request's
/// Host header or a configurable public base URL; for now we use the
/// configurable base and fall back to relative.
fn build_invite_url(state: &AppState, token: &str) -> String {
    let base = state.public_base_url.trim_end_matches('/');
    format!("{base}/accept-invite?token={token}")
}

pub async fn create_invitation(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(body): Json<CreateInvitationBody>,
) -> Result<(StatusCode, Json<InvitationResponse>), ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let role = parse_invitation_role(&body.role)?;
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::days(INVITATION_TTL_DAYS);

    let invitation = state
        .storage
        .create_invitation(&ctx.org_id, &role, &ctx.user_id, expires_at)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(InvitationResponse {
            id: invitation.id,
            token: invitation.token.clone(),
            url: build_invite_url(&state, &invitation.token),
            role: invitation.role.as_str().to_string(),
            created_at: invitation.created_at,
            expires_at: invitation.expires_at,
            accepted_at: None,
            accepted_by: None,
            revoked_at: None,
        }),
    ))
}

pub async fn list_invitations(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<InvitationResponse>>, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let invitations = state
        .storage
        .list_invitations_for_org(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Filter: pending OR (accepted within last 30 days).
    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let mut out = Vec::with_capacity(invitations.len());
    for inv in invitations {
        let include = match inv.accepted_at {
            Some(t) => t > cutoff,
            None => inv.revoked_at.is_none(), // hide revoked
        };
        if !include {
            continue;
        }
        let username = match inv.accepted_by {
            Some(uid) => state
                .storage
                .get_user(&uid)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .map(|u| u.username)
                .unwrap_or_default(),
            None => String::new(),
        };
        out.push(InvitationResponse {
            id: inv.id,
            token: inv.token.clone(),
            url: build_invite_url(&state, &inv.token),
            role: inv.role.as_str().to_string(),
            created_at: inv.created_at,
            expires_at: inv.expires_at,
            accepted_at: inv.accepted_at,
            accepted_by: inv.accepted_by.map(|_| username.clone()).or(None),
            revoked_at: inv.revoked_at,
        });
    }
    // Re-map accepted_by: we populated `username` into the wrong field above
    // for clarity. Fix the mapping: accepted_by should be the username string
    // (frontend doesn't need raw user IDs here).
    Ok(Json(out))
}

pub async fn revoke_invitation(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, invitation_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }
    state
        .storage
        .revoke_invitation(&ctx.org_id, &invitation_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
```

Note: the `accepted_by` username mapping in `list_invitations` is awkward. Clean it up before committing — the intent is "frontend sees the username of who accepted, not a raw UUID." Resolve the username once via `get_user`, then store it as a string field. The `InvitationResponse.accepted_by` is already `Option<String>` per Task 2's type definition — use the username string.

- [ ] **Step 3: Add `public_base_url` to `AppState`**

In `crates/api/src/lib.rs`, add to `AppState`:

```rust
/// Public-facing base URL for constructing invitation links etc.
/// Configurable via config.toml; defaults to "http://localhost:5173".
pub public_base_url: String,
```

Initialize from config in the gateway bootstrap (find where other `AppState` fields are populated — likely in `crates/gateway/src/main.rs` or similar). Default to `http://localhost:5173` if unset in config.

Add to `config.toml.example` (if one exists) or document in CHANGELOG.

- [ ] **Step 4: Register module and routes**

In `crates/api/src/management/mod.rs`:

```rust
// line 11 (after `pub mod members;`):
pub mod invitations;
```

In `fn org_scoped_routes()`, after the members routes:

```rust
// Invitations (admin-only) — list/mint/revoke.
.route(
    "/invitations",
    get(invitations::list_invitations).post(invitations::create_invitation),
)
.route("/invitations/{id}", delete(invitations::revoke_invitation))
```

Add `delete` to the existing `use axum::routing::{...}` line if not present.

- [ ] **Step 5: Run the tests**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test -p llm-gateway-api -- --nocapture invitations
```

Expected: all invitation tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/invitations.rs \
        crates/api/src/management/mod.rs \
        crates/api/src/lib.rs \
        crates/gateway/  # wherever AppState is built
git commit -m "feat(api): invitation mint/list/revoke endpoints"
```

---

## Task 5: Backend — public preview + accept endpoints

**Files:**
- Modify: `crates/api/src/management/invitations.rs` (add `preview_invitation`, `accept_invitation`)
- Modify: `crates/api/src/management/mod.rs` (wire global routes — these are NOT under `/{org_slug}/`)

- [ ] **Step 1: Write failing tests**

Append to `crates/api/src/management/invitations.rs` `#[cfg(test)] mod tests`:

```rust
#[tokio::test]
async fn preview_returns_metadata_for_valid_pending_token() {
    // Mint → GET /api/v1/invitations/preview?token=... → 200 with org_name/slug/role/inviter/expires_at.
}

#[tokio::test]
async fn preview_returns_410_for_expired() {
    // Mint with expiry in the past → 410.
}

#[tokio::test]
async fn preview_returns_410_for_revoked() {
    // Mint → DELETE → preview → 410.
}

#[tokio::test]
async fn preview_returns_410_for_already_accepted() {
    // Mint → accept via second user → preview → 410.
}

#[tokio::test]
async fn preview_returns_identical_410_body_for_invalid_token() {
    // GET ?token=nonexistent → 410 with same body shape as expired/revoked.
}

#[tokio::test]
async fn preview_sets_already_member_flag_for_logged_in_member() {
    // User is already a member of inviting org → preview returns 200 with already_member=true.
}

#[tokio::test]
async fn accept_creates_membership_and_reissues_jwt() {
    // Mint → accept as a different existing user → 200 with AuthResponse (fresh token).
}

#[tokio::test]
async fn accept_returns_410_for_consumed_token() {
    // Mint → accept → second accept → 410.
}

#[tokio::test]
async fn accept_concurrent_only_one_wins() {
    // Mint → spawn 2 concurrent accepts (different users) → exactly one 200, one 410/409.
}
```

- [ ] **Step 2: Implement preview**

In `crates/api/src/management/invitations.rs`:

```rust
use axum::extract::Query;
use llm_gateway_storage::InvitationPreview;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub token: String,
}

/// Error body returned for ANY non-consumable preview. Identical for invalid /
/// expired / revoked / already-accepted to prevent enumeration.
#[derive(Debug, Serialize)]
pub struct InvitationGone {
    pub reason: &'static str, // single static string; do not vary per cause
}

const INVITATION_GONE_REASON: &str = "This invitation is no longer valid.";

pub async fn preview_invitation(
    State(state): State<Arc<AppState>>,
    Query(q): Query<PreviewQuery>,
) -> Result<Json<InvitationPreview>, ApiError> {
    let Some(inv) = state
        .storage
        .get_invitation_by_token(&q.token)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        return Err(ApiError::Gone(INVITATION_GONE_REASON.to_string()));
    };

    let now = chrono::Utc::now();
    if inv.accepted_at.is_some() || inv.revoked_at.is_some() || inv.expires_at < now {
        return Err(ApiError::Gone(INVITATION_GONE_REASON.to_string()));
    }

    let org = state
        .storage
        .get_org(&inv.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Internal("invitation references missing org".into()))?;
    let inviter = state
        .storage
        .get_user(&inv.created_by)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(|u| u.username)
        .unwrap_or_default();

    Ok(Json(InvitationPreview {
        org_name: org.name,
        org_slug: org.slug,
        role: inv.role.as_str().to_string(),
        inviter_username: inviter,
        expires_at: inv.expires_at,
        already_member: false, // populated below for the authed path
    }))
}
```

Note: the `already_member` flag needs auth context. The simplest design: mount TWO copies of the preview route — one global (unauth) returning `already_member: false`, one under `:orgSlug` for the authed path. But that's awkward. Simpler: have the frontend compute `already_member` from its own user state after fetching the preview. Drop the field from the unauth preview and let the frontend cross-check.

**Refinement:** drop `already_member` from `InvitationPreview`. The frontend already knows the user's memberships; it can check `org_slug in userOrgs`. Update the type in Task 2's `InvitationPreview` to remove that field.

- [ ] **Step 3: Implement accept**

```rust
use axum::http::HeaderMap;
use crate::auth::{require_auth, create_jwt, AuthResponse, UserInfo};
use llm_gateway_storage::AcceptInvitationRequest;

pub async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AcceptInvitationRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let Some(member) = state
        .storage
        .accept_invitation(&body.token, &claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        return Err(ApiError::Gone(INVITATION_GONE_REASON.to_string()));
    };

    // Reload user to pick up the new membership in their org list + persist
    // current_org_id switch.
    let mut user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    user.current_org_id = Some(member.org_id.clone());
    user.updated_at = chrono::Utc::now();
    user = state
        .storage
        .update_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(&user.id, &member.org_id, platform_role_str, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let memberships = state
        .storage
        .list_orgs_for_user(&user.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let orgs: Vec<_> = memberships.into_iter().map_into().collect();
    let current_org_summary = orgs
        .iter()
        .find(|o| o.id == member.org_id)
        .cloned()
        .ok_or_else(|| ApiError::Internal("just-joined org not in membership list".into()))?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: user.refresh_token.clone().unwrap_or_default(),
        user: UserInfo::from(&user),
        current_org: current_org_summary,
        orgs,
    }))
}
```

- [ ] **Step 4: Add `ApiError::Gone` variant if missing**

In `crates/api/src/error.rs`, check whether `Gone` exists. If not, add:

```rust
Gone(String),
```

with status code 410 in the `IntoResponse` impl.

- [ ] **Step 5: Wire global routes**

In `crates/api/src/management/mod.rs`'s `management_router`, in the global section:

```rust
// Public invitation preview + accept (accept requires auth, but is global
// because the token itself identifies the org — no URL slug).
.route("/api/v1/invitations/preview", get(invitations::preview_invitation))
.route("/api/v1/invitations/accept", post(invitations::accept_invitation))
```

- [ ] **Step 6: Run tests**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test -p llm-gateway-api -- --nocapture invitations::tests
```

Expected: all 12 invitation tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/api/
git commit -m "feat(api): public preview + transactional accept endpoints"
```

---

## Task 6: Backend — modify `register` + `create_org` + add `me/onboarding`

**Files:**
- Modify: `crates/api/src/auth.rs` (register: stop auto-org; create_org: conditional switch + JWT reissue)
- Modify: `crates/api/src/management/mod.rs` (add `me/onboarding` route)

- [ ] **Step 1: Write failing tests**

In `crates/api/src/auth.rs` test module (or wherever auth tests live):

```rust
#[tokio::test]
async fn register_returns_jwt_with_null_current_org() {
    // POST /auth/register → 200. Response has current_org: None, orgs: [].
    // JWT decoded shows current_org_id claim as None.
}

#[tokio::test]
async fn create_org_called_by_limbo_user_switches_current_org_and_reissues_jwt() {
    // Register (limbo) → POST /api/v1/orgs → 200 with AuthResponse, current_org set, fresh JWT.
}

#[tokio::test]
async fn create_org_called_by_established_user_does_not_switch_current_org() {
    // User already has Org A as current. POST /api/v1/orgs with slug "b" → 200
    // with AuthResponse, but current_org is STILL A (the response.orgs contains
    // both A and B; current_org = A).
}

#[tokio::test]
async fn me_onboarding_returns_true_for_limbo_user() {
    // Register → GET /api/v1/me/onboarding → { needs_onboarding: true }.
}

#[tokio::test]
async fn me_onboarding_returns_false_once_user_has_org() {
    // Register → create_org → GET /api/v1/me/onboarding → { needs_onboarding: false }.
}
```

- [ ] **Step 2: Modify `register`**

In `crates/api/src/auth.rs`, replace the body of `register` from the "TODO(Phase 3)" comment (line 288) through the `current_membership` call (~line 339). New version:

```rust
// Phase 3: brand-new users land in limbo — no auto-org-membership, no
// current_org_id, no account. They complete the onboarding wizard to
// create or join an org.
//
// First-user platform_admin auto-grant is preserved (cold-start deploys
// still need a way to bootstrap a platform_admin without an existing org).
let user = state
    .storage
    .update_user(&user)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

let orgs: Vec<OrgSummary> = Vec::new(); // limbo user has no memberships

let token = create_jwt(&user.id, "", platform_role_str, &state.jwt_secret)
    .map_err(|e| ApiError::Internal(e.to_string()))?;
// Note: `create_jwt` signature may need adjustment to accept Option<org_id>.
// If it requires a non-empty string, change it to take Option<&str> and
// emit a JWT without the current_org_id claim (or with claim = null).

Ok(Json(AuthResponse {
    token,
    refresh_token: user.refresh_token.clone().unwrap_or_default(),
    user: UserInfo::from(&user),
    current_org: None,
    orgs,
}))
```

Adjust `create_jwt` signature if needed to accept `Option<&str>` for the org_id argument. This is a mechanical change touching all call sites (4-5 of them).

- [ ] **Step 3: Modify `create_org`**

Replace the unconditional `current_org_id = Some(org.id)` block (lines 729-738) with:

```rust
// Auto-switch current_org ONLY when the caller was in limbo. An
// established user creating an additional org keeps their current_org so
// they don't get yanked out of their working context.
let was_limbo = user.current_org_id.is_none();
let mut updated = user.clone();
if was_limbo {
    updated.current_org_id = Some(org.id.clone());
}
updated.updated_at = now;
let updated_user = state
    .storage
    .update_user(&updated)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

// Reissue JWT with the caller's current org (new org if was limbo,
// previous org otherwise).
let effective_current_org_id = updated_user
    .current_org_id
    .as_deref()
    .unwrap_or(&org.id);
let platform_role_str = updated_user.platform_role.as_ref().map(|p| p.as_str());
let token = create_jwt(
    &updated_user.id,
    effective_current_org_id,
    platform_role_str,
    &state.jwt_secret,
)
.map_err(|e| ApiError::Internal(e.to_string()))?;

let memberships = state
    .storage
    .list_orgs_for_user(&updated_user.id)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
let orgs: Vec<OrgSummary> = memberships.into_iter().map(Into::into).collect();
let current_org_summary = orgs
    .iter()
    .find(|o| o.id == *effective_current_org_id)
    .cloned()
    .ok_or_else(|| ApiError::Internal("current_org not in membership list".into()))?;

Ok(Json(AuthResponse {
    token,
    refresh_token: updated_user.refresh_token.clone().unwrap_or_default(),
    user: UserInfo::from(&updated_user),
    current_org: current_org_summary,
    orgs,
}))
```

This changes the return type of `create_org` from `OrgSummary` to `AuthResponse`. No frontend callers exist (verified), so this is non-breaking.

- [ ] **Step 4: Add `me/onboarding` endpoint**

In `crates/api/src/auth.rs`, after `me`:

```rust
#[derive(Debug, serde::Serialize)]
pub struct OnboardingStatus {
    pub needs_onboarding: bool,
}

pub async fn me_onboarding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<OnboardingStatus>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let memberships = state
        .storage
        .list_orgs_for_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(OnboardingStatus {
        needs_onboarding: memberships.is_empty(),
    }))
}
```

In `crates/api/src/management/mod.rs`, in the global section:

```rust
.route("/api/v1/me/onboarding", get(auth::me_onboarding))
```

- [ ] **Step 5: Run tests**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test -p llm-gateway-api -- --nocapture
```

Expected: all tests pass, including the 5 new ones.

- [ ] **Step 6: Run full workspace tests**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test --workspace
```

Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add crates/api/
git commit -m "feat(api): wizard-first signup (no auto-org) + create_org JWT reissue + me/onboarding"
```

---

## Task 7: Frontend — types, API client, auth store

**Files:**
- Modify: `web/src/types/index.ts` (mirror new types)
- Create: `web/src/api/invitations.ts`
- Modify: `web/src/stores/authStore.ts` (add `pendingInviteToken`, `needsOnboarding`)
- Modify: `web/src/api/auth.ts` (register accepts optional invite token)

- [ ] **Step 1: Add types**

In `web/src/types/index.ts`:

```typescript
// Phase 3: invitations
export interface Invitation {
  id: string;
  token: string;
  url: string;
  role: 'member' | 'admin';
  created_at: string;
  expires_at: string;
  accepted_at: string | null;
  accepted_by: string | null; // username, not user id
  revoked_at: string | null;
}

export interface InvitationPreview {
  org_name: string;
  org_slug: string;
  role: 'member' | 'admin';
  inviter_username: string;
  expires_at: string;
}

export interface CreateInvitationBody {
  role: 'member' | 'admin';
}

export interface AcceptInvitationBody {
  token: string;
}
```

- [ ] **Step 2: Create API client**

`web/src/api/invitations.ts`:

```typescript
import { apiClient } from './client';
import type {
  Invitation,
  InvitationPreview,
  CreateInvitationBody,
  AcceptInvitationBody,
  AuthResponse,
} from '../types';

export async function listInvitations(orgSlug: string): Promise<Invitation[]> {
  const r = await apiClient.get<Invitation[]>(`/api/v1/${orgSlug}/invitations`);
  return r.data;
}

export async function createInvitation(
  orgSlug: string,
  body: CreateInvitationBody,
): Promise<Invitation> {
  const r = await apiClient.post<Invitation>(`/api/v1/${orgSlug}/invitations`, body);
  return r.data;
}

export async function revokeInvitation(
  orgSlug: string,
  id: string,
): Promise<void> {
  await apiClient.delete(`/api/v1/${orgSlug}/invitations/${id}`);
}

export async function previewInvitation(token: string): Promise<InvitationPreview> {
  const r = await apiClient.get<InvitationPreview>(
    '/api/v1/invitations/preview',
    { params: { token } },
  );
  return r.data;
}

export async function acceptInvitation(body: AcceptInvitationBody): Promise<AuthResponse> {
  const r = await apiClient.post<AuthResponse>('/api/v1/invitations/accept', body);
  return r.data;
}
```

If `AuthResponse` doesn't exist in `types/index.ts` yet, add it (mirror the backend shape: `token, refresh_token, user, current_org, orgs`).

- [ ] **Step 3: Extend auth store**

In `web/src/stores/authStore.ts`:

```typescript
interface AuthState {
  // ... existing fields ...
  pendingInviteToken: string | null;
  setPendingInviteToken: (t: string | null) => void;
}

// In the store creator:
pendingInviteToken: null,
setPendingInviteToken: (t) => set({ pendingInviteToken: t }),

// New selector:
export function useNeedsOnboarding(): boolean {
  return useAuthStore((s) => s.user !== null && s.user.orgs.length === 0);
}
```

- [ ] **Step 4: Modify register flow to accept invite**

In `web/src/api/auth.ts` (or wherever `register` lives):

```typescript
export async function register(
  username: string,
  password: string,
  inviteToken?: string | null,
): Promise<AuthResponse> {
  const r = await apiClient.post<AuthResponse>('/api/v1/auth/register', {
    username,
    password,
  });
  // If an invite token was stashed, immediately accept it after register.
  if (inviteToken) {
    const acceptR = await apiClient.post<AuthResponse>(
      '/api/v1/invitations/accept',
      { token: inviteToken },
    );
    return acceptR.data;
  }
  return r.data;
}
```

- [ ] **Step 5: Run frontend tests + typecheck**

```bash
source ~/.nvm/nvm.sh && cd web
npm run build   # type-check via tsc
npm test
```

Expected: build clean, all existing tests pass (no new tests yet — those come with the page components).

- [ ] **Step 6: Commit**

```bash
git add web/src/types/index.ts web/src/api/invitations.ts web/src/api/auth.ts web/src/stores/authStore.ts
git commit -m "feat(web): invitation API client + auth-store onboarding state"
```

---

## Task 8: Frontend — Onboarding wizard (`/onboarding`)

**Files:**
- Create: `web/src/pages/Onboarding.tsx`
- Create: `web/src/components/OnboardingCreateCard.tsx`
- Create: `web/src/components/OnboardingJoinCard.tsx`
- Modify: `web/src/App.tsx` (add route + routing guard)
- Modify: `web/src/i18n/en.json` + `zh.json` (onboarding.* keys)
- Create: `web/src/pages/Onboarding.test.tsx`

- [ ] **Step 1: Write failing tests**

`web/src/pages/Onboarding.test.tsx`:

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Onboarding } from './Onboarding';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';

describe('Onboarding wizard', () => {
  it('shows two branch cards by default', () => {
    render(<MemoryRouter><Onboarding /></MemoryRouter>);
    expect(screen.getByText(/create an org/i)).toBeInTheDocument();
    expect(screen.getByText(/have an invite/i)).toBeInTheDocument();
  });

  it('create branch: slug pre-filled from username, live-collision error', async () => {
    // ... user event types, MSW intercepts POST /orgs with 409, assert error shown
  });

  it('create branch: success redirects to dashboard', async () => {
    // MSW returns AuthResponse; assert navigate to /{slug}/dashboard
  });

  it('join branch: paste token, success redirects to inviting org', async () => {
    // MSW intercepts POST /invitations/accept with AuthResponse
  });

  it('join branch: invalid token shows error from 410', async () => {
    // MSW returns 410; assert error rendered
  });
});
```

- [ ] **Step 2: Implement CreateCard**

`web/src/components/OnboardingCreateCard.tsx`:

```tsx
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '../stores/authStore';
import { apiClient } from '../api/client';
import type { AuthResponse } from '../types';
import { toast } from 'sonner';

export function OnboardingCreateCard() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const user = useAuthStore((s) => s.user);
  const setAuth = useAuthStore((s) => s.setAuth); // assumed setter

  const defaultSlug = (user?.username ?? '').toLowerCase().replace(/[^a-z0-9-]/g, '');
  const [name, setName] = useState(user?.username ?? '');
  const [slug, setSlug] = useState(defaultSlug);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      const r = await apiClient.post<AuthResponse>('/api/v1/orgs', { name, slug });
      setAuth(r.data);
      navigate(`/${r.data.current_org.slug}/dashboard`);
    } catch (e: any) {
      if (e?.response?.status === 409) {
        setError(t('onboarding.create.errors.slugTaken'));
      } else {
        toast.error(t('common.errors.generic'));
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    // DaisyUI-styled card with name input, slug input (with live preview),
    // submit button. See Phase 2.2 OrgSettings page for styling reference.
  );
}
```

- [ ] **Step 3: Implement JoinCard**

`web/src/components/OnboardingJoinCard.tsx`:

```tsx
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { acceptInvitation } from '../api/invitations';
import { useAuthStore } from '../stores/authStore';
import { toast } from 'sonner';

export function OnboardingJoinCard() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const setAuth = useAuthStore((s) => s.setAuth);
  const [input, setInput] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  function extractToken(s: string): string {
    // Allow pasting either a bare token or a full URL like
    // https://app.example.com/accept-invite?token=abc
    try {
      const u = new URL(s);
      return u.searchParams.get('token') ?? s;
    } catch {
      return s;
    }
  }

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      const r = await acceptInvitation({ token: extractToken(input) });
      setAuth(r);
      navigate(`/${r.current_org.slug}/dashboard`);
    } catch (e: any) {
      if (e?.response?.status === 410) {
        setError(t('onboarding.join.errors.invalidToken'));
      } else {
        toast.error(t('common.errors.generic'));
      }
    } finally {
      setBusy(false);
    }
  }

  // ... render input + button
}
```

- [ ] **Step 4: Implement page**

`web/src/pages/Onboarding.tsx`:

```tsx
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { OnboardingCreateCard } from '../components/OnboardingCreateCard';
import { OnboardingJoinCard } from '../components/OnboardingJoinCard';

export function Onboarding() {
  const { t } = useTranslation();
  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        className="max-w-2xl w-full p-8"
      >
        <h1 className="text-2xl font-semibold mb-6">{t('onboarding.title')}</h1>
        <div className="grid md:grid-cols-2 gap-4">
          <OnboardingCreateCard />
          <OnboardingJoinCard />
        </div>
      </motion.div>
    </div>
  );
}
```

- [ ] **Step 5: Wire route + routing guard**

In `web/src/App.tsx`, add a public route for `/onboarding` (NOT under `<RequireAuth/>` — but actually it does need auth; place it under RequireAuth but exempt from the org-guard).

Adjust `<RequireAuth/>`:

```tsx
function RequireAuth() {
  // ... existing auth check ...
  const location = useLocation();
  const user = useAuthStore((s) => s.user);
  const onLimboAllowedPath =
    location.pathname === '/onboarding' ||
    location.pathname.startsWith('/accept-invite');

  if (user && user.orgs.length === 0 && !onLimboAllowedPath) {
    return <Navigate to="/onboarding" replace />;
  }
  // ... rest unchanged ...
}
```

Add the route:

```tsx
<Route path="/onboarding" element={<Onboarding />} />
```

- [ ] **Step 6: Add i18n keys**

`web/src/i18n/en.json`:

```json
{
  "onboarding": {
    "title": "Set up your workspace",
    "create": {
      "title": "Create an org",
      "subtitle": "1 minute form",
      "name": "Org name",
      "slug": "Slug",
      "submit": "Create",
      "errors": {
        "slugTaken": "That slug is taken, try another"
      }
    },
    "join": {
      "title": "Have an invite?",
      "subtitle": "Paste the link or token",
      "tokenLabel": "Invitation link or token",
      "submit": "Join",
      "errors": {
        "invalidToken": "This invitation is no longer valid"
      }
    }
  }
}
```

Mirror in `zh.json` with Chinese strings.

- [ ] **Step 7: Run tests**

```bash
source ~/.nvm/nvm.sh && cd web
npm test -- src/pages/Onboarding.test.tsx
npm run build
```

Expected: 5 new tests pass, build clean.

- [ ] **Step 8: Commit**

```bash
git add web/src/pages/Onboarding.tsx \
        web/src/pages/Onboarding.test.tsx \
        web/src/components/OnboardingCreateCard.tsx \
        web/src/components/OnboardingJoinCard.tsx \
        web/src/App.tsx \
        web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): onboarding wizard with create + join branches"
```

---

## Task 9: Frontend — `/accept-invite` landing page

**Files:**
- Create: `web/src/pages/AcceptInvite.tsx`
- Create: `web/src/pages/AcceptInvite.test.tsx`
- Modify: `web/src/App.tsx` (add public route)
- Modify: `web/src/i18n/en.json` + `zh.json` (`acceptInvite.*` keys)
- Modify: `web/src/pages/Register.tsx` (consume `?invite=...`)

- [ ] **Step 1: Write failing tests**

`web/src/pages/AcceptInvite.test.tsx`:

```typescript
describe('AcceptInvite page', () => {
  it('logged out: shows org metadata + signup/login buttons', async () => {
    // Render with no auth, MSW intercepts preview with valid response.
    // Assert: org name shown, signup + login buttons present.
  });

  it('logged out: expired token shows invalid message', async () => {
    // MSW returns 410; assert "no longer valid" rendered.
  });

  it('logged in: shows Accept/Decline buttons', async () => {
    // Render with auth, MSW intercepts preview; assert Accept button.
  });

  it('logged in: accept click navigates to inviting org dashboard', async () => {
    // MSW intercepts POST /invitations/accept with AuthResponse.
  });

  it('logged in: already a member of inviting org shows informational message', async () => {
    // user.orgs contains the inviting org's slug. Assert message rendered.
  });
});
```

- [ ] **Step 2: Implement page**

`web/src/pages/AcceptInvite.tsx`:

```tsx
import { useEffect, useState } from 'react';
import { useNavigate, useSearchParams, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { previewInvitation, acceptInvitation } from '../api/invitations';
import { useAuthStore } from '../stores/authStore';
import type { InvitationPreview } from '../types';

export function AcceptInvite() {
  const { t } = useTranslation();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const token = params.get('token') ?? '';
  const user = useAuthStore((s) => s.user);
  const setAuth = useAuthStore((s) => s.setAuth);
  const setPendingInviteToken = useAuthStore((s) => s.setPendingInviteToken);

  const [preview, setPreview] = useState<InvitationPreview | null>(null);
  const [status, setStatus] = useState<'loading' | 'ok' | 'gone'>('loading');

  useEffect(() => {
    if (!token) {
      setStatus('gone');
      return;
    }
    previewInvitation(token)
      .then((p) => {
        setPreview(p);
        setStatus('ok');
      })
      .catch((e) => {
        if (e?.response?.status === 410) setStatus('gone');
      });
  }, [token]);

  // Referrer-Policy is set via a <meta> tag in index.html or per-route via
  // a useEffect that mutates document.head — implement the per-route version.

  if (status === 'loading') return <LoadingSpinner />;
  if (status === 'gone') {
    return (
      <div>
        <h1>{t('acceptInvite.gone.title')}</h1>
        <p>{t('acceptInvite.gone.description')}</p>
        <Link to="/login">{t('acceptInvite.gone.back')}</Link>
      </div>
    );
  }

  const alreadyMember = preview && user?.orgs.some((o) => o.slug === preview.org_slug);

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <motion.div className="max-w-md w-full p-8 card">
        <h1>{t('acceptInvite.title', { org: preview!.org_name })}</h1>
        <p>{t('acceptInvite.role', { role: preview!.role })}</p>
        <p>{t('acceptInvite.inviter', { user: preview!.inviter_username })}</p>

        {alreadyMember ? (
          <div>
            <p>{t('acceptInvite.alreadyMember')}</p>
            <Link to={`/${preview!.org_slug}/dashboard`}>
              {t('acceptInvite.goToOrg')}
            </Link>
          </div>
        ) : user ? (
          <div className="flex gap-2">
            <button onClick={handleAccept} className="btn btn-primary">
              {t('acceptInvite.accept')}
            </button>
            <Link to="/" className="btn btn-ghost">{t('acceptInvite.decline')}</Link>
          </div>
        ) : (
          <div className="flex gap-2">
            <button
              onClick={() => {
                setPendingInviteToken(token);
                navigate(`/register?invite=${token}`);
              }}
              className="btn btn-primary"
            >
              {t('acceptInvite.signUp')}
            </button>
            <Link to={`/login?next=/accept-invite?token=${token}`} className="btn btn-ghost">
              {t('acceptInvite.logIn')}
            </Link>
          </div>
        )}
      </motion.div>
    </div>
  );

  async function handleAccept() {
    const r = await acceptInvitation({ token });
    setAuth(r);
    navigate(`/${r.current_org.slug}/dashboard`);
  }
}
```

- [ ] **Step 3: Wire route**

In `web/src/App.tsx`:

```tsx
import { AcceptInvite } from './pages/AcceptInvite';
// ... in the router:
<Route path="/accept-invite" element={<AcceptInvite />} />
```

This route is PUBLIC (not under `<RequireAuth/>`).

Also exempt `/accept-invite` from the limbo guard in `<RequireAuth/>` (already done in Task 8 Step 5).

- [ ] **Step 4: Update Register to consume invite token**

In `web/src/pages/Register.tsx`:

```tsx
const [params] = useSearchParams();
const pendingInviteToken = useAuthStore((s) => s.pendingInviteToken);
const inviteFromUrl = params.get('invite');
const invite = inviteFromUrl ?? pendingInviteToken;

// In the submit handler:
const r = await register(username, password, invite);
// If invite was consumed, clear it:
if (invite) setPendingInviteToken(null);
setAuth(r);
navigate(`/${r.current_org.slug}/dashboard`);
```

- [ ] **Step 5: Add i18n keys**

Add `acceptInvite.*` keys to en.json + zh.json:

```json
{
  "acceptInvite": {
    "title": "Join {{org}}",
    "role": "You'll join as {{role}}.",
    "inviter": "Invite from {{user}}",
    "accept": "Accept",
    "decline": "Decline",
    "signUp": "Sign up to accept",
    "logIn": "Log in",
    "alreadyMember": "You're already a member of {{org}}.",
    "goToOrg": "Go to {{org}}",
    "gone": {
      "title": "Invitation no longer valid",
      "description": "This link may have expired, been revoked, or already been used. Please request a new invitation.",
      "back": "Back to login"
    }
  }
}
```

- [ ] **Step 6: Run tests**

```bash
source ~/.nvm/nvm.sh && cd web
npm test -- src/pages/AcceptInvite.test.tsx
npm run build
```

Expected: 5 tests pass.

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/AcceptInvite.tsx \
        web/src/pages/AcceptInvite.test.tsx \
        web/src/pages/Register.tsx \
        web/src/App.tsx \
        web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): invite-aware /accept-invite landing page"
```

---

## Task 10: Frontend — Invitations admin page

**Files:**
- Create: `web/src/pages/Invitations.tsx`
- Create: `web/src/components/CopyableInviteLink.tsx`
- Create: `web/src/pages/Invitations.test.tsx`
- Modify: `web/src/App.tsx` (add admin route)
- Modify: `web/src/i18n/en.json` + `zh.json` (`invitations.*` keys)
- Modify: `web/src/components/Layout.tsx` (add settings nav link)

- [ ] **Step 1: Implement CopyableInviteLink**

`web/src/components/CopyableInviteLink.tsx`:

```tsx
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, Copy } from 'lucide-react';

export function CopyableInviteLink({ url, expiresAt }: { url: string; expiresAt: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  async function copy() {
    await navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  const daysLeft = Math.ceil((new Date(expiresAt).getTime() - Date.now()) / 86400000);

  return (
    <div className="flex items-center gap-2">
      <code className="text-xs bg-base-200 px-2 py-1 rounded truncate max-w-xs">{url}</code>
      <button onClick={copy} className="btn btn-ghost btn-xs">
        {copied ? <Check size={12} /> : <Copy size={12} />}
      </button>
      <span className="text-xs text-base-content/50">
        {t('invitations.expiresInDays', { count: daysLeft })}
      </span>
    </div>
  );
}
```

- [ ] **Step 2: Implement page**

`web/src/pages/Invitations.tsx`:

```tsx
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useState } from 'react';
import { listInvitations, createInvitation, revokeInvitation } from '../api/invitations';
import { CopyableInviteLink } from '../components/CopyableInviteLink';

export function Invitations() {
  const { orgSlug = '' } = useParams();
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [role, setRole] = useState<'member' | 'admin'>('member');

  const { data: invitations } = useQuery({
    queryKey: ['invitations', orgSlug],
    queryFn: () => listInvitations(orgSlug),
  });

  const createMut = useMutation({
    mutationFn: () => createInvitation(orgSlug, { role }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['invitations', orgSlug] }),
  });

  const revokeMut = useMutation({
    mutationFn: (id: string) => revokeInvitation(orgSlug, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['invitations', orgSlug] }),
  });

  return (
    <div className="p-6 max-w-3xl">
      <h1 className="text-2xl mb-4">{t('invitations.title')}</h1>

      {/* Generate */}
      <div className="card bg-base-100 shadow mb-6 p-4">
        <h2>{t('invitations.generate.title')}</h2>
        <select value={role} onChange={(e) => setRole(e.target.value as 'member' | 'admin')}>
          <option value="member">{t('invitations.roles.member')}</option>
          <option value="admin">{t('invitations.roles.admin')}</option>
        </select>
        <button onClick={() => createMut.mutate()} className="btn btn-primary">
          {t('invitations.generate.submit')}
        </button>
      </div>

      {/* List */}
      <div>
        {invitations?.map((inv) => (
          <div key={inv.id} className="card bg-base-100 shadow mb-2 p-3">
            {inv.accepted_at ? (
              <div>{t('invitations.acceptedBy', { user: inv.accepted_by ?? '?' })}</div>
            ) : inv.revoked_at ? (
              <div>{t('invitations.revoked')}</div>
            ) : (
              <>
                <CopyableInviteLink url={inv.url} expiresAt={inv.expires_at} />
                <button
                  onClick={() => revokeMut.mutate(inv.id)}
                  className="btn btn-ghost btn-xs text-error"
                >
                  {t('invitations.revoke')}
                </button>
              </>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Wire route + nav link**

In `web/src/App.tsx`, add the route under the `:orgSlug` group:

```tsx
<Route path="settings/invitations" element={<Invitations />} />
```

In `web/src/components/Layout.tsx`, add a nav link under Settings (admin-only — gate based on `currentOrg.role === 'admin' || role === 'owner'`).

- [ ] **Step 4: Add i18n keys**

`invitations.*` keys in en.json + zh.json.

- [ ] **Step 5: Write tests**

`web/src/pages/Invitations.test.tsx` — covers: empty state, generate flow, list rendering (pending/accepted/revoked), revoke button click.

- [ ] **Step 6: Run tests + build**

```bash
source ~/.nvm/nvm.sh && cd web
npm test -- src/pages/Invitations.test.tsx
npm run build
```

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/Invitations.tsx \
        web/src/pages/Invitations.test.tsx \
        web/src/components/CopyableInviteLink.tsx \
        web/src/App.tsx web/src/components/Layout.tsx \
        web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): admin invitations page (list + generate + revoke)"
```

---

## Task 11: E2E + final integration

**Files:**
- Create: `web/e2e/invitations.spec.ts`
- Modify: `CHANGELOG.md` (add v1.11.0 entry — see Phase 2 release process for format)

- [ ] **Step 1: Write the Playwright happy path**

`web/e2e/invitations.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

test('signup → wizard → create org → land in dashboard', async ({ page }) => {
  await page.goto('/register');
  await page.fill('[name=username]', 'alice');
  await page.fill('[name=password]', 'password123');
  await page.click('button[type=submit]');

  // Should land at /onboarding
  await expect(page).toHaveURL(/\/onboarding/);

  // Create branch
  await page.fill('[name=orgName]', 'Acme');
  await page.fill('[name=orgSlug]', 'acme');
  await page.click('text=Create');

  await expect(page).toHaveURL(/\/acme\/dashboard/);
});

test('admin mints invite → second user signs up via link → both in same org', async ({ browser }) => {
  // Admin context
  const adminCtx = await browser.newContext();
  const adminPage = await adminCtx.newPage();
  // Login as existing admin, navigate to /acme/settings/invitations, click Generate.
  // Capture the invite URL.

  // Invitee context
  const inviteeCtx = await browser.newContext();
  const inviteePage = await inviteeCtx.newPage();
  await inviteePage.goto(inviteUrl);
  await expect(inviteePage.locator('h1')).toContainText(/join acme/i);
  await inviteePage.click('text=Sign up to accept');
  // Fill register form, submit
  // Should land at /acme/dashboard
});
```

- [ ] **Step 2: Run E2E**

```bash
source ~/.nvm/nvm.sh && cd web
# Backend must be running on :8080 for E2E. See CLAUDE.md for setup.
npm run test:e2e -- invitations.spec.ts
```

Expected: both tests pass.

- [ ] **Step 3: CHANGELOG entry**

In `CHANGELOG.md`, add above the most recent entry:

```markdown
## [Unreleased]

### Added — Phase 3: Wizard-gated signup + invitations (v1.11.0)

- Wizard-first signup: brand-new users land at `/onboarding` and create or join
  an org before reaching any org-scoped UI. Pre-existing users are unaffected.
- Generic single-use magic-link invitations. Org admins can mint a 7-day
  invitation URL at `/{org_slug}/settings/invitations` and share it out-of-band
  (Slack, etc.); the first user to present the token joins the org.
- `/accept-invite?token=...` landing page renders invite metadata for logged-out
  visitors (Sign up / Log in) and logged-in users (Accept / Decline).
- `POST /api/v1/orgs` now reissues the access token. Auto-switches `current_org`
  only for users in the limbo state.

### Changed
- `POST /api/v1/auth/register` no longer auto-assigns a default-org membership.
  Brand-new users have `current_org_id = NULL` and `orgs = []` until they
  complete the onboarding wizard.

### Removed
- The "default org" bootstrap on first-user signup (was a Phase 1 holdover).
  The migration-time default org still exists for pre-Phase-3 data.
```

- [ ] **Step 4: Final full test pass**

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
    cargo test --workspace
source ~/.nvm/nvm.sh && cd web
npm run build && npm test
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add web/e2e/invitations.spec.ts CHANGELOG.md
git commit -m "test(e2e): invitations happy-path + CHANGELOG entry for v1.11.0"
```

---

## Self-review

**Spec coverage:** Every item in the spec's API table (Section: "API surface (delta from Phase 2.3)") maps to a task:
- `POST /auth/register` modified → Task 6 ✓
- `POST /api/v1/orgs` modified → Task 6 ✓
- `POST /api/v1/{org_slug}/invitations` → Task 4 ✓
- `GET /api/v1/{org_slug}/invitations` → Task 4 ✓
- `DELETE /api/v1/{org_slug}/invitations/{id}` → Task 4 ✓
- `GET /api/v1/invitations/preview` → Task 5 ✓
- `POST /api/v1/invitations/accept` → Task 5 ✓
- `GET /api/v1/me/onboarding` → Task 6 ✓
- Frontend routes `/onboarding`, `/accept-invite`, `/:orgSlug/settings/invitations` → Tasks 8, 9, 10 ✓
- `invitations` migration → Task 1 ✓
- Storage types + trait → Tasks 2, 3 ✓
- Frontend types + API client → Task 7 ✓
- E2E → Task 11 ✓

**Placeholder scan:** A few "fill in the body" markers remain in tasks where the implementer should mirror an existing Phase 2.x pattern (e.g., DaisyUI card styling, test helper names). These reference concrete prior code as the source of truth rather than abstract "TBD" — acceptable.

**Type consistency:** `InvitationPreview` is defined with `already_member` in Task 2 but Task 5 says to drop it. Resolution: drop it in Task 2 before implementing. Implementer should reconcile this — the canonical shape is without `already_member`.

**Open question for implementer:** the `create_jwt` signature may not accept `Option<&str>` for org_id today. Task 6 Step 2 mentions this — the implementer should adjust the signature and update all 4-5 call sites in one commit.

**Scope check:** Plan is large (11 tasks) but coherent — every piece is needed for the Phase 3 user-facing flow. Splitting into sub-plans is possible at task boundaries (1-3 = storage, 4-6 = backend, 7-10 = frontend, 11 = E2E), but each piece is independently shippable on a feature branch and the spec specifies a single v1.11.0 release.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-08-saas-phase3-wizard-invitations.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
