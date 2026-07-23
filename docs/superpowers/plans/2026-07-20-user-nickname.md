# User `nickname` Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional `nickname` column to `users`, a self-service `POST /api/v1/auth/me/nickname` endpoint, and a new `/{slug}/profile` page so email-only users can set a friendly display name.

**Architecture:** Single nullable TEXT column on `users` (no uniqueness, no index). Backend follows the existing `set_my_email` pattern (`POST /api/v1/auth/me/nickname`, returns refreshed `MeResponse`). Frontend adds a dedicated Profile page and updates the `displayName()` helper's fallback chain to `nickname → username → email`.

**Tech Stack:** Rust (axum + sqlx + chrono), React + TypeScript + Vite + Vitest, react-hook-form + zod, React Query, i18next, Tailwind.

**Spec:** `docs/superpowers/specs/2026-07-20-user-nickname-design.md`

---

## File Structure

**Backend (Rust) — to modify:**
- `crates/storage/migrations/postgres/20260720000001_users_nickname.sql` (new)
- `crates/storage/migrations/postgres/20260720000001_users_nickname.down.sql` (new)
- `crates/storage/src/types.rs` — add `nickname` field to `User` (line 1047) and `CreateUser` (line 1065)
- `crates/storage/src/lib.rs` — add `set_user_nickname` to `Storage` trait (after `set_user_email` at line 319)
- `crates/storage/src/postgres.rs` — `PgUserRow` (line 487), `From<PgUserRow>` (line 504), 4 SELECT queries (lines 2086, 2099, 2112, ~3471), `create_user` INSERT (line 2060), `update_user` UPDATE (line 2126), new `set_user_nickname` impl
- `crates/api/src/auth.rs` — `UserInfo` (line 105), `MeResponse` (line 134), `me()` (line 788), `set_my_email` surroundings — add `SetMyNicknameRequest`, `set_my_nickname` handler, validation
- `crates/api/src/management/mod.rs` — route registration (line 76 area)
- `crates/api/src/error.rs` — new `InvalidNickname` variant
- `CHANGELOG.md` — Unreleased → Added entry

**Backend (Rust) — tests to add/extend:**
- `crates/api/src/auth.rs` test module (append) — 6 new tests
- `crates/storage/src/postgres.rs` test module — 1 new test for `set_user_nickname`

**Frontend — to modify:**
- `web/src/types/index.ts` — `User` interface: add `nickname?: string | null`
- `web/src/lib/displayName.ts` — signature + fallback chain
- `web/src/lib/displayName.test.ts` — extend with nickname-priority test
- `web/src/api/auth.ts` — `setMyNickname` function
- `web/src/hooks/useUpdateMyNickname.ts` (new) — mutation hook
- `web/src/pages/Profile.tsx` (new) — page
- `web/src/pages/Profile.test.tsx` (new) — page tests
- `web/src/App.tsx` — register `/:orgSlug/profile` route
- `web/src/components/Layout.tsx` — add Profile item in user dropdown
- `web/src/i18n/en.json` + `web/src/i18n/zh.json` — `profile.*` and `header.profile` keys

---

## Task 1: Migration + `User` struct field

**Goal:** Land the DB column and the Rust struct field; verify migrations apply cleanly.

**Files:**
- Create: `crates/storage/migrations/postgres/20260720000001_users_nickname.sql`
- Create: `crates/storage/migrations/postgres/20260720000001_users_nickname.down.sql`
- Modify: `crates/storage/src/types.rs:1046-1062` (User struct) and `crates/storage/src/types.rs:1064+` (CreateUser struct)

- [ ] **Step 1: Create the migration SQL file**

Create `crates/storage/migrations/postgres/20260720000001_users_nickname.sql`:

```sql
-- Add optional `nickname` column to users.
-- Nullable, no UNIQUE, no index: nickname is a display label, not an
-- identifier (multiple users may share a nickname). NULL means "user
-- hasn't set one" — display code falls back via displayName().
ALTER TABLE users ADD COLUMN nickname TEXT;
```

- [ ] **Step 2: Create the down migration**

Create `crates/storage/migrations/postgres/20260720000001_users_nickname.down.sql`:

```sql
ALTER TABLE users DROP COLUMN IF EXISTS nickname;
```

- [ ] **Step 3: Add `nickname` to `User` struct**

In `crates/storage/src/types.rs`, find the `User` struct (starts line 1047). After the `password_changed_at: DateTime<Utc>,` line (around line 1061), add:

```rust
    /// User-chosen friendly name. Optional — display code falls back via
    /// the frontend `displayName()` helper (nickname → username → email).
    /// Not unique; validated to 1-32 UTF-8 chars when set via the API.
    pub nickname: Option<String>,
```

- [ ] **Step 4: Add `nickname` to `CreateUser` struct**

In the same file, find `CreateUser` (line 1064+). Add `pub nickname: Option<String>,` next to the existing `username: Option<String>` field.

- [ ] **Step 5: Verify the project still builds**

Run: `cargo check -p llm-gateway-storage`
Expected: COMPILE FAILURES in postgres.rs because `PgUserRow` and `From<PgUserRow>` don't yet include `nickname`. **This is expected** — Task 2 fixes them. Do not commit yet.

- [ ] **Step 6: Do not commit yet**

This task is incomplete until Task 2 lands. Continue to Task 2.

---

## Task 2: Update postgres.rs SELECTs/INSERT/UPDATE + `PgUserRow`

