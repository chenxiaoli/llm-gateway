# Email Auth Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Drop the requirement that every user has a `username`; login accepts either username or email, registration collects email only.

**Architecture:** One permissive DB migration (`users.username` becomes nullable, CHECK constraint enforces "username or email"). Backend login handler branches on `@`; register handler drops the `username` field; CLI grant command does the same branch. Frontend widens `User.username` to `string | null`, adds a `displayName()` helper, updates the two auth pages and the seven call sites that render `username` directly.

**Tech Stack:** Rust (Axum + sqlx), React 18 + TypeScript + Vite, vitest + MSW, react-i18next.

**Spec:** `docs/superpowers/specs/2026-07-13-email-auth-design.md`

---

## File Structure

**Backend (Rust) — type cascade:**
- Modify: `crates/storage/src/types.rs:1003` (`User.username: String → Option<String>`), `:1020` (`CreateUser.username`), `:263` (`InvitationPreview.inviter_username`).
- Modify: `crates/api/src/auth.rs` — `LoginRequest` (unchanged shape, just doc the widening), `RegisterRequest` (drop `username`, add `deny_unknown_fields`), `UserInfo.username: Option<String>` (`:106`), `MeResponse.username: Option<String>` (`:135`), login handler (`:268`), register handler (`:340`), `From<&User> for UserInfo` (`:206-214`), plus mechanical `username: user.username.clone()` at `:210, :499, :667, :721, :827, :908, :948` (clone stays valid since both sides are now `Option<String>`).
- Modify: `crates/api/src/management/invitations.rs:110` (inviter_username lookup falls back to email), `:283` (constructs `InvitationPreview`), `:795` (test assertion).
- Modify: `crates/gateway/src/main.rs:253` — CLI grant branching.
- Modify: `crates/gateway/src/cli.rs:21` — help text update.

**Backend (Rust) — migration:**
- Create: `crates/storage/migrations/postgres/20260713000001_users_username_optional.sql`

**Frontend:**
- Create: `web/src/lib/displayName.ts`
- Modify: `web/src/types/index.ts` — widen `User.username`, `MeResponse.username`, `InvitationPreview.inviter_username` to `string | null`; remove `username` from `RegisterRequest`.
- Modify: `web/src/pages/Login.tsx` — label + placeholder text.
- Modify: `web/src/pages/Register.tsx` — remove username field + state.
- Modify: `web/src/i18n/{en,zh}.json` — add `auth.usernameOrEmail`.
- Modify (call sites): `web/src/components/Layout.tsx`, `web/src/components/PlatformLayout.tsx`, `web/src/components/ImpersonationBanner.tsx`, `web/src/components/OnboardingCreateCard.tsx`, `web/src/pages/Account.tsx`.
- Modify: `web/src/pages/Login.test.tsx`, `web/src/pages/Register.test.tsx` — adapt existing tests.

---

## Task 1: DB migration — `users.username` nullable

**Files:**
- Create: `crates/storage/migrations/postgres/20260713000001_users_username_optional.sql`

- [ ] **Step 1: Write the migration SQL**

Create `crates/storage/migrations/postgres/20260713000001_users_username_optional.sql`:

```sql
-- Drop the NOT NULL on username so email-only registrations are valid.
-- Existing rows all have a username, so the change is permissive: no
-- backfill needed. UNIQUE constraint on username is preserved (duplicates
-- were never allowed and still aren't); the partial UNIQUE index on
-- LOWER(email) is unaffected.
--
-- The CHECK constraint enforces "at least one identifier" — a row with
-- both username and email NULL is rejected. Both being set is allowed
-- (e.g. legacy users who later add an email).

ALTER TABLE users ALTER COLUMN username DROP NOT NULL;

ALTER TABLE users ADD CONSTRAINT users_username_or_email_required
    CHECK (username IS NOT NULL OR email IS NOT NULL);
```

- [ ] **Step 2: Verify migration applies cleanly**

Run: `cargo test --workspace -p llm-gateway-storage -- --nocapture`
Expected: all existing tests pass (the embedded migrations runner picks up the new file automatically via `MIGRATOR`).

