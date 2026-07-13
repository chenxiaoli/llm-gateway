# Email Auth Migration — Design

## Goal

Drop the requirement that every user has a `username`. Login accepts either a username or an email in a single identifier field. Registration collects email only — no username field.

## Why

Today, username is the only hard-required identifier on `users`. Email landed later (Phase 4) as an optional column. That forces every new signup to invent a username even though email alone uniquely identifies the user. Login already has a single "username" input on the UI — it should accept email too, since that's what new users will know.

The DB schema for `members` is already correct: it has no `email` column at all (only `user_id, org_id, role, group_id, created_at, created_by`). Email lives on `users` where it belongs. This migration does **not** touch the `members` table.

## Non-goals

- **Not changing** the `members` table, `MemberResponse`, or the frontend `Member` type. Email stays joined onto the member payload at read time via the existing SQL — denormalization through JOIN is fine, denormalization through duplication in the DB is not.
- **Not changing** the `/auth/login` request payload shape — the field stays named `username` for backward compatibility; only its meaning widens to "username or email".
- **Not backfilling** or migrating existing users — every existing row already has a username, so the new CHECK constraint passes on day one.
- **Not adding** a `display_name` field anywhere. Display logic stays on the frontend where it already is.

## Current state

### DB

`crates/storage/migrations/postgres/20260415000000_initial.sql:119-129` — `users.username TEXT NOT NULL UNIQUE`.

`crates/storage/migrations/postgres/20260711000001_users_email_fields.sql` — adds nullable `email`, `email_verified_at`, `requires_email_verification`, `password_changed_at`, plus the partial unique index `users_email_unique_idx ON users (LOWER(email)) WHERE email IS NOT NULL`.

### Backend

- `crates/api/src/auth.rs` login handler resolves the user via `storage.get_user_by_username(&input.username)` — email is not accepted.
- Register handler requires `username` in the payload (validated as non-empty).
- `crates/gateway/src/cli.rs` `GrantPlatformAdmin` subcommand takes `username: String` as a positional arg.

### Frontend

- `web/src/pages/Login.tsx` — single input labeled "Username" (or i18n equivalent), posts `{ username, password }`.
- `web/src/pages/Register.tsx` — has a username field plus email + password.
- `web/src/types/index.ts` — `User.username: string`, `MeResponse.username: string`, `Member.email: string` (sourced from `MemberResponse.email`).
- `web/src/lib/displayName.ts` (if present) already falls back to email — or if not, will be added as a small helper for the cases where `username` is now null.

## Design

### 1. Schema & migration

New migration `crates/storage/migrations/postgres/20260713000001_users_username_optional.sql`:

```sql
ALTER TABLE users ALTER COLUMN username DROP NOT NULL;

ALTER TABLE users ADD CONSTRAINT users_username_or_email_required
    CHECK (username IS NOT NULL OR email IS NOT NULL);
```

