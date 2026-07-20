# User `nickname` Field — Design Spec

**Date:** 2026-07-20
**Status:** Draft (awaiting user review)
**Tracks:** develop (v2.0.0-track)

## Motivation

The recently-shipped email-auth feature (`2026-07-13-email-auth-design.md`) made `users.username` nullable and dropped `username` from the registration wire format. New users register with `{email, password}` and have `username = NULL`. The frontend `displayName()` helper (`web/src/lib/displayName.ts`) falls back to `username → email → ""` for display purposes.

This leaves email-only users with no friendly display name. Email addresses are long, privacy-sensitive, and awkward in compact UI (sidebar avatars, audit log actor, members table). We need a user-settable friendly-name field that:

- Is **optional** (no friction at registration)
- Is **non-unique** (multiple users may share a nickname; it's a label, not an identifier)
- Supports international content (CJK, emoji) — the userbase is Chinese-speaking
- Defaults to unset for existing rows (no auto-derivation from email/username — the user didn't choose it)

The project convention is to call this `nickname`, not `display_name` (see project memory: `nickname-not-display-name.md`).

## Scope

**In scope:**
- New `nickname TEXT` column on `users` (nullable, no constraints)
- New `POST /api/v1/auth/me/nickname` endpoint (self-service update only)
- `nickname` surfaced in `UserInfo` (i.e. everywhere `User` info is serialized)
- New `/{slug}/profile` page with nickname edit form + read-only email/username
- Menu entry under the existing user dropdown
- `displayName()` helper updated to fall back `nickname → username → email`
- Tests (Rust integration + frontend Vitest)
- CHANGELOG entry

**Out of scope (YAGNI):**
- Registration form change — registration stays `{email, password}`
- Onboarding wizard change — onboarding stays focused on org creation
- Admin-forcing another user's nickname
- Audit-logging nickname changes (not identity-critical like username/password)
- Avatar, bio, locale preferences (future Profile page growth)
- Backfill of existing rows (NULL is the correct starting state)

## Decisions

| Decision | Value | Rationale |
|---|---|---|
| Field name | `nickname` | Project convention; user preference |
| Column constraints | `TEXT`, nullable, no UNIQUE, no index | Display-only field, duplicates allowed |
| Backfill existing rows | `NULL` | User didn't choose; `displayName()` falls back gracefully |
| Validation | trim → 1–32 UTF-8 chars; control/zero-width chars rejected; empty after trim = clear (write NULL) | Matches Discord/Twitter convention; supports CJK + emoji |
| API endpoint | `POST /api/v1/auth/me/nickname` | Mirrors existing `POST /api/v1/auth/me/email` pattern |
| Request body | `{ "nickname": string }` (single string; empty string = clear) | Simpler than optional field; handler normalizes empty → NULL |
| Frontend entry | New `/{slug}/profile` page + entry in user dropdown menu | Separates self-service from org/admin settings; room to grow |
| Route guard | `OrgRouteGuard` (any logged-in user, no admin required) | Self-service feature |

## Architecture

### Data model

Single column added to `users`:

```sql
ALTER TABLE users ADD COLUMN nickname TEXT;
```

No constraints. NULL means "user hasn't set a nickname" — display code falls back via `displayName()`.

The `users_username_or_email_required` CHECK constraint (added in `20260713000001_users_username_optional.sql`) is unaffected — it only requires `(username IS NOT NULL OR email IS NOT NULL)`, and `nickname` is independent.

### Layer-by-layer changes

#### Storage (Rust)

**Migration files (new):**
- `crates/storage/migrations/postgres/20260720000001_users_nickname.sql`:
  ```sql
  ALTER TABLE users ADD COLUMN nickname TEXT;
  ```
- `crates/storage/migrations/postgres/20260720000001_users_nickname.down.sql`:
  ```sql
  ALTER TABLE users DROP COLUMN IF EXISTS nickname;
  ```

**`crates/storage/src/types.rs`:**
- `User` struct: add `pub nickname: Option<String>`

**`crates/storage/src/lib.rs`:**
- `Storage` trait: add
  ```rust
  async fn set_user_nickname(&self, user_id: &str, nickname: Option<&str>) -> Result<User, StorageError>;
  ```

**`crates/storage/src/postgres.rs`:**
- 4 SELECT sites that build a `User` row to update (column list): `get_user`, `get_user_by_username`, `get_user_by_email`, `list_users`
- 1 INSERT site (`create_user`): add `nickname` to column list + bind
- 1 UPDATE site (`update_user`): add `nickname = $N` to the SET clause + bind (for symmetry; the dedicated `set_user_nickname` is the primary write path)
- `PgUserRow` struct: add `nickname: Option<String>` field
- `From<PgUserRow> for User` impl: map the new field
- Implement `set_user_nickname`:
  ```rust
  UPDATE users SET nickname = $1, updated_at = NOW() WHERE id = $2 RETURNING ...
  ```
  Returns the full updated `User` row (reuse the existing SELECT-after-update pattern, or use `RETURNING *` with the same column list).

#### API (Rust)

**`crates/api/src/auth.rs`:**
- `UserInfo` struct: add `pub nickname: Option<String>` (serialized as `null` when unset)
- New request DTO:
  ```rust
  #[derive(Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct SetMyNicknameRequest {
      pub nickname: String,  // empty string = clear (write NULL)
  }
  ```
- New error variant `ApiError::InvalidNickname` → 400 with `{"error": {"type": "invalid_nickname", "message": "..."}}`
- New handler `set_my_nickname(State, headers, Json<SetMyNicknameRequest>) -> Result<Json<MeResponse>, ApiError>`:
  1. `require_auth(&headers, &state.jwt_secret)?` → claims
  2. Validate:
     - `let trimmed = body.nickname.trim()`
     - If `trimmed.is_empty()`: target nickname = `None`
     - Else: check `trimmed.chars().count() <= 32`; reject if longer
     - Reject if any char matches `c.is_control() || matches!(c, '\u{200B}'..='\u{200D}' | '\u{FEFF}')`
     - On any validation failure: return `InvalidNickname`
  3. `state.storage.set_user_nickname(&claims.sub, target_nickname).await?`
  4. Re-fetch the data `me()` normally returns (memberships, current org, etc.) and return `MeResponse`

**`crates/api/src/management/mod.rs`:**
- Register route:
  ```rust
  .route("/api/v1/auth/me/nickname", post(auth::set_my_nickname))
  ```

#### Frontend

**`web/src/types/index.ts`:**
- `User` interface: add `nickname?: string | null`

**`web/src/lib/displayName.ts`:**
- Update signature: `Pick<User, 'nickname' | 'username' | 'email'>`
- New fallback chain: `nickname → username → email → ""`
- All call sites must pass `nickname` (TypeScript will flag them)

**`web/src/api/auth.ts`** (or new `web/src/api/me.ts`):
- `setMyNickname(nickname: string): Promise<UserInfo>` — POST `/auth/me/nickname` with `{ nickname }`

**`web/src/hooks/useUpdateMyNickname.ts`** (new):
- `useMutation` wrapper; on success, invalidate `/auth/me` query so `useAuthStore` refetches

**`web/src/pages/Profile.tsx`** (new, ~80 lines):
- Top section: form with one input (nickname) + Save button
  - Prefill with current `user.nickname ?? ""`
  - Client-side validation mirroring backend (1–32 chars, no control chars)
  - On Save: call mutation; on success: toast `t('profile.savedShort')`
- Bottom section: read-only display of `username` and `email`

**`web/src/App.tsx`:**
- Add route inside `OrgRouteGuard` (not `RequireAdmin`):
  ```tsx
  <Route path="profile" element={<Profile />} />
  ```

**`web/src/components/Layout.tsx`:**
- User dropdown (around line 440-460): add a "Profile" button above "Logout" that navigates to `/${slug}/profile`

**`web/src/i18n/en.json` + `zh.json`:**
- New keys:
  - `header.profile` — "Profile" / "个人资料"
  - `profile.title` — "Profile" / "个人资料"
  - `profile.nickname` — "Nickname" / "昵称"
  - `profile.nicknameHint` — "Shown wherever you appear in the UI. 1–32 characters." / "在界面中显示你的称呼,1–32 字符"
  - `profile.nicknamePlaceholder` — "Your nickname" / "你的昵称"
  - `profile.save` — "Save" / "保存"
  - `profile.savedShort` — "Nickname updated" / "昵称已更新"
  - `profile.clearedShort` — "Nickname cleared" / "昵称已清除"
  - `profile.invalidTooLong` — "Nickname must be 1–32 characters" / "昵称需在 1–32 字符之间"
  - `profile.invalidControlChars` — "Nickname contains invalid characters" / "昵称包含无效字符"
  - `profile.username` — "Username" / "用户名"
  - `profile.email` — "Email" / "邮箱"

### Error handling

| Failure | HTTP | Body |
|---|---|---|
| Not authenticated | 401 | standard `unauthorized` |
| Nickname > 32 chars (after trim) | 400 | `invalid_nickname` |
| Nickname contains control char or zero-width char | 400 | `invalid_nickname` |
| DB write fails | 500 | standard `internal_error` |

Empty string after trim is **not** an error — it's the explicit "clear" signal and writes `NULL`.

### Testing

**Rust integration tests** (appended to `crates/api/src/auth.rs` test module):
- `set_my_nickname_persists_and_appears_in_me` — register → POST nickname → GET `/auth/me` sees it
- `set_my_nickname_empty_string_clears_existing` — set → empty string → GET `/auth/me` shows `nickname: null`
- `set_my_nickname_rejects_too_long` — 33-char string → 400 `invalid_nickname`
- `set_my_nickname_rejects_control_chars` — contains `\u{200B}` → 400 `invalid_nickname`
- `set_my_nickname_rejects_unauthenticated` — no bearer → 401
- `set_my_nickname_accepts_emoji_and_cjk` — `"🌟小明"` (4 chars) → 200, persisted

**Frontend tests:**
- `web/src/pages/Profile.test.tsx` (new):
  - renders current nickname/username/email
  - successful save shows toast and updates store
  - over-length input shows client-side error, does not POST
- `web/src/lib/displayName.test.ts` (extend existing):
  - prefers `nickname` over `username` and `email`
  - falls back through to email when nickname and username are both empty

## Migration notes

- The migration is purely additive (one new nullable column). No data movement. No risk to existing flows.
- The `displayName()` helper signature change is **breaking at the TypeScript level** — all call sites must add `nickname` to the `Pick`. The TS compiler will flag every one; current count is 16 call sites.
- The `UserInfo` schema change is **additive on the wire** — old clients ignoring the new `nickname` field are unaffected. No FE/BE version skew concern (unlike the email-auth rollout).

## Open questions

None — all locked-in decisions are documented above.

## Out of scope (explicitly)

- Avatar / profile picture upload
- Bio / "about me" free-text
- Locale / language preference on user row
- Per-org nickname (nickname is global to the user, not per-membership)
- Audit-log entry for nickname changes
- Admin endpoints to set another user's nickname
- Nickname in registration form or onboarding wizard