If any test fails with a CHECK violation on `users`, it's because a test fixture inserts a user row without an email — that test needs to be updated to include `email`. Investigate before continuing.

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/postgres/20260713000001_users_username_optional.sql
git commit -m "feat(storage): make users.username nullable, require username or email"
```

---

## Task 2: Storage layer — widen nullable username fields

**Files:**
- Modify: `crates/storage/src/types.rs:1003` (struct `User`)
- Modify: `crates/storage/src/types.rs:1020` (struct `CreateUser`)
- Modify: `crates/storage/src/types.rs:263` (struct `InvitationPreview` — `inviter_username`)

- [ ] **Step 1: Widen the fields on `User`, `CreateUser`, and `InvitationPreview`**

In `crates/storage/src/types.rs`, change:

```rust
pub struct User {
    pub id: String,
    pub username: String,                       // ← change to Option<String>
    // ...
}

pub struct CreateUser {
    pub username: String,                       // ← change to Option<String>
    pub password: String,
}

// around line 263:
pub struct InvitationPreview {
    pub org_name: String,
    pub org_slug: String,
    pub role: MemberRole,
    pub inviter_username: String,               // ← change to Option<String>
    pub recipient_email: String,
    pub expires_at: DateTime<Utc>,
}
```

to:

```rust
pub struct User {
    pub id: String,
    pub username: Option<String>,
    // ...
}

pub struct CreateUser {
    pub username: Option<String>,
    pub password: String,
}

pub struct InvitationPreview {
    pub org_name: String,
    pub org_slug: String,
    pub role: MemberRole,
    pub inviter_username: Option<String>,
    pub recipient_email: String,
    pub expires_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Drive the cascade with `cargo check`**

Run: `cargo check --workspace 2>&1 | tee /tmp/cargo-check.log`
Expected: many compile errors at every site that constructs `User { username: ... }` or treats `user.username` as `String`. These are mechanical fixes — for each error, either:
- Where a row is built from a DB query (e.g. `postgres.rs` `fetch_user`): the SELECT should be `username` (which is now nullable in the DB) — if sqlx complains, change the mapped struct field to `Option<String>` (already done in step 1).
- Where a `User` is constructed in code with a known username: wrap the literal in `Some(...)` — e.g. `username: Some("alice".to_string())`.
- Where code reads `user.username` as `String` (e.g. logging, formatting): change to `user.username.as_deref().unwrap_or("")` or `user.username.clone().unwrap_or_default()`.

Do NOT touch `crates/api/src/auth.rs` in this task — that's Task 3. Just make `cargo check --workspace` pass for storage and any non-`api`/non-`gateway` crates. (The api + gateway crates will still fail; that's expected.)

If you hit `crates/api` or `crates/gateway` errors during this step, defer them — Task 3 handles those.

- [ ] **Step 3: Run storage tests**

Run: `cargo test --workspace -p llm-gateway-storage`
Expected: all storage tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/storage/
git commit -m "feat(storage): widen User.username to Option<String>"
```

---

## Task 3: API + CLI — login branching, register drops username, CLI grant branch

**Files:**
- Modify: `crates/api/src/auth.rs` — `RegisterRequest` (`:31-45`), `UserInfo.username` (`:106`), `MeResponse.username` (`:135`), login handler (`:268-327`), register handler (`:340-489`), `From<&User> for UserInfo` (`:206-214`), plus mechanical `username: user.username.clone()` → `username: user.username.clone()` at `:210, :499, :667, :721, :827, :908, :948` (clone stays valid since both sides are now `Option<String>`).
- Modify: `crates/gateway/src/cli.rs:21` (help text).
- Modify: `crates/gateway/src/main.rs:253` (CLI grant resolver branch).
- Test: `crates/api/src/auth.rs` test module starting `:1865`.

- [ ] **Step 1: Widen `UserInfo.username` and `MeResponse.username`**

In `crates/api/src/auth.rs`:

```rust
#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: Option<String>,
    pub platform_role: Option<String>,
}

// ...

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub username: Option<String>,
    pub platform_role: Option<String>,
    // ... rest unchanged
}
```

`From<&User> for UserInfo` at `:206` stays as `username: u.username.clone()` — both sides are `Option<String>` now.

- [ ] **Step 2: Update `LoginRequest` doc; branch in login handler**

`LoginRequest` keeps its `username: String` field name (the wire format is unchanged for backward compat — only its meaning widens). Add a doc comment:

```rust
#[derive(Deserialize)]
pub struct LoginRequest {
    /// Identifier supplied by the user. May be a username or an email —
    /// the login handler branches on `@` to pick the lookup. Field name
    /// is `username` for wire-format backward compat with existing clients.
    pub username: String,
    pub password: String,
}
```

Update the login handler at `:268` to branch on `@`:

```rust
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    validate_password(&input.password).map_err(ApiError::BadRequest)?;

    let user = if input.username.contains('@') {
        state
            .storage
            .get_user_by_email(&input.username)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
    } else {
        state
            .storage
            .get_user_by_username(&input.username)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
    }
    .ok_or(ApiError::Unauthorized)?;

    // ... rest of the handler (enabled check, password verify, email gate,
    // JWT, refresh token, AuthResponse) unchanged.