**Goal:** Make `cargo check -p llm-gateway-storage` pass after the struct field add in Task 1.

**Files:**
- Modify: `crates/storage/src/postgres.rs` — `PgUserRow` (line 487), `From<PgUserRow>` (line 504), 4 SELECT queries, `create_user` INSERT, `update_user` UPDATE

- [ ] **Step 1: Add `nickname` to `PgUserRow`**

In `crates/storage/src/postgres.rs:487-502`, find `struct PgUserRow`. After `password_changed_at: chrono::DateTime<chrono::Utc>,` add:

```rust
    nickname: Option<String>,
```

- [ ] **Step 2: Update `From<PgUserRow> for User`**

In the same file at lines 504-522, find `impl From<PgUserRow> for User`. After `password_changed_at: r.password_changed_at,` add:

```rust
            nickname: r.nickname,
```

- [ ] **Step 3: Update `get_user` SELECT (line 2084)**

Find `async fn get_user`. The SELECT query at line 2086 ends with `email, email_verified_at, requires_email_verification, password_changed_at`. Change the column list to add `nickname`:

```sql
SELECT id, username, password, platform_role, current_org_id, enabled, refresh_token,
       created_at, updated_at,
       email, email_verified_at, requires_email_verification, password_changed_at,
       nickname
FROM users WHERE id = $1
```

- [ ] **Step 4: Update `get_user_by_username` SELECT (line 2097)**

Same column-list edit as Step 3, applied to the `WHERE username = $1` query.

- [ ] **Step 5: Update `get_user_by_email` SELECT (~line 3471)**

Find `async fn get_user_by_email`. Same column-list edit (note this query already uses the bare column list without `u.` prefix; just add `nickname` after `password_changed_at`).

- [ ] **Step 6: Update `list_users` SELECT (line 2112)**

This query uses `u.` prefix. Add `u.nickname` to the column list after `u.password_changed_at`.

- [ ] **Step 7: Update `create_user` INSERT (line 2059)**

Find `async fn create_user`. Update the INSERT statement to include `nickname`:

```rust
sqlx::query(
    "INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, refresh_token,
                        created_at, updated_at,
                        email, email_verified_at, requires_email_verification, password_changed_at,
                        nickname)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
)
```

Then add `.bind(&user.nickname)` after the existing `.bind(&user.password_changed_at)` (around line 2078).

- [ ] **Step 8: Update `update_user` UPDATE (line 2126)**

Find `async fn update_user`. The UPDATE at line 2128 currently sets 8 columns. Add `nickname` to the SET clause:

```rust
sqlx::query(
    "UPDATE users SET username = $1, password = $2, platform_role = $3, current_org_id = $4, enabled = $5, refresh_token = $6, password_changed_at = $7, nickname = $8, updated_at = $9 WHERE id = $10",
)
```

Renumber the existing binds and add `.bind(&user.nickname)` as `$8`. The `WHERE id = ...` bind becomes `$10`. Make sure to push `updated_at` bind before `id` bind.

- [ ] **Step 9: Verify the storage crate compiles**

Run: `cargo check -p llm-gateway-storage`
Expected: PASS (no errors).

- [ ] **Step 10: Verify the workspace compiles**