The existing partial unique index on `LOWER(email)` continues to work — no change there. `username` stays `UNIQUE` (duplicates were never allowed and still aren't).

A matching SQLite migration for the test/dev path is not needed — SQLite was dropped per `2026-05-03-drop-sqlite-design.md`. Postgres only.

**Rollback safety**: dropping the NOT NULL is permissive — existing rows pass, queries that select `username` still work (they just may see NULL). The CHECK constraint is also satisfied by every existing row because all of them have a username.

### 2. Backend

#### Login (`crates/api/src/auth.rs`, around the current `get_user_by_username` call)

Resolve the user by inspecting the identifier:

```rust
let user = if input.username.contains('@') {
    storage.get_user_by_email(&input.username).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
} else {
    storage.get_user_by_username(&input.username).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}
.ok_or_else(|| ApiError::Unauthorized("invalid credentials".into()))?;
```

`get_user_by_email` already exists (`crates/storage/src/postgres.rs:3446`). No new storage code needed.

The request struct stays `LoginRequest { username: String, password: String }` — same JSON shape, same field name. Only the resolver branch is new.

#### Register (`crates/api/src/auth.rs`)

`RegisterRequest` loses `username`. The handler now:

1. Validates email format + uniqueness (already done today).
2. Inserts a new `users` row with `username = NULL`, `email = Some(...)`, `requires_email_verification = TRUE`.
3. Triggers the existing Phase 4 email-verification flow.

Existing legacy users keep their usernames. New sign-ups have none until they optionally add one via a future profile page (out of scope here).

#### CLI (`crates/gateway/src/cli.rs`)

`GrantPlatformAdmin { username: String }` keeps the positional arg name on the CLI for compatibility, but the handler resolves the target user with the same `'@'` branch as login:

```rust
let user = if cmd.username.contains('@') {
    storage.get_user_by_email(&cmd.username).await?
} else {
    storage.get_user_by_username(&cmd.username).await?
};
```

Help text updated to "username or email".

### 3. Frontend

#### `web/src/pages/Login.tsx`

Single input label changes from "Username" to "Username or email" (`i18n: login.usernameOrEmail`). The `name="username"`, the form state, and the POST body all stay identical — the backend now accepts either form in that field.

#### `web/src/pages/Register.tsx`

Remove the username field entirely. Form state and submit body become `{ email, password }` only. The page already enforces email format + email uniqueness via existing validation; that doesn't change.

#### `web/src/types/index.ts`

```ts
interface User {
  id: string;
  username: string | null;   // was: string
  email: string | null;
  // ...rest unchanged
}

interface MeResponse {
  // ...
  username: string | null;   // was: string
  // ...
}

interface InvitationPreview {
  // ...
  inviter_username: string | null;   // was: string
}
```

`Member.email` stays as-is (still sourced from `MemberResponse.email`).

#### `web/src/lib/displayName.ts`

Helper for the cases where `username` is null:

```ts
import type { User } from '../types';

export function displayName(user: Pick<User, 'username' | 'email'>): string {
  return user.username ?? user.email ?? '';
}
```

Used at any call site that previously did `user.username` directly and now needs to handle the null case (sidebar, header avatar, audit log actor, invitations list, etc.). The exact call sites get enumerated in the implementation plan; the helper exists so the fallback rule lives in one place.

### 4. Tests

#### Backend

- **Migration**: insert with `username=NULL, email='a@b.com'` → OK; both NULL → CHECK violation; both set → OK.
- **Login**: identifier `'a@b.com'` resolves via `get_user_by_email`; identifier `'alice'` resolves via `get_user_by_username`; unknown identifier → 401 with no leak about which lookup ran.
- **Register**: payload without `username` succeeds and creates a row with `username IS NULL`; payload with stale `username` field is rejected (request schema tightened).
- **CLI**: `grant-platform-admin a@b.com` resolves by email; bare username still resolves.

#### Frontend

- **Login**: label renders as "Username or email"; submitting `'a@b.com'` posts the same payload as before; success still routes to dashboard.
- **Register**: no username field visible; missing email shows validation error; successful submit posts only `{ email, password }`.
- **`displayName`**: returns `username` when set, falls back to `email`, returns `''` when both null.
- **Audit log / invitation list / sidebar**: where `username` was rendered directly, the test asserts that a null-username user falls back to email-derived display.

### 5. Rollout & safety

- **Migration is permissive**: every existing row passes both the dropped NOT NULL and the new CHECK. No backfill, no data movement.
- **Login backward compat**: existing clients posting `{ username: "alice", password }` keep working. Clients posting `{ username: "a@b.com", password }` start working.
- **Register is a breaking change**: any client still sending `username` in the register payload gets a 400. We accept this because the only client is our own frontend, shipped in the same release.
- **No member-table changes** — `MemberResponse.email` keeps being joined from `users.email` at read time. Existing API consumers see no shape change on `/members`.
- **Risk on rollback**: re-adding `NOT NULL` to `username` would fail if any null-username rows exist. Mitigation: if we need to roll back, first run `UPDATE users SET username = split_part(email, '@', 1) WHERE username IS NULL;` to backfill, then re-add NOT NULL.

## Open questions for the implementation plan

These are deferred to the writing-plans phase, not blockers for this spec:

- Exact list of frontend call sites that render `username` directly and need to switch to `displayName(user)`.
- Whether to add a "Profile" page so email-only users can set a username later (out of scope for this spec; future work).