```

Leave the rest of the handler (`enabled` check, `verify_password`, `requires_email_verification` gate, JWT, refresh token, response) exactly as-is.

- [ ] **Step 3: Drop `username` from `RegisterRequest` and the register handler**

Change:

```rust
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub invite_token: Option<String>,
}
```

to:

```rust
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub password: String,
    /// Required — the new user must verify this email before they can log in.
    /// `Option<String>` so a missing field deserializes to `None` and we return
    /// the typed `EmailRequired` (400) error instead of Axum's default 422.
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub invite_token: Option<String>,
}
```

In the register handler at `:340`, remove:
- The line `validate_username(&input.username).map_err(ApiError::BadRequest)?;` (`:358`)
- The duplicate-username check at `:371-379`:
  ```rust
  if state.storage.get_user_by_username(&input.username).await...is_some() {
      return Err(ApiError::BadRequest("Username already exists".to_string()));
  }
  ```

In the `User { ... }` construction at `:427-444`, change:

```rust
let user = User {
    id: uuid::Uuid::new_v4().to_string(),
    username: input.username.clone(),
    // ...
};
```

to:

```rust
let user = User {
    id: uuid::Uuid::new_v4().to_string(),
    username: None,  // Email-only registration; username may be added later via profile UI.
    // ...
};
```

Also remove the `validate_username` import from the `use` block at the top of the file (`:9`) if it's no longer referenced elsewhere — `cargo check` will confirm.

- [ ] **Step 4: Cascade `InvitationPreview` inviter lookup in API layer**

Because the `InvitationPreview` storage type was widened in Task 2 (inviter_username is now `Option<String>`), the API code that builds it must use the `username.or(email)` fallback for inviters with no username.

In `crates/api/src/management/invitations.rs:110`, the lookup currently does something like `let inviter_username = state.storage.get_user(...)?.username;`. Read the surrounding code first, then change to:

```rust
let inviter = state
    .storage
    .get_user(&inv.invited_by)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::NotFound("inviter not found".into()))?;
let inviter_username = inviter.username.clone().or(inviter.email.clone());
```

(Exact field names like `inv.invited_by` may differ — read the file. The principle: never construct `InvitationPreview` from a bare `String` — always go through `username.or(email)` so the field is `Option<String>`.)

At `crates/api/src/management/invitations.rs:795`, the existing assertion `assert_eq!(body.inviter_username, "owner-acme")` won't compile because the field is now `Option<String>`. Change to:

```rust
assert_eq!(body.inviter_username.as_deref(), Some("owner-acme"));
```

- [ ] **Step 5: Update the CLI grant subcommand**

In `crates/gateway/src/cli.rs:21`, update the help text:

```rust
GrantPlatformAdmin {
    /// Username or email of the target user. Must already exist.
    #[arg(long)]
    username: String,
    // ...
}
```

Field name stays `username` for CLI arg compatibility (`--username alice@example.com` works the same as `--username alice`).

In `crates/gateway/src/main.rs:253`, branch on `@`:

```rust
let user = if username.contains('@') {
    db.get_user_by_email(&username).await?
} else {
    db.get_user_by_username(&username).await?
}
.ok_or_else(|| {
    eprintln!("error: user '{username}' not found");
    "user not found"
})?;
```

The downstream `set_user_platform_role` call and the println messages stay unchanged (they reference `username` as the user-supplied identifier string, which is fine).

- [ ] **Step 6: Write failing tests for the new login + register behavior**

In the test module at the bottom of `crates/api/src/auth.rs` (starting around `:1860`), the existing `register_returns_jwt_with_null_current_org` test at `:1865` will fail because it sends `{"username": "alice", ...}` — update it to drop `username`:

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn register_returns_jwt_with_null_current_org(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
    let app = build_router(storage);
    let resp = post_json(
        &app,
        "/api/v1/auth/register",
        None,
        json!({"password": "password123", "email": "alice@example.com"}),
    )
    .await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["current_org"].is_null());
    assert!(body["orgs"].as_array().unwrap().is_empty());
    assert!(body["token"].is_string());

    // Look up the user by email (the row has username = NULL).
    let row: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT username, current_org_id FROM users WHERE email = 'alice@example.com'",
    )
    .fetch_one(&pool)
    .await
    .expect("user row");
    assert!(row.0.is_none(), "username must be NULL for email-only signup");
    assert!(row.1.is_none(), "DB current_org_id must be NULL for limbo user");
}
```