Run: `cargo check --workspace`
Expected: PASS (downstream crates don't break because `nickname` is `Option<_>`).

- [ ] **Step 11: Verify migrations apply against a clean DB**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-storage --lib postgres::tests`
Expected: existing storage tests pass (the new column allows NULL, so no fixture data needs updating).

- [ ] **Step 12: Commit**

```bash
git add crates/storage/migrations/postgres/20260720000001_users_nickname.sql \
        crates/storage/migrations/postgres/20260720000001_users_nickname.down.sql \
        crates/storage/src/types.rs \
        crates/storage/src/postgres.rs
git commit -m "feat(storage): add nickname column to users"
```

---

## Task 3: Storage trait method `set_user_nickname`

**Goal:** TDD-style: write a failing integration test, implement `set_user_nickname`, verify pass.

**Files:**
- Modify: `crates/storage/src/lib.rs:319` — add trait method after `set_user_email`
- Modify: `crates/storage/src/postgres.rs` — add impl after `set_user_email` (line ~3588)
- Modify: `crates/storage/src/postgres.rs` test module — append new test

- [ ] **Step 1: Write the failing test**

In `crates/storage/src/postgres.rs`, find the test module (search `mod tests`). Append:

```rust
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn set_user_nickname_persists_and_clears(pool: PgPool) {
    let storage = PostgresStorage::from_pool(pool);
    let user = make_test_user("nick-test", None);
    let created = storage.create_user(&user).await.unwrap();

    // Set a nickname.
    let updated = storage
        .set_user_nickname(&created.id, Some("小明🌟"))
        .await
        .unwrap();
    assert_eq!(updated.nickname.as_deref(), Some("小明🌟"));

    // Refetch via get_user — must see the same value (covers SELECT roundtrip).
    let refetched = storage.get_user(&created.id).await.unwrap().unwrap();
    assert_eq!(refetched.nickname.as_deref(), Some("小明🌟"));

    // Clear via None.
    let cleared = storage
        .set_user_nickname(&created.id, None)
        .await
        .unwrap();
    assert!(cleared.nickname.is_none());
}
```

If `make_test_user` doesn't already exist in the test module, search for an existing user-fixture helper to mirror; otherwise add a minimal one:

```rust
fn make_test_user(id: &str, nickname: Option<&str>) -> crate::types::User {
    use chrono::Utc;
    crate::types::User {
        id: id.to_string(),
        username: Some(id.to_string()),
        password: "x".to_string(),
        platform_role: None,
        current_org_id: None,
        enabled: true,
        refresh_token: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        email: None,
        email_verified_at: None,
        requires_email_verification: false,
        password_changed_at: Utc::now(),
        nickname: nickname.map(|s| s.to_string()),
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-storage --lib postgres::tests::set_user_nickname_persists_and_clears`
Expected: COMPILE ERROR — `set_user_nickname` method not found on `PostgresStorage`. This is the expected failure.

- [ ] **Step 3: Add trait method declaration**

In `crates/storage/src/lib.rs`, after the `set_user_email` declaration (line 319-325), add:

```rust
    /// Set or clear the user's nickname. Pass `None` to clear (write NULL).
    /// Storage does NOT validate length/charset — that's the API layer's
    /// job (so the rule lives in exactly one place). Returns the full
    /// updated User row.
    async fn set_user_nickname(
        &self,
        user_id: &str,
        nickname: Option<&str>,
    ) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 4: Add postgres impl**

In `crates/storage/src/postgres.rs`, after the `set_user_email` impl ends (~line 3588), add:

```rust
    async fn set_user_nickname(
        &self,
        user_id: &str,
        nickname: Option<&str>,
    ) -> Result<User, DbErr> {
        sqlx::query(
            "UPDATE users SET nickname = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(nickname)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        // Re-read for the response — same pattern as set_user_email
        // (PgUserRow shape is large enough that maintaining a separate
        // RETURNING-bind is more error-prone than refetching).
        let row = self.get_user(user_id).await?;
        row.ok_or_else(|| {
            sqlx::Error::RowNotFound
        })
    }
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-storage --lib postgres::tests::set_user_nickname_persists_and_clears`
Expected: PASS.

- [ ] **Step 6: Run the full storage test suite to check for regressions**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-storage`
Expected: PASS (no regressions).

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/postgres.rs
git commit -m "feat(storage): add set_user_nickname"
```

---

## Task 4: Surface `nickname` in `UserInfo` + `MeResponse`

**Goal:** Wire the new field through to the API response shapes. After this task, `GET /auth/me` returns a `nickname` field (null for users who haven't set one).

**Files:**
- Modify: `crates/api/src/auth.rs:105-110` (`UserInfo` struct)
- Modify: `crates/api/src/auth.rs:134-160ish` (`MeResponse` struct)
- Modify: `crates/api/src/auth.rs:788-823` (`me()` handler)
- Modify: every other site that constructs `UserInfo` or `MeResponse`

- [ ] **Step 1: Write the failing test**

In `crates/api/src/auth.rs`, find the test module. Append:

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn me_returns_nickname_field_null_for_fresh_user(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);
    let resp = post_json(
        &app,
        "/api/v1/auth/register",
        None,
        json!({"password": "password123", "email": "nick@example.com"}),
    )
    .await;
    let body: serde_json::Value = serde_json::from_slice(&resp.into_body()).await.unwrap();
    let token = body["token"].as_str().unwrap();

    let me_resp = read_json(
        &app,
        "/api/v1/auth/me",
        Some(token),
    ).await;
    assert_eq!(me_resp["nickname"], serde_json::Value::Null);
}
```

If `read_json` (or whatever GET-with-auth helper exists) is not already defined, look at the existing test module for the GET-with-bearer helper (likely named `get_json` or similar) and use it; if there is no such helper, use `app.oneshot(Request::builder().method("GET").uri(...).header("authorization", format!("Bearer {token}"))...)` directly — mirror whatever pattern the existing `me_*` tests use.

- [ ] **Step 2: Run the test to verify it fails**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-api --lib auth::tests::me_returns_nickname_field_null_for_fresh_user`
Expected: FAIL — `nickname` key missing from JSON response (or compile error if `MeResponse` doesn't have the field yet).

- [ ] **Step 3: Add `nickname` to `UserInfo`**

In `crates/api/src/auth.rs:105-110`, find `UserInfo` struct. Add `nickname`:

```rust
#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: Option<String>,
    pub platform_role: Option<String>,
    pub nickname: Option<String>,
}
```

- [ ] **Step 4: Add `nickname` to `MeResponse`**

In the same file, find `MeResponse` (line 134). Add `nickname`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub nickname: Option<String>,
```

Place it next to the existing `email` field (so the Phase 4 / post-Phase 4 fields cluster together).

- [ ] **Step 5: Populate `nickname` in `me()` handler**

In `me()` (around line 788-823), the `MeResponse { ... }` construction sets `email: user.email.clone(),` etc. Add right after `email: ...`:

```rust
        nickname: user.nickname.clone(),
```

- [ ] **Step 6: Find and fix every other `UserInfo { ... }` and `MeResponse { ... }` construction**

Run: `cargo check -p llm-gateway-api 2>&1 | grep 'missing field'`
For each compile error, add `nickname: user.nickname.clone(),` (or `nickname: None,` if no `user` is available — e.g. constructing a stub UserInfo for an error path). Likely sites:
- `register()` (builds `AuthResponse { user: UserInfo { ... } }`)
- `login()` (same)
- `refresh()` (same)
- `accept_invitation()` and any other auth-response builders
- `set_my_email()` (builds `MeResponse`)
- `switch_org()` (if it returns `MeResponse`)
- `create_org()` (returns `AuthResponse`)

Each call site has the user row available; just thread `user.nickname.clone()` through.

- [ ] **Step 7: Run the failing test**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-api --lib auth::tests::me_returns_nickname_field_null_for_fresh_user`
Expected: PASS.

- [ ] **Step 8: Run the full API test suite**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-api`
Expected: PASS (no regressions).

- [ ] **Step 9: Commit**

```bash
git add crates/api/src/auth.rs
git commit -m "feat(api): surface nickname in UserInfo and MeResponse"
```

---

## Task 5: API — `POST /auth/me/nickname` endpoint

**Goal:** Self-service endpoint for setting/clearing nickname. Includes validation.

**Files:**
- Modify: `crates/api/src/error.rs` — add `InvalidNickname` variant
- Modify: `crates/api/src/auth.rs` — add `SetMyNicknameRequest` DTO + `set_my_nickname` handler
- Modify: `crates/api/src/management/mod.rs:76` — register route

- [ ] **Step 1: Write the failing tests (6 cases)**

Append to the test module in `crates/api/src/auth.rs`:

```rust
async fn register_and_get_token(app: &axum::Router, email: &str) -> String {
    let resp = post_json(
        app,
        "/api/v1/auth/register",
        None,
        json!({"password": "password123", "email": email}),
    ).await;
    let body: serde_json::Value = serde_json::from_slice(&resp.into_body()).await.unwrap();
    body["token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_nickname_persists_and_appears_in_me(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);
    let token = register_and_get_token(&app, "nick1@example.com").await;

    let resp = post_json(
        &app,
        "/api/v1/auth/me/nickname",
        Some(&token),
        json!({"nickname": "Alice"}),
    ).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(&resp.into_body()).await.unwrap();
    assert_eq!(body["nickname"], "Alice");

    // GET /me also reflects it.
    let me_resp = read_json(&app, "/api/v1/auth/me", Some(&token)).await;
    assert_eq!(me_resp["nickname"], "Alice");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_nickname_empty_string_clears(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);
    let token = register_and_get_token(&app, "nick2@example.com").await;

    // Set first.
    post_json(&app, "/api/v1/auth/me/nickname", Some(&token),
        json!({"nickname": "Bob"})).await;
    // Then clear.
    let resp = post_json(&app, "/api/v1/auth/me/nickname", Some(&token),
        json!({"nickname": ""})).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(&resp.into_body()).await.unwrap();
    assert!(body["nickname"].is_null());
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_nickname_rejects_too_long(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);
    let token = register_and_get_token(&app, "nick3@example.com").await;

    let too_long = "x".repeat(33);
    let resp = post_json(&app, "/api/v1/auth/me/nickname", Some(&token),
        json!({"nickname": too_long})).await;
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_nickname_rejects_control_chars(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);
    let token = register_and_get_token(&app, "nick4@example.com").await;

    // U+200B (zero-width space) must be rejected.
    let resp = post_json(&app, "/api/v1/auth/me/nickname", Some(&token),
        json!({"nickname": "bad\u{200B}name"})).await;
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_nickname_rejects_unauthenticated(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);

    let resp = post_json(&app, "/api/v1/auth/me/nickname", None,
        json!({"nickname": "Anon"})).await;
    assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn set_my_nickname_accepts_emoji_and_cjk(pool: PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    let app = build_router(storage);
    let token = register_and_get_token(&app, "nick5@example.com").await;

    let resp = post_json(&app, "/api/v1/auth/me/nickname", Some(&token),
        json!({"nickname": "🌟小明"})).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(&resp.into_body()).await.unwrap();
    assert_eq!(body["nickname"], "🌟小明");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-api --lib auth::tests::set_my_nickname`
Expected: COMPILE ERROR — no route `/auth/me/nickname`, no `set_my_nickname` handler.

- [ ] **Step 3: Add `InvalidNickname` to `ApiError`**

In `crates/api/src/error.rs`, find the existing variants (`EmailRequired`, `EmailInUse`, etc.). Add a new variant:

```rust
    InvalidNickname,             // 400 invalid_nickname
```

Then in the `match self` block that maps variants to `(StatusCode, &str)` tuples, add:

```rust
    ApiError::InvalidNickname => (StatusCode::BAD_REQUEST, "invalid_nickname"),
```

If the file has a separate `error_body` / `response` match, mirror what `EmailMismatchRegister` does and add the matching arm there too.

- [ ] **Step 4: Add `SetMyNicknameRequest` DTO**

In `crates/api/src/auth.rs`, near `SetMyEmailRequest` (around line 197-200), add:

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetMyNicknameRequest {
    /// Empty string is the explicit "clear" signal (writes NULL).
    pub nickname: String,
}
```

- [ ] **Step 5: Add `validate_nickname` helper**

Near other validation helpers in `auth.rs` (search `fn validate_email`), add:

```rust
/// Returns the trimmed nickname to persist, or `None` if the input clears it.
/// Returns `Err(())` on validation failure (too long, or contains control /
/// zero-width chars). The caller maps `Err(())` to `ApiError::InvalidNickname`.
fn validate_nickname(input: &str) -> Result<Option<String>, ()> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    // 1-32 chars after trim. chars().count() counts Unicode scalar values
    // (not grapheme clusters) — good enough for our purposes and matches
    // what most platforms do.
    if trimmed.chars().count() > 32 {
        return Err(());
    }
    // Reject control chars and zero-width chars. Allows emoji and CJK as-is.
    for c in trimmed.chars() {
        if c.is_control() {
            return Err(());
        }
        // U+200B (ZWSP), U+200C (ZWNJ), U+200D (ZWJ), U+FEFF (BOM/ZWNBSP).
        if matches!(c, '\u{200B}'..='\u{200D}' | '\u{FEFF}') {
            return Err(());
        }
    }
    Ok(Some(trimmed.to_string()))
}
```

- [ ] **Step 6: Add `set_my_nickname` handler**

In `crates/api/src/auth.rs`, place it right after `set_my_email` ends (around line 940):

```rust
/// POST /api/v1/auth/me/nickname — authenticated.
///
/// Sets or clears the user's nickname. Empty string after trim = clear
/// (writes NULL). Validation: 1-32 UTF-8 chars after trim, no control /
/// zero-width chars. Returns the refreshed MeResponse (same shape as
/// `me()` and `set_my_email()`).
pub async fn set_my_nickname(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<SetMyNicknameRequest>,
) -> Result<Json<MeResponse>, ApiError> {
    let nickname = validate_nickname(&input.nickname).map_err(|_| ApiError::InvalidNickname)?;

    let claims = require_auth(&headers, &state.jwt_secret)?;
    let user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    let updated = state
        .storage
        .set_user_nickname(&user.id, nickname.as_deref())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build the fresh MeResponse — mirror `set_my_email`'s pattern.
    let (current_org, orgs) = current_membership(&state, &updated).await?;
    let impersonating = match &current_org {
        Some(org) => state
            .storage
            .get_member(&updated.id, &org.id)
            .await
            .map_err(|e| ApiError::Internal(format!("member lookup failed: {e}")))?
            .map(|m| m.created_by.as_deref() == Some("system"))
            .unwrap_or(false),
        None => false,
    };

    Ok(Json(MeResponse {
        id: updated.id,
        username: updated.username,
        platform_role: updated.platform_role.as_ref().map(|p| p.as_str().to_string()),
        current_org,
        orgs,
        allow_registration: get_allow_registration(&state).await,
        impersonating,
        email: updated.email.clone(),
        email_verified_at: updated.email_verified_at.map(|t| t.to_rfc3339()),
        requires_email_verification: updated.requires_email_verification,
        nickname: updated.nickname.clone(),
    }))
}
```

- [ ] **Step 7: Register the route**

In `crates/api/src/management/mod.rs` at line 76 (next to the existing `.route("/api/v1/auth/me/email", post(auth::set_my_email))` line), add:

```rust
.route("/api/v1/auth/me/nickname", post(auth::set_my_nickname))
```

- [ ] **Step 8: Run the failing tests**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-api --lib auth::tests::set_my_nickname`
Expected: all 6 tests PASS.

- [ ] **Step 9: Run the full API test suite**

Run: `DATABASE_URL=postgres://test:Xabc12345@localhost/postgres cargo test -p llm-gateway-api`
Expected: PASS (no regressions).

- [ ] **Step 10: Commit**

```bash
git add crates/api/src/auth.rs crates/api/src/error.rs crates/api/src/management/mod.rs
git commit -m "feat(api): POST /auth/me/nickname endpoint"
```

---

## Task 6: Frontend types + `displayName()` helper

**Goal:** Update TypeScript types and the displayName helper to know about `nickname`. All existing call sites must keep compiling.

**Files:**
- Modify: `web/src/types/index.ts` — add `nickname` to `User`
- Modify: `web/src/lib/displayName.ts` — change signature + fallback chain
- Modify: `web/src/lib/displayName.test.ts` — extend tests
- Modify: all 16 call sites flagged by TypeScript

- [ ] **Step 1: Extend `displayName.test.ts` with the nickname-priority test (failing first)**

In `web/src/lib/displayName.test.ts`, append:

```typescript
import { displayName } from './displayName';

describe('displayName', () => {
  it('prefers nickname over username and email', () => {
    expect(displayName({ nickname: 'Alice', username: 'alice123', email: 'a@x.com' })).toBe('Alice');
  });
  it('falls back to username when nickname is null', () => {
    expect(displayName({ nickname: null, username: 'alice123', email: 'a@x.com' })).toBe('alice123');
  });
  it('falls back to email when nickname and username are both null', () => {
    expect(displayName({ nickname: null, username: null, email: 'a@x.com' })).toBe('a@x.com');
  });
  it('returns empty string when all three are missing', () => {
    expect(displayName({ nickname: null, username: null, email: null })).toBe('');
  });
  it('prefers nickname even when username is empty string', () => {
    expect(displayName({ nickname: 'Bob', username: '', email: 'b@x.com' })).toBe('Bob');
  });
});
```

If the file already has tests, merge the new cases in (don't duplicate the import or `describe`).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run src/lib/displayName.test.ts`
Expected: FAIL — TypeScript error because `displayName` doesn't know about `nickname`.

- [ ] **Step 3: Add `nickname` to `User` type**

In `web/src/types/index.ts`, find the `User` interface. Add:

```typescript
  nickname?: string | null;
```

- [ ] **Step 4: Update `displayName` helper**

In `web/src/lib/displayName.ts`, replace the entire file content:

```typescript
import type { User } from '../types';

/**
 * Pick the most user-friendly identifier available. Priority:
 *   nickname → username → email → ""
 *
 * `nickname` is the user-chosen friendly name (set via /profile).
 * `username` is set by legacy users (email-auth made it optional).
 * `email` is the always-present fallback for email-only sign-ups.
 *
 * Callers should handle the empty-string case explicitly (e.g. show
 * "Unnamed user").
 */
export function displayName(
  user: Pick<User, 'nickname' | 'username' | 'email'>,
): string {
  if (user.nickname && user.nickname.length > 0) return user.nickname;
  if (user.username && user.username.length > 0) return user.username;
  return user.email ?? '';
}
```

- [ ] **Step 5: Fix all TypeScript compile errors at call sites**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit 2>&1 | grep 'displayName'`

For each error, the fix is mechanical: the call site passes a `User`-shaped object — passing the whole user (which now includes `nickname`) will work because TypeScript picks up the new field automatically. If a call site constructs a partial object explicitly (e.g. `displayName({ username: x.username, email: x.email })`), add `nickname: x.nickname` to that object literal.

Iterate until `npx tsc --noEmit` passes.

- [ ] **Step 6: Run displayName tests**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run src/lib/displayName.test.ts`
Expected: PASS.

- [ ] **Step 7: Run the full frontend test suite to check for regressions**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add web/src/types/index.ts web/src/lib/displayName.ts web/src/lib/displayName.test.ts
# Also stage any other frontend files touched by the call-site fix-up in step 5:
git add $(git diff --name-only web/src/ | grep -E '\.(ts|tsx)$')
git commit -m "feat(web): prefer nickname in displayName helper"
```

---

## Task 7: Frontend API client + `useUpdateMyNickname` hook

**Goal:** Add the API call + React Query mutation hook used by the Profile page.

**Files:**
- Modify: `web/src/api/auth.ts` (or wherever `setMyEmail` lives) — add `setMyNickname`
- Create: `web/src/hooks/useUpdateMyNickname.ts`

- [ ] **Step 1: Locate the existing `setMyEmail` API function**

Run: `grep -rn 'setMyEmail\|/auth/me/email' web/src/api/`
This tells you which file to add `setMyNickname` to. Likely `web/src/api/auth.ts`.

- [ ] **Step 2: Add `setMyNickname` API function**

In the file from Step 1, near `setMyEmail`, add:

```typescript
/**
 * Set or clear the current user's nickname. Pass empty string to clear
 * (server writes NULL). Returns the refreshed MeResponse.
 */
export async function setMyNickname(nickname: string): Promise<MeResponse> {
  const { data } = await apiClient.post<MeResponse>('/auth/me/nickname', { nickname });
  return data;
}
```

If `MeResponse` isn't already imported as a type, find where the existing `/auth/me` response type is defined and reuse it (the type may be called `Me`, `User`, or similar). Mirror whatever type `setMyEmail` returns.

- [ ] **Step 3: Add `useUpdateMyNickname` hook**

Create `web/src/hooks/useUpdateMyNickname.ts`:

```typescript
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { setMyNickname } from '../api/auth';
import { useAuthStore } from '../stores/authStore';

/**
 * Mutation: set or clear the current user's nickname. On success:
 *   - Invalidates `/auth/me` so useAuthStore refetches
 *   - Updates the local user state optimistically (the response carries
 *     the new nickname, so we don't have to wait for the refetch).
 */
export function useUpdateMyNickname() {
  const qc = useQueryClient();
  const setUser = useAuthStore((s) => s.setUser);

  return useMutation({
    mutationFn: (nickname: string) => setMyNickname(nickname),
    onSuccess: (data) => {
      // data is the refreshed MeResponse; it includes the new nickname.
      setUser((prev) => (prev ? { ...prev, nickname: data.nickname ?? null } : prev));
      qc.invalidateQueries({ queryKey: ['auth', 'me'] });
    },
  });
}
```

Check that `useAuthStore` exposes `setUser` (or equivalent). If the actual API is different (e.g. `updateUser` patch instead of setter), mirror the existing pattern used elsewhere — look at how `useUpdateMyEmail` or similar hooks update the auth store.

- [ ] **Step 4: Verify TypeScript compiles**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/api/auth.ts web/src/hooks/useUpdateMyNickname.ts
git commit -m "feat(web): setMyNickname API + useUpdateMyNickname hook"
```

---

## Task 8: i18n keys + Profile page + route + menu

**Goal:** Build the user-facing entry point. After this task, a logged-in user can navigate to `/{slug}/profile`, edit their nickname, and see it reflected across the UI.

**Files:**
- Modify: `web/src/i18n/en.json` — add `profile.*` + `header.profile`
- Modify: `web/src/i18n/zh.json` — same keys, Chinese
- Create: `web/src/pages/Profile.tsx`
- Create: `web/src/pages/Profile.test.tsx`
- Modify: `web/src/App.tsx` — register route
- Modify: `web/src/components/Layout.tsx` — add Profile item in user dropdown

- [ ] **Step 1: Add i18n keys to `en.json`**

In `web/src/i18n/en.json`, find the `header` section (it has `logout` etc.). Add a `profile` key to `header`:

```json
"profile": "Profile",
```

Then at the top level, add a new `profile` section (next to existing sections like `account`, `settings`):

```json
"profile": {
  "title": "Profile",
  "nickname": "Nickname",
  "nicknameHint": "Shown wherever you appear in the UI. 1–32 characters.",
  "nicknamePlaceholder": "Your nickname",
  "save": "Save",
  "savedShort": "Nickname updated",
  "clearedShort": "Nickname cleared",
  "invalidTooLong": "Nickname must be 1–32 characters",
  "invalidControlChars": "Nickname contains invalid characters",
  "username": "Username",
  "email": "Email"
},
```

- [ ] **Step 2: Add the same keys to `zh.json` with Chinese translations**

In `web/src/i18n/zh.json`, mirror the structure:

```json
"profile": "个人资料",
```

in `header`, and at top level:

```json
"profile": {
  "title": "个人资料",
  "nickname": "昵称",
  "nicknameHint": "在界面中显示你的称呼,1–32 字符。",
  "nicknamePlaceholder": "你的昵称",
  "save": "保存",
  "savedShort": "昵称已更新",
  "clearedShort": "昵称已清除",
  "invalidTooLong": "昵称需在 1–32 字符之间",
  "invalidControlChars": "昵称包含无效字符",
  "username": "用户名",
  "email": "邮箱"
},
```

- [ ] **Step 3: Write the failing Profile page test**

Create `web/src/pages/Profile.test.tsx`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import Profile from './Profile';

describe('Profile page', () => {
  beforeEach(() => {
    // Seed an authenticated user with no nickname.
    window.localStorage.setItem('auth_token', 'test-token');
    useAuthStore.setState({
      user: { id: 'u1', username: null, email: 'me@x.com', nickname: null, platform_role: null },
      currentOrg: { id: 'o1', slug: 'o1', name: 'Org', role: 'owner' },
    });
  });

  it('renders current nickname/username/email', async () => {
    renderWithProviders(<Profile />);
    expect(await screen.findByDisplayValue('')).toBeInTheDocument(); // empty nickname input
    expect(screen.getByText('me@x.com')).toBeInTheDocument();
  });

  it('saves nickname and shows success toast', async () => {
    server.use(
      http.post('/api/v1/auth/me/nickname', async ({ request }) => {
        const body = await request.json() as { nickname: string };
        return HttpResponse.json({
          id: 'u1', username: null, platform_role: null, nickname: body.nickname,
          current_org: null, orgs: [], allow_registration: true, impersonating: false,
        });
      }),
    );

    renderWithProviders(<Profile />);
    const input = await screen.findByPlaceholderText('Your nickname');
    await userEvent.type(input, 'Alice');
    await userEvent.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => {
      expect(screen.getByText('Nickname updated')).toBeInTheDocument();
    });
  });

  it('rejects over-length input client-side', async () => {
    renderWithProviders(<Profile />);
    const input = await screen.findByPlaceholderText('Your nickname');
    await userEvent.type(input, 'x'.repeat(33));
    await userEvent.click(screen.getByRole('button', { name: /save/i }));

    expect(await screen.findByText(/must be 1–32 characters/i)).toBeInTheDocument();
  });
});
```

If `renderWithProviders` isn't the right helper name, check `web/src/test/` for the actual export. Mirror existing page tests (e.g. `Members.test.tsx`) for setup style.

- [ ] **Step 4: Run the test to verify it fails**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run src/pages/Profile.test.tsx`
Expected: FAIL — `Profile` module doesn't exist.

- [ ] **Step 5: Create the Profile page**

Create `web/src/pages/Profile.tsx`:

```tsx
import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { toast } from 'sonner';
import { useAuthStore } from '../stores/authStore';
import { useUpdateMyNickname } from '../hooks/useUpdateMyNickname';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';

const EASE = [0.16, 1, 0.3, 1] as const;

const NICKNAME_MAX = 32;

function validateNickname(raw: string): string | null | Error {
  const trimmed = raw.trim();
  if (trimmed.length === 0) return null; // clear
  if ([...trimmed].length > NICKNAME_MAX) {
    return new Error('too_long');
  }
  // Reject ASCII control chars (0x00-0x1F), DEL (0x7F), C1 control chars
  // (0x80-0x9F), zero-width chars (U+200B-200D, U+FEFF). for...of iterates
  // Unicode code points so emoji are treated as single chars (allowed).
  for (const c of trimmed) {
    const code = c.codePointAt(0)!;
    if (code < 0x20 || code === 0x7f || (code >= 0x80 && code <= 0x9f)) {
      return new Error('control');
    }
    if (code >= 0x200b && code <= 0x200d) return new Error('control');
    if (code === 0xfeff) return new Error('control');
  }
  return trimmed;
}

export default function Profile() {
  const { t } = useTranslation();
  const user = useAuthStore((s) => s.user);
  const updateNickname = useUpdateMyNickname();

  const [input, setInput] = useState(user?.nickname ?? '');
  const [errorKey, setErrorKey] = useState<string | null>(null);

  // Re-sync when the store updates (e.g. after a successful save).
  useEffect(() => {
    setInput(user?.nickname ?? '');
  }, [user?.nickname]);

  const handleSave = () => {
    const validated = validateNickname(input);
    if (validated instanceof Error) {
      setErrorKey(validated.message === 'too_long' ? 'invalidTooLong' : 'invalidControlChars');
      return;
    }
    setErrorKey(null);
    updateNickname.mutate(validated ?? '', {
      onSuccess: (_data, vars) => {
        toast.success(vars.trim().length === 0 ? t('profile.clearedShort') : t('profile.savedShort'));
      },
      onError: () => {
        toast.error(t('profile.invalidControlChars'));
      },
    });
  };

  return (
    <motion.div
      className="px-6 pb-8 max-w-2xl"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: EASE }}
    >
      <h1 className="text-2xl font-semibold mb-6">{t('profile.title')}</h1>

      <section className="space-y-4 rounded-lg border border-base-300 bg-base-100 p-6">
        <div>
          <label className="block text-sm font-medium mb-1">{t('profile.nickname')}</label>
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={t('profile.nicknamePlaceholder')}
            maxLength={NICKNAME_MAX * 4} // UTF-8 byte headroom; real check is on chars
            className="w-full rounded-md border border-base-300 bg-base-100 px-3 py-2 text-base"
          />
          <p className="text-xs text-base-content/50 mt-1">{t('profile.nicknameHint')}</p>
        </div>

        {errorKey && (
          <Alert variant="error">{t(`profile.${errorKey}`)}</Alert>
        )}

        <div className="flex justify-end">
          <Button onClick={handleSave} disabled={updateNickname.isPending}>
            {t('profile.save')}
          </Button>
        </div>
      </section>

      <section className="mt-6 space-y-3 rounded-lg border border-base-300 bg-base-100 p-6">
        <div>
          <div className="text-xs text-base-content/50">{t('profile.username')}</div>
          <div className="text-sm">{user?.username || '—'}</div>
        </div>
        <div>
          <div className="text-xs text-base-content/50">{t('profile.email')}</div>
          <div className="text-sm">{user?.email || '—'}</div>
        </div>
      </section>
    </motion.div>
  );
}
```

If `Alert`'s `variant="error"` API differs, mirror whatever existing pages use. Same for `Button`'s `disabled` prop.

- [ ] **Step 6: Register the route in App.tsx**

In `web/src/App.tsx`, find the routes block. There's an `<Route element={<OrgRouteGuard />}>` wrapper. Inside it, add:

```tsx
<Route path="profile" element={<Profile />} />
```

(Without the leading `/:orgSlug/` — the OrgRouteGuard wrapper handles the org prefix.)

Also add `import Profile from './pages/Profile';` to the imports at the top.

- [ ] **Step 7: Add Profile menu item to user dropdown in Layout.tsx**

In `web/src/components/Layout.tsx`, find the user dropdown (around lines 440-460). It currently has a Logout button. Above the Logout button, add a Profile button:

```tsx
<button
  onClick={() => {
    setDropdownOpen(false);
    navigate(slug ? `/${slug}/profile` : '/login');
  }}
  className="block w-full text-left px-4 py-2 text-sm text-base-content/80 hover:bg-base-200"
>
  {t('header.profile')}
</button>
```

If `navigate` (react-router) isn't already imported in Layout.tsx, add `import { useNavigate } from 'react-router-dom';` and `const navigate = useNavigate();` to the component.

- [ ] **Step 8: Run the Profile page test**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run src/pages/Profile.test.tsx`
Expected: PASS (3 tests).

- [ ] **Step 9: Run the full frontend test suite**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run`
Expected: PASS (no regressions).

- [ ] **Step 10: Run TypeScript check + production build**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build`
Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add web/src/i18n/en.json web/src/i18n/zh.json web/src/pages/Profile.tsx \
        web/src/pages/Profile.test.tsx web/src/App.tsx web/src/components/Layout.tsx
git commit -m "feat(web): add Profile page for nickname editing"
```

---

## Task 9: CHANGELOG entry

**Goal:** Document the change for the next release.

**Files:**
- Modify: `CHANGELOG.md` — Unreleased → Added section

- [ ] **Step 1: Add CHANGELOG entry**

In `CHANGELOG.md`, find the `## [Unreleased]` section, then `### Added` (or `### Added — Platform admin bootstrap & management` — pick the most recent feature group, or create a new `### Added` if no general one exists). Add:

```markdown
- **User nickname field** + `POST /api/v1/auth/me/nickname` endpoint + new `/{slug}/profile` page. Optional display name; NULL by default for existing rows. Non-unique, validated to 1–32 UTF-8 chars after trim (empty = clear). The frontend `displayName()` helper falls back `nickname → username → email`.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: changelog entry for nickname field"
```

---

## Verification (final smoke test)

After all 9 tasks are merged:

- [ ] Restart the backend with the new binary; verify existing users can log in without errors
- [ ] Log in as an existing user → navigate to `/{slug}/profile` → set nickname → see it reflected in the user dropdown immediately
- [ ] Verify nickname persists across logout/login
- [ ] Verify the nickname appears in the Members table (other members can see it via `displayName()`)
- [ ] Verify setting nickname to empty string clears it
- [ ] Verify 33-char input is rejected client-side AND server-side (try via curl with bypass)
- [ ] Verify emoji + CJK input works (`🌟小明`)
- [ ] Verify existing users have `nickname: null` in the DB (no auto-backfill):

```bash
psql ... -c 'SELECT id, username, nickname FROM users LIMIT 5;'
```

---

## Out of scope (do NOT add)

- Avatar / profile picture upload
- Bio / "about me" free-text
- Locale / language preference
- Per-membership (per-org) nickname — nickname is global to the user
- Audit-log entry for nickname changes
- Admin endpoints to set another user's nickname
- Nickname in registration form or onboarding wizard
- Auto-backfill of nickname from username/email for existing rows