Then add these new tests below it:

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn login_accepts_email_identifier(pool: PgPool) {
    // Register an email-only user, then log in by email. Verifies the '@'
    // branch in the login handler resolves to the right row.
    let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
    let app = build_router(storage);

    // Register (skips email verification gate at login only if we also flip
    // email_verified_at — but we instead just check the login result code is
    // email_not_verified, which proves the email lookup found the row).
    let _ = post_json(
        &app,
        "/api/v1/auth/register",
        None,
        json!({"password": "password123", "email": "alice@example.com"}),
    )
    .await;

    let resp = post_json(
        &app,
        "/api/v1/auth/login",
        None,
        json!({"username": "alice@example.com", "password": "password123"}),
    )
    .await;
    // User hasn't verified email — expect 403 email_not_verified (proves the
    // email branch resolved the row, otherwise we'd get 401 invalid creds).
    assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
    let body = body_json(resp).await;
    assert_eq!(body["error"]["code"], "email_not_verified");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn login_accepts_username_identifier_for_legacy_user(pool: PgPool) {
    // Convert a freshly-registered email-only user into a "legacy-style
    // verified user with a username" by SQL UPDATE, then log in by username.
    // This avoids hardcoding an argon2 hash in the test (we let /auth/register
    // hash the password, then mutate the row).
    let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
    let app = build_router(storage);

    let _ = post_json(
        &app,
        "/api/v1/auth/register",
        None,
        json!({"password": "password123", "email": "alice@example.com"}),
    )
    .await;

    // Mutate the row: set username, mark email verified, disable the
    // verification gate (mirrors a legacy user who predates Phase 4).
    sqlx::query(
        "UPDATE users SET username = 'alice', email_verified_at = NOW(), \
         requires_email_verification = FALSE \
         WHERE email = 'alice@example.com'",
    )
    .execute(&pool)
    .await
    .expect("update user");

    let resp = post_json(
        &app,
        "/api/v1/auth/login",
        None,
        json!({"username": "alice", "password": "password123"}),
    )
    .await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["user"]["username"], "alice");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn login_rejects_unknown_identifier_with_401(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);

    let resp = post_json(
        &app,
        "/api/v1/auth/login",
        None,
        json!({"username": "ghost@example.com", "password": "password123"}),
    )
    .await;
    assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn register_rejects_request_with_username_field(pool: PgPool) {
    // The wire format change is breaking by design — if a stale client still
    // sends `username`, Axum's deserializer rejects the unknown field. This
    // requires #[serde(deny_unknown_fields)] on RegisterRequest (added in
    // step 3).
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);
    let resp = post_json(
        &app,
        "/api/v1/auth/register",
        None,
        json!({"username": "alice", "password": "password123", "email": "alice@example.com"}),
    )
    .await;
    assert_eq!(resp.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}
```

To make the last test pass, add `#[serde(deny_unknown_fields)]` to `RegisterRequest`:

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub invite_token: Option<String>,
}
```

- [ ] **Step 7: Run the new tests**

Run: `cargo test --workspace -p llm-gateway-api auth::`
Expected: all tests pass including the four new ones.

If `login_accepts_username_identifier_for_legacy_user` fails with 401, double-check the SQL UPDATE — the password hash from `/auth/register` is what the login handler is verifying against, so as long as the UPDATE didn't touch `password`, the credentials should match.

- [ ] **Step 8: Run the full workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass. If a test outside `auth.rs` and `invitations.rs` fails because it constructs a `User { username: "..." }` or `InvitationPreview { inviter_username: "..." }`, fix it inline by wrapping in `Some(...)` (or for `InvitationPreview`, use `.to_string()` → wrap with `Some(...)`).

- [ ] **Step 9: Commit**

```bash
git add crates/api/src/auth.rs crates/gateway/src/cli.rs crates/gateway/src/main.rs
git commit -m "feat(auth): login by email or username, drop username from register"
```

---

## Task 4: Frontend — widen types + add `displayName` helper

**Files:**
- Modify: `web/src/types/index.ts` (lines 220, 293-306, 322, 783)
- Create: `web/src/lib/displayName.ts`
- Create: `web/src/lib/displayName.test.ts`

- [ ] **Step 1: Widen `User.username`, `MeResponse.username`, `InvitationPreview.inviter_username`**

In `web/src/types/index.ts`:

```typescript
export interface User {
  id: string;
  username: string | null;   // ← was: string
  platform_role: 'platform_admin' | null;
  // ...
}

export interface MeResponse {
  id: string;
  username: string | null;   // ← was: string
  // ...
}

export interface InvitationPreview {
  org_name: string;
  org_slug: string;
  role: 'member' | 'admin';
  inviter_username: string | null;   // ← was: string
  // ...
}
```

`LoginRequest.username` (line 288) stays `string` — the wire format didn't change.

`RegisterRequest.username` (line 293) is removed:

```typescript
export interface RegisterRequest {
  password: string;
  email: string;
  inviteToken?: string;
}
```

- [ ] **Step 2: Write failing test for `displayName`**

Create `web/src/lib/displayName.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { displayName } from './displayName';

describe('displayName', () => {
  it('returns the username when set', () => {
    expect(displayName({ username: 'alice', email: 'a@b.com' })).toBe('alice');
  });

  it('falls back to email when username is null', () => {
    expect(displayName({ username: null, email: 'a@b.com' })).toBe('a@b.com');
  });

  it('falls back to email when username is empty string', () => {
    // Defensive — backend may serialize Option<String> as "" in some edge case.
    // Treat empty as "unset".
    expect(displayName({ username: '', email: 'a@b.com' })).toBe('a@b.com');
  });

  it('returns empty string when both are null', () => {
    expect(displayName({ username: null, email: null })).toBe('');
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- displayName.test.ts`
Expected: FAIL (module `./displayName` doesn't exist).

- [ ] **Step 4: Implement `displayName`**

Create `web/src/lib/displayName.ts`:

```typescript
import type { User } from '../types';

/**
 * Pick the most user-friendly identifier available. Username takes priority
 * (legacy users + anyone who sets one later); email is the fallback for
 * email-only sign-ups. Returns empty string if neither is set — callers
 * should handle that case explicitly (e.g. show "Unnamed user").
 */
export function displayName(
  user: Pick<User, 'username' | 'email'>,
): string {
  if (user.username && user.username.length > 0) return user.username;
  return user.email ?? '';
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- displayName.test.ts`
Expected: PASS, 4 tests.

- [ ] **Step 6: Commit**

```bash
git add web/src/types/index.ts web/src/lib/displayName.ts web/src/lib/displayName.test.ts
git commit -m "feat(web): widen User.username to string | null, add displayName helper"
```

---

## Task 5: Frontend — Login page label change

**Files:**
- Modify: `web/src/i18n/en.json` (auth section, around line 79-80)
- Modify: `web/src/i18n/zh.json` (matching section)
- Modify: `web/src/pages/Login.tsx:113-121` (label + placeholder)
- Modify: `web/src/pages/Login.test.tsx` (placeholder text in existing tests)

- [ ] **Step 1: Add i18n key `auth.usernameOrEmail`**

In `web/src/i18n/en.json`, in the `auth` block (the one with `"signIn": "Sign In"`, around line 76-80), add:

```json
"usernameOrEmail": "Username or email",
```

In `web/src/i18n/zh.json`, in the matching block, add:

```json
"usernameOrEmail": "用户名或邮箱",
```

Keep the existing `"username"` key — it's still used elsewhere (e.g. `common.username`, the Members page header).

- [ ] **Step 2: Update Login page label and placeholder**

In `web/src/pages/Login.tsx:113-121`, change the form-control block:

```tsx
<div className="form-control">
  <label className="label"><span className="label-text font-medium">{t('auth.username')}</span></label>
  <input
    type="text"
    value={username}
    onChange={(e) => setUsername(e.target.value)}
    placeholder={t('auth.username')}
    required
    className="input input-bordered w-full"
  />
</div>
```

to:

```tsx
<div className="form-control">
  <label className="label"><span className="label-text font-medium">{t('auth.usernameOrEmail')}</span></label>
  <input
    type="text"
    value={username}
    onChange={(e) => setUsername(e.target.value)}
    placeholder={t('auth.usernameOrEmail')}
    required
    className="input input-bordered w-full"
  />
</div>
```

The local React state is still named `username` — keep it. The form field still posts as `{ username, password }`; the backend now interprets it.

- [ ] **Step 3: Update Login.test.tsx to match the new placeholder**

In `web/src/pages/Login.test.tsx`, replace `screen.getByPlaceholderText('Username')` with `screen.getByPlaceholderText('Username or email')` in all places (around 5 occurrences: lines 35, 51, 69, 99, 120, 148). Use `replace_all` if editing programmatically.

The test bodies stay identical — same flows, same assertions, just the placeholder text changed.

- [ ] **Step 4: Run the Login tests**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- Login.test.tsx`
Expected: PASS, all tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/i18n/en.json web/src/i18n/zh.json web/src/pages/Login.tsx web/src/pages/Login.test.tsx
git commit -m "feat(web): Login accepts username or email"
```

---

## Task 6: Frontend — Register page drops username field

**Files:**
- Modify: `web/src/pages/Register.tsx:15-17` (state), `:71` (validation), `:74` (register call), `:105-117` (form field)
- Modify: `web/src/pages/Register.test.tsx` (drop username from test inputs)

- [ ] **Step 1: Drop username from Register component state and submit**

In `web/src/pages/Register.tsx`:

Remove the `username` useState at line 15:
```tsx
const [username, setUsername] = useState('');
```

In the `handleSubmit` at line 71, change the validation:
```tsx
if (!username || !password || !email) return;
```
to:
```tsx
if (!password || !email) return;
```

In the `await register(...)` call at line 74, change:
```tsx
await register({ username, password, email });
```
to:
```tsx
await register({ password, email });
```

Remove the entire username form-control block at lines 105-117:
```tsx
<div className="form-control">
  <label className="label"><span className="label-text font-medium">{t('auth.username')}</span></label>
  <input
    type="text"
    value={username}
    onChange={(e) => setUsername(e.target.value)}
    placeholder={t('auth.username')}
    required
    minLength={3}
    disabled={registrationDisabled}
    className="input input-bordered w-full"
  />
</div>
```

- [ ] **Step 2: Update Register.test.tsx**

In `web/src/pages/Register.test.tsx`:

In the "renders registration form" test (around line 21-30), remove the assertion:
```tsx
expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
```
Keep the email/password/confirm assertions.

In every test that types into the form, remove the line:
```tsx
await userEvent.type(screen.getByPlaceholderText('Username'), '...');
```

In the "sends email in the register request body" test (line 103-139), update the expected request body:
```tsx
expect(capturedBody).toMatchObject({
  username: 'newuser',
  email: 'new@example.com',
  password: 'password123',
});
```
to:
```tsx
expect(capturedBody).toMatchObject({
  email: 'new@example.com',
  password: 'password123',
});
```

Also update the MSW mock response in that test to drop the `username` field from the user object (since `UserInfo.username` is now nullable, sending it as missing is fine; or send `username: null`):

```tsx
return HttpResponse.json({
  token: 'test-jwt-token',
  refresh_token: 'test-refresh-jwt-token',
  user: { id: 'user-1', username: null, platform_role: null },
  current_org: null,
  orgs: [],
});
```

- [ ] **Step 3: Run Register tests**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- Register.test.tsx`
Expected: PASS, all tests.

- [ ] **Step 4: Run the TypeScript build**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build`
Expected: no type errors.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/Register.tsx web/src/pages/Register.test.tsx
git commit -m "feat(web): drop username from Register form"
```

---

## Task 7: Frontend — apply `displayName` across UI call sites

These are the 7 spots that render `user.username` directly and need a null-safe fallback. The authStore `me.username` pass-through lines (71, 97, 122, 152, 162, 249) and AddEmailModal's `username: me.username` (line 46) need no change — type widening handles them.

**Files:**
- Modify: `web/src/components/Layout.tsx:428` (avatar), `:431` (header chip)
- Modify: `web/src/components/PlatformLayout.tsx:74` (header chip)
- Modify: `web/src/components/ImpersonationBanner.tsx:31` (banner text)
- Modify: `web/src/components/OnboardingCreateCard.tsx:33-34` (org name + slug prefill)
- Modify: `web/src/pages/Account.tsx:49` (avatar), `:53` (profile heading)

- [ ] **Step 1: Add the import in each file**

Add to each of the 5 files:

```tsx
import { displayName } from '../lib/displayName';
```

(Path is `../lib/displayName` from `components/` and `pages/`.)

- [ ] **Step 2: Replace `user?.username` with `displayName(user)` at each call site**

**`web/src/components/Layout.tsx:428`** (avatar first letter):

```tsx
<span className="text-xs font-semibold">{user?.username?.charAt(0).toUpperCase()}</span>
```

becomes:

```tsx
<span className="text-xs font-semibold">{displayName(user ?? { username: null, email: null }).charAt(0).toUpperCase() || '?'}</span>
```

Actually cleaner — `displayName` takes `Pick<User, 'username' | 'email'>`. If `user` is null (not yet loaded), pass a default:

```tsx
<span className="text-xs font-semibold">
  {(user ? displayName(user) : '').charAt(0).toUpperCase() || '?'}
</span>
```

**`web/src/components/Layout.tsx:431`** (header chip):

```tsx
<span className="hidden sm:inline text-[13px] font-medium text-base-content/60">{user?.username}</span>
```

becomes:

```tsx
<span className="hidden sm:inline text-[13px] font-medium text-base-content/60">
  {user ? displayName(user) : ''}
</span>
```

**`web/src/components/PlatformLayout.tsx:74`** (header chip):

```tsx
<div className="ml-auto text-xs text-base-content/40">{user?.username}</div>
```

becomes:

```tsx
<div className="ml-auto text-xs text-base-content/40">{user ? displayName(user) : ''}</div>
```

**`web/src/components/ImpersonationBanner.tsx:31`** (banner text):

```tsx
user: user?.username ?? '',
```

becomes:

```tsx
user: user ? displayName(user) : '',
```

**`web/src/components/OnboardingCreateCard.tsx:33-34`** (org name + slug prefill):

```tsx
const [name, setName] = useState(user?.username ?? '');
const [slug, setSlug] = useState(slugify(user?.username ?? ''));
```

becomes:

```tsx
const [name, setName] = useState(user ? displayName(user) : '');
const [slug, setSlug] = useState(slugify(user ? displayName(user) : ''));
```

**`web/src/pages/Account.tsx:49`** (avatar):

```tsx
<span className="text-2xl font-bold text-primary">{user?.username?.charAt(0).toUpperCase()}</span>
```

becomes:

```tsx
<span className="text-2xl font-bold text-primary">
  {(user ? displayName(user) : '').charAt(0).toUpperCase() || '?'}
</span>
```

**`web/src/pages/Account.tsx:53`** (profile heading):

```tsx
{user?.username}
```

becomes:

```tsx
{user ? displayName(user) : ''}
```

- [ ] **Step 3: Run TypeScript build to confirm types**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build`
Expected: no type errors.

- [ ] **Step 4: Run the full vitest suite**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run`
Expected: all tests pass. If a snapshot test for Layout/Account/etc. fails because rendered text changed, update the snapshot or adjust the test fixture.

- [ ] **Step 5: Manual smoke test**

```bash
source ~/.nvm/nvm.sh && cd web && npm run dev
```

In a browser:
1. Register a new user with email only — confirm the form has no username field, registration succeeds, lands on `/check-email`.
2. Verify the email via the dev link, then log in by typing the email into the "Username or email" field.
3. Once logged in, confirm the sidebar header chip shows the email (since username is null).
4. Confirm the Account page shows the email as the profile name + the first letter as the avatar.
5. Collapse the sidebar — confirm the chip still renders correctly.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Layout.tsx web/src/components/PlatformLayout.tsx \
        web/src/components/ImpersonationBanner.tsx web/src/components/OnboardingCreateCard.tsx \
        web/src/pages/Account.tsx
git commit -m "feat(web): use displayName helper for null-safe username rendering"
```

---

## Final verification

After all 7 tasks are merged:

- [ ] **Backend full suite**: `cargo test --workspace` — all green.
- [ ] **Frontend full suite**: `cd web && npm test -- --run` — all green.
- [ ] **Frontend build**: `cd web && npm run build` — no type errors.
- [ ] **End-to-end smoke**: register by email → verify → log in by email → log out → log in by username (legacy user) → both succeed.
- [ ] **CLI check**: `cargo run -p llm-gateway -- grant-platform-admin --username alice@example.com` resolves the user by email and prints the success message.
