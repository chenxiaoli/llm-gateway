# SaaS Phase 4: Email + Email-Bound Invitations

**Date**: 2026-07-09
**Status**: Design
**Depends on**: Phase 3 (`docs/superpowers/specs/2026-07-08-saas-phase3-invitations-design.md`), Phase 2 (`2026-07-07-saas-multi-tenant-orgs-design.md`)

## Overview

Phase 4 adds a transactional email subsystem to the gateway and tightens the invitation flow so that each invitation is bound to a specific recipient email. Three user-facing capabilities land together:

1. **Email verification on signup** — new users must verify ownership of their email before they can log in.
2. **Email-bound invitations** — invitations are minted with a specific `recipient_email`; only a user with that email (and a verified email) can accept. The invitation email is delivered automatically; admins no longer need to copy/paste links.
3. **Password reset** — self-service "forgot password" flow for any user with a verified email.

A fourth, smaller piece closes the loop for pre-Phase-4 users:

4. **Existing-user migration** — users without an email are prompted (via a dismissable banner) to add one. They keep full access in the meantime; only email-dependent features are unavailable until they verify.

## Theme and scope decisions

| Question | Decision |
|---|---|
| Phase 4 theme | Email + email-bound invites (over per-org depth, soft delete + janitor, or "something else"). |
| Identity model | Email optional for existing users, required for new users. |
| Feature scope | Full transactional: invitation emails + password reset + email verification on signup. |
| SMTP delivery | Self-hosted via `lettre`, configurable SMTP via `config.toml`. File transport for dev. |
| Invitation model | Email-bound replaces generic. Old pending invitations are revoked by the migration. |

## 1. Architecture

```
                 ┌─────────────────────────────────────────────┐
                 │                Axum handlers                 │
                 │  auth.rs · invitations.rs (modified)         │
                 └──────────────┬──────────────────────────────┘
                                │  spawns
                                ▼
                  ┌──────────────────────────────┐
                  │     email dispatch task       │
                  │  (tokio::spawn + retry)       │
                  └──────────────┬───────────────┘
                                 │
                                 ▼
            ┌────────────────────────────────────────────┐
            │              crates/email                  │
            │   Mailer trait                              │
            │   ├─ SmtpMailer (lettre + Tokio1Executor)  │
            │   ├─ FileMailer  (lettre FileTransport)    │
            │   └─ NoopMailer  (tests)                   │
            └────────────┬───────────────────────────────┘
                         │
                         ▼
            ┌────────────────────────────────────────────┐
            │       Handlebars templates                 │
            │  verification · invitation · password_reset│
            │  (.html.hbs + .txt.hbs per template)       │
            └────────────────────────────────────────────┘
```

The `Mailer` is injected into `AppState` alongside the existing `Pool`, `Nats` publisher, etc. Handlers that need to send email call `state.mailer.send(...)` inside a `tokio::spawn` so the HTTP response isn't blocked. The spawn retries up to 3 times with exponential backoff (1s/2s/4s); on total failure it logs an error and writes an audit row. The user can always re-request via the appropriate UI (resend verification, resend password reset, re-mint invitation).

## 2. Schema changes

All migrations live under `crates/storage/migrations/postgres/`.

### 2.1 `users` table

```sql
ALTER TABLE users
    ADD COLUMN email                      TEXT,
    ADD COLUMN email_verified_at          TIMESTAMPTZ,
    ADD COLUMN requires_email_verification BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN password_changed_at        TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE UNIQUE INDEX users_email_unique_idx
    ON users (LOWER(email))
    WHERE email IS NOT NULL;
```

- `email` is `NULL` for pre-Phase-4 users; required for new registrations.
- `email_verified_at` is `NULL` until the user clicks the verification link.
- `requires_email_verification` distinguishes "new signup that must verify before login" (`TRUE`) from "existing user who added an email post-hoc" (`FALSE`). Register endpoint sets it to `TRUE`; the `POST /auth/me/email` endpoint does not flip it.
- `password_changed_at` starts at `NOW()` for existing users (so existing refresh tokens remain valid). Updated on every successful password reset.
- The partial unique index on `LOWER(email)` enforces case-insensitive uniqueness among users with an email. The `WHERE email IS NOT NULL` clause keeps the index small and lets legacy users coexist.

### 2.2 `invitations` table

```sql
ALTER TABLE invitations
    ADD COLUMN recipient_email TEXT;

-- Data migration FIRST: revoke all pending Phase 3 invitations (no
-- recipient_email). Must run before the CHECK constraint is added or the
-- ADD CONSTRAINT would reject the existing rows.
UPDATE invitations
SET revoked_at = NOW()
WHERE accepted_at IS NULL
  AND revoked_at IS NULL
  AND recipient_email IS NULL;

ALTER TABLE invitations
    ADD CONSTRAINT invitations_pending_need_recipient
    CHECK (
        accepted_at IS NOT NULL
        OR revoked_at IS NOT NULL
        OR recipient_email IS NOT NULL
    );
```

The `CHECK` constraint enforces "going forward, every pending invitation must have a recipient_email". Accepted/revoked rows are grandfathered (the constraint allows them through the `OR` arms). The data migration revokes the existing pending rows so the constraint can be added cleanly.

### 2.3 `email_verifications` table

```sql
CREATE TABLE email_verifications (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token        TEXT NOT NULL UNIQUE,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email        TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    CONSTRAINT email_verifications_expires_after_created
        CHECK (expires_at > created_at),
    CONSTRAINT email_verifications_consumed_after_created
        CHECK (consumed_at IS NULL OR consumed_at > created_at)
);

CREATE INDEX email_verifications_user_idx
    ON email_verifications (user_id)
    WHERE consumed_at IS NULL;
```

- `email` is denormalized from `users.email` so a verification row records *what* was being verified, not just *who* requested it. If we ever support email changes, this becomes important.
- Partial index on `user_id WHERE consumed_at IS NULL` keeps the "active verifications for this user" lookup fast.

### 2.4 `password_resets` table

```sql
CREATE TABLE password_resets (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token        TEXT NOT NULL UNIQUE,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    CONSTRAINT password_resets_expires_after_created
        CHECK (expires_at > created_at),
    CONSTRAINT password_resets_consumed_after_created
        CHECK (consumed_at IS NULL OR consumed_at > created_at)
);

CREATE INDEX password_resets_user_idx
    ON password_resets (user_id)
    WHERE consumed_at IS NULL;
```

Same shape as `email_verifications`. 1-hour expiry (vs. 24-hour for verifications) since password resets are higher-stakes.

### 2.5 Notes on TEXT vs. UUID

Consistent with Phase 2/3: `users.id` and `orgs.id` are `TEXT` (server-generated, see `20260415000000_initial.sql` and `20260708000000_saas_orgs.sql`), so FK columns referencing them are `TEXT`. The PKs of `email_verifications` and `password_resets` are `UUID` (server-generated via `gen_random_uuid()`), matching the `invitations` table from Phase 3. The `sqlx` `uuid` feature is **not** enabled workspace-wide, so all UUID columns are read via `id::text` casts in queries.

## 3. Email subsystem

### 3.1 Crate layout

New crate `crates/email/`:

```
crates/email/
├── Cargo.toml
└── src/
    ├── lib.rs              # Mailer trait, EmailMessage, EmailError
    ├── smtp.rs             # SmtpMailer (lettre SmtpTransport + Tokio1Executor)
    ├── file.rs             # FileMailer (lettre FileTransport, dev only)
    ├── noop.rs             # NoopMailer (tests)
    ├── templates.rs        # TemplateRegistry: loads + renders Handlebars
    └── templates/
        ├── verification.html.hbs
        ├── verification.txt.hbs
        ├── invitation.html.hbs
        ├── invitation.txt.hbs
        ├── password_reset.html.hbs
        └── password_reset.txt.hbs
```

### 3.2 Public API

```rust
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

pub trait Mailer: Send + Sync {
    fn send(&self, msg: EmailMessage)
        -> impl Future<Output = Result<(), EmailError>> + Send;
}

pub struct TemplateRegistry { /* handlebars::Handlebars wrapped */ }

impl TemplateRegistry {
    pub fn load() -> Result<Self, EmailError>;  // compile-time include_str! of templates
    pub fn render_verification(&self, ctx: VerificationCtx) -> Result<EmailMessage, EmailError>;
    pub fn render_invitation(&self, ctx: InvitationCtx) -> Result<EmailMessage, EmailError>;
    pub fn render_password_reset(&self, ctx: PasswordResetCtx) -> Result<EmailMessage, EmailError>;
}
```

### 3.3 Implementations

- **`SmtpMailer`**: wraps `lettre::AsyncSmtpTransport<Tokio1Executor>`. Configurable host, port, username, password, TLS mode. Built once at startup from `[email]` config and stored in `Arc` inside `AppState`.
- **`FileMailer`**: wraps `lettre::AsyncFileTransport`. Writes each message as an `.eml` file under `file_output_dir` (default `./dev-emails/`). Used in dev and in tests; the filename includes a timestamp + recipient so tests can grep for the token.
- **`NoopMailer`**: discards everything, returns `Ok(())`. Used in unit tests that exercise handler logic without caring about delivery.

### 3.4 Config additions

```toml
[email]
transport = "smtp"          # "smtp" | "file" | "noop"
from_address = "noreply@example.com"
from_name = "LLM Gateway"
file_output_dir = "./dev-emails"   # used when transport = "file"

# SMTP-specific (used when transport = "smtp")
smtp_host = "smtp.example.com"
smtp_port = 587
smtp_username = "apikey"
smtp_password = "..."
smtp_use_tls = true         # STARTTLS when true, plain when false
```

A `[email]` section is **required** in production (validated at startup — boot fails fast if `transport = "smtp"` and `smtp_host` is missing). For dev, `[email] transport = "file"` is the default in the bootstrap config.

### 3.5 URL construction

Tokens are placed in the URL path, not query string (cleaner logs, no accidentally-leaked tokens in Referer headers):

| Flow | URL |
|---|---|
| Email verification | `{public_base_url}/verify-email/{token}` |
| Password reset | `{public_base_url}/reset-password/{token}` |
| Invitation accept | `{public_base_url}/accept-invite/{token}` |

`public_base_url` is reused from Phase 3 (`AppState::public_base_url`, default `http://localhost:5173`).

### 3.6 Background dispatch

```rust
let mailer = state.mailer.clone();
let msg = state.templates.render_verification(ctx)?;
tokio::spawn(async move {
    let mut backoff = Duration::from_secs(1);
    for attempt in 0..3 {
        match mailer.send(msg.clone()).await {
            Ok(()) => return,
            Err(e) if attempt == 2 => {
                tracing::error!(recipient = %msg.to, error = ?e, "verification email delivery failed");
                // audit row
                return;
            }
            Err(e) => {
                tracing::warn!(attempt, error = ?e, "verification email send failed, retrying");
                tokio::time::sleep(backoff).await;
                backoff *= 2;
            }
        }
    }
});
```

Three attempts, exponential backoff (1s → 2s → 4s). Total failure logs an error and writes an audit row; the user can re-request via the UI.

### 3.7 Template variables

```typescript
// verification.html.hbs / .txt.hbs
{
  username: string,
  verification_url: string,
  expires_in_hours: number,  // 24
  public_base_url: string,
}

// invitation.html.hbs / .txt.hbs
{
  org_name: string,
  inviter_username: string,
  role: 'member' | 'admin',
  recipient_email: string,
  accept_url: string,
  expires_in_days: number,   // 7
  public_base_url: string,
}

// password_reset.html.hbs / .txt.hbs
{
  username: string,
  reset_url: string,
  expires_in_hours: number,  // 1
  public_base_url: string,
}
```

## 4. Email verification on signup

### 4.1 Flow

1. `/signup` form gains a required `email` field (RFC 5322 lite validation, case-normalized to `LOWER(email)` on the server before insert).
2. `POST /api/v1/auth/register` with `{username, password, email, inviteToken?}`:
   - Rejects `400 email_required` if email missing or malformed.
   - Rejects `409 email_in_use` if the normalized email already exists.
   - If `inviteToken` present: looks up the invitation, rejects `400 email_mismatch` if `register.email != invitation.recipient_email`. Otherwise creates user + accepts invitation in one transaction.
   - Creates user with `email`, `requires_email_verification = TRUE`, `email_verified_at = NULL`.
   - Mints 24-hour `email_verifications` token, dispatches verification email in background.
   - Returns `AuthResponse` for shape compatibility with Phase 3 — but the client deliberately discards the tokens (see Section 9.2) and redirects to `/check-email`. Issuing tokens here would be useless because login is gated on verification (Section 4.3). We keep the response shape to avoid a breaking change for any non-browser caller; the tokens simply won't work until the user verifies.
3. Frontend redirects to `/check-email`.
4. User clicks the link → `/verify-email/{token}` → frontend POSTs to `/api/v1/auth/verify-email` with `{token}`.
5. Backend: `SELECT … FOR UPDATE` on token, check `expires_at > NOW()` and `consumed_at IS NULL`, then in one transaction: set `users.email_verified_at = NOW()` and `email_verifications.consumed_at = NOW()`. Returns `204`.
6. Frontend shows "Email verified ✓" and redirects to `/login`.

### 4.2 Token model

- 32-byte random via `OsRng`, base64url-no-pad (same generator as invitation tokens — share the helper).
- `expires_at = NOW() + 24 hours`.
- `SELECT … FOR UPDATE` on verify so concurrent clicks can't both succeed.
- Resend mints a fresh token; old tokens are **not** revoked. A click on a stale (expired) token returns `410 verification_expired` with a "click here to resend" hint.

### 4.3 Login gate

`POST /api/v1/auth/login` returns `403 email_not_verified` when:

```
user.email IS NOT NULL
  AND user.email_verified_at IS NULL
  AND user.requires_email_verification = TRUE
```

Pre-Phase-4 users (`email IS NULL`) and existing users who added an email post-hoc (`requires_email_verification = FALSE`) are not gated.

### 4.4 Resend

`POST /api/v1/auth/resend-verification` with `{email}` (anonymous-OK so a user who closed the tab can recover):

- Look up user by `LOWER(email)`.
- If user doesn't exist → `204` anyway (no enumeration).
- If user exists and already verified → `204` anyway (no email sent, no error).
- If user exists and unverified → mint fresh 24-hour token, dispatch verification email.
- Rate-limited: 5/hour per IP.

## 5. Email-bound invitations

### 5.1 Mint flow

`POST /{org}/invitations` body:

```json
{
  "recipient_email": "alice@example.com",
  "role": "member"
}
```

- `recipient_email` is **required** (was optional in Phase 3).
- `role` excludes `owner` (same as Phase 3).
- Backend validates email format, does **not** check whether the email corresponds to an existing user (keeps mint agnostic).
- Row written to `invitations` with `recipient_email`. Same 32-byte token, same 7-day expiry, same partial index.
- Dispatches invitation email in background via `Mailer`.

### 5.2 Preview

`GET /invitations/preview?token=X` response gains `recipient_email`:

```json
{
  "org_name": "...",
  "org_slug": "...",
  "role": "member",
  "expires_at": "...",
  "state": "pending",
  "recipient_email": "alice@example.com"
}
```

### 5.3 Accept flow — new user via register

1. User clicks invitation link → `/accept-invite/{token}` → frontend shows preview including `recipient_email`.
2. User clicks "Accept & sign up" → navigates to `/signup?inviteToken={token}`. Signup form pre-fills and **locks** the email field from `invitation.recipient_email`.
3. `POST /api/v1/auth/register` with `{username, password, email, inviteToken}`:
   - Looks up invitation, rejects `400 email_mismatch` if `register.email != invitation.recipient_email`.
   - In one transaction: create user (with `requires_email_verification = TRUE`) + accept invitation (insert membership, set `accepted_at` / `accepted_by`) + mint verification token.
   - Dispatches verification email.
4. User verifies email → logs in → already in their org. No second accept step.

### 5.4 Accept flow — existing user via `/invitations/accept`

`POST /invitations/accept` with `{token}` (auth required):

- Checks `user.email IS NOT NULL AND email_verified_at IS NOT NULL`. If not → `403 email_verification_required` ("verify your email first").
- Checks `user.email == invitation.recipient_email`. If not → `403 email_mismatch` ("this invitation was sent to a different address").
- Otherwise: same transaction as Phase 3 (insert membership, set `accepted_at` / `accepted_by`).

### 5.5 Enumeration considerations

- The mint endpoint is admin-only — no public surface for "does this email have an invitation?" probing.
- The accept endpoint's `email_mismatch` error is only returned to authenticated users, so it doesn't leak emails.

### 5.6 Copy-link fallback

The admin UI keeps a copy-link button on each pending invitation. Clicking the link still works — the `recipient_email` check happens at accept time, not preview. If an admin reuses one invitation across multiple people, the second accepter hits `email_mismatch`.

## 6. Password reset

### 6.1 Flow

1. `/forgot-password` form (public) → `POST /api/v1/auth/password-reset/request` with `{email}`.
2. Backend looks up user by `LOWER(email)`:
   - If user doesn't exist → `204` anyway (no enumeration).
   - If user exists but `email_verified_at IS NULL` → `204` anyway (no email sent, silently).
   - If user exists and verified → mint 1-hour `password_resets` token, dispatch email.
3. User clicks link → `/reset-password/{token}` (public route).
4. Frontend on mount: `GET /api/v1/auth/password-reset/preview?token=X`. Returns `{valid, expires_at}` or `404`.
5. If invalid/expired: show "link expired" + link to `/forgot-password`. If valid: show new-password form.
6. `POST /api/v1/auth/password-reset/confirm` with `{token, new_password}`.
7. Backend: `SELECT … FOR UPDATE` on token, check `expires_at > NOW()` and `consumed_at IS NULL`, then in one transaction: set `consumed_at = NOW()`, update `users.password_hash`, update `users.password_changed_at = NOW()`.
8. Return `204`. Frontend redirects to `/login` with a "password updated" toast.

### 6.2 Session invalidation

`password_changed_at` is checked on refresh: if the refresh token's `iat` claim is less than `user.password_changed_at`, reject with `401` and force re-login. Short-lived access tokens (~15 min) expire on their own; only refresh tokens need this check.

### 6.3 Rate limits

- `/password-reset/request`: 5/hour per IP, 3/hour per email.

### 6.4 UX details

- `/forgot-password` link appears on `/login` below the password field.
- After request, show "if an account exists for that email, we've sent a reset link" regardless of outcome.
- After confirm, the user is **not** auto-logged-in. They go to `/login` and authenticate with the new password.

### 6.5 Edge cases

- User clicks an older reset link from a previous email after resetting → `410 reset_consumed` with redirect to `/forgot-password`.
- User has no email (pre-Phase-4 user) → can't use this flow. They're prompted to add an email first (Section 7).

## 7. Existing-user migration

### 7.1 Banner

A persistent dismissable banner renders on all authenticated pages when `user.email == null`. Renders above the page content, below `ImpersonationBanner` if present. Copy: "Add an email to receive invitations and reset your password." Two buttons: "Add email" (opens `AddEmailModal`) and "Dismiss" (per-session flag in `useAuthStore`). Never shown once `email_verified_at` is set.

### 7.2 Add-email endpoint

`POST /api/v1/auth/me/email` with `{email}` (authenticated):

- Validate email format.
- Check uniqueness (partial unique index from Section 2). Return `409 email_in_use` on collision.
- Set `users.email`.
- Mint 24-hour verification token, dispatch verification email.
- Does **not** flip `requires_email_verification` — user can keep logging in with their existing password in the meantime.
- Returns updated `MeResponse`.

### 7.3 Verification

User clicks the verification link → same `/verify-email/{token}` flow as Section 4. On success, `email_verified_at` is set and all email-dependent features unlock for that user.

### 7.4 What's not blocked

Existing users without email can still create API keys, use the gateway, manage orgs, etc. Only email-dependent features (accept email-bound invitation, password reset, receive notifications) are unavailable.

## 8. API surface (delta from Phase 3)

| Method | Path | Auth | Body / Query | Response | Change |
|---|---|---|---|---|---|
| POST | `/api/v1/auth/register` | none | `{username, password, email, inviteToken?}` | `AuthResponse` | **changed**: `email` required; if `inviteToken` present, runs accept in same txn |
| POST | `/api/v1/auth/login` | none | `{username, password}` | `AuthResponse` or `403` | **changed**: gate on unverified email |
| POST | `/api/v1/auth/verify-email` | none | `{token}` | `204` or `410`/`404` | **new** |
| POST | `/api/v1/auth/resend-verification` | none | `{email}` | `204` always | **new** |
| POST | `/api/v1/auth/password-reset/request` | none | `{email}` | `204` always | **new** |
| GET | `/api/v1/auth/password-reset/preview` | none | `?token=X` | `{valid, expires_at}` or `404` | **new** |
| POST | `/api/v1/auth/password-reset/confirm` | none | `{token, new_password}` | `204` or `410` | **new** |
| POST | `/api/v1/auth/me/email` | bearer | `{email}` | `MeResponse` | **new**: set email for existing user |
| GET | `/api/v1/auth/me` | bearer | — | `MeResponse` | **changed**: response gains `email`, `email_verified_at`, `requires_email_verification` |
| POST | `/{org}/invitations` | admin | `{recipient_email, role}` | `Invitation` | **changed**: `recipient_email` required; dispatches email |
| GET | `/invitations/preview` | none | `?token=X` | `InvitationPreview` | **changed**: response gains `recipient_email` |
| POST | `/invitations/accept` | bearer optional | `{token}` | `AuthResponse` | **changed**: enforces email match |

All gateway endpoints (`/v1/chat/completions`, `/v1/messages`) and all other management endpoints are unchanged.

### 8.1 Response shape changes

```typescript
interface User {
  id: string;
  username: string;
  platform_role: string;
  email: string | null;             // NEW
  email_verified_at: string | null; // NEW
}

interface MeResponse {
  // ...existing fields...
  email: string | null;
  email_verified_at: string | null;
  requires_email_verification: boolean;
}

interface InvitationPreview {
  // ...existing fields...
  recipient_email: string;          // NEW
}

interface CreateInvitationBody {
  recipient_email: string;          // CHANGED from optional to required
  role: 'member' | 'admin';
}
```

### 8.2 Error codes

All errors are JSON: `{error: "<code>", message: "..."}`.

| Code | HTTP | When |
|---|---|---|
| `email_required` | 400 | Register without email |
| `email_in_use` | 409 | Register or `/me/email` with duplicate email |
| `email_mismatch` | 400 | Register via invite where email ≠ recipient_email |
| `email_mismatch` | 403 | Accept invite where logged-in user's email ≠ recipient_email |
| `email_not_verified` | 403 | Login attempt with `requires_email_verification=TRUE` and `email_verified_at IS NULL` |
| `email_verification_required` | 403 | Accept invite while logged-in user's email is unverified |
| `verification_expired` | 410 | Verify-email with expired or already-consumed token |
| `reset_expired` | 410 | Password-reset confirm with expired token |
| `reset_consumed` | 410 | Password-reset confirm with already-used token |

### 8.3 Rate limits

- `POST /auth/resend-verification`: 5/hour per IP.
- `POST /auth/password-reset/request`: 5/hour per IP, 3/hour per email.
- Other auth endpoints: unchanged from existing limits.

## 9. Frontend routes and pages

### 9.1 New public routes

| Route | Component | Behavior |
|---|---|---|
| `/verify-email/:token` | `VerifyEmail.tsx` | States: `loading` → `ok` \| `expired` \| `error`. On mount POSTs verify endpoint. On `ok`, shows success + button to `/login`. On `410`, shows "link expired" + link to `/login`. |
| `/check-email` | `CheckEmail.tsx` | Interstitial after `/signup`. Shows "We sent a verification email to {email}". Buttons: "Resend email" (POSTs `resend-verification`), "Go to login". |
| `/forgot-password` | `ForgotPassword.tsx` | Single email field. On submit POSTs `password-reset/request`. Always shows same message regardless of outcome. |
| `/reset-password/:token` | `ResetPassword.tsx` | On mount GETs preview. If invalid, shows "link expired". If valid, shows new-password form. On submit POSTs confirm. On `204`, redirects to `/login` with toast. |

### 9.2 Changed existing routes

- **`/signup`** (`Register.tsx`): gains required `email` field. If navigated from `/accept-invite/:token`, email is pre-filled and **locked** from `invitation.recipient_email` (with a "email locked by invitation" hint). After successful register, redirects to `/check-email` instead of auto-logging-in.
- **`/login`** (`Login.tsx`): catches `403 email_not_verified`. Inline message replaces form error: "Please verify your email before logging in." Two actions: "Resend verification email" (small modal → POSTs `resend-verification`) and "Use a different account" (clears form).
- **`/accept-invite/:token`** (`AcceptInvite.tsx`): preview gains `recipient_email` display. Branches:
  - Logged out: "Accept & sign up" → `/signup?inviteToken=…` (email pre-filled). "I already have an account" → `/login?next=/accept-invite/:token`.
  - Logged in, email matches and verified: "Accept & Join" → POSTs `/invitations/accept`. On success, redirect to `/{orgSlug}`.
  - Logged in, email mismatch: "This invitation was sent to a different address. Your account uses {user.email}."
  - Logged in, email not verified: "Verify your email first." Link to `/check-email`.
- **`/:orgSlug/admin/invitations`** (`Invitations.tsx`): form gains required `recipient_email` field. After submit: toast "Invitation sent to {email}". List gains a `recipient_email` column. Copy-link button remains with tooltip.

### 9.3 New components

- **`EmailBanner.tsx`**: persistent banner shown on all authenticated pages when `user.email == null`. Renders above page content, below `ImpersonationBanner` if present. Copy: "Add an email to receive invitations and reset your password." Buttons: "Add email" (opens `AddEmailModal`), "Dismiss" (per-session flag in `useAuthStore`). Never shown once `email_verified_at` is set.
- **`AddEmailModal.tsx`**: controlled by parent. Single email field. Submit → `POST /auth/me/email`. On `409 email_in_use`, inline error. On success, toast + close modal.

### 9.4 App shell wiring (`App.tsx`)

- New routes added to the public-route group: `/verify-email/:token`, `/check-email`, `/forgot-password`, `/reset-password/:token`.
- `EmailBanner` mounted inside the authenticated layout (one place, renders on every authed page).

### 9.5 Tests

**Vitest**:
- `VerifyEmail.test.tsx` (3 states)
- `CheckEmail.test.tsx` (resend + redirect)
- `ForgotPassword.test.tsx` (submit + 204-always)
- `ResetPassword.test.tsx` (valid + expired + success)
- `EmailBanner.test.tsx` (shown when no email, hidden when verified, dismiss flow)
- `AddEmailModal.test.tsx` (success + 409)

**Updated Vitest**:
- `Register.test.tsx` (email field validation)
- `Login.test.tsx` (403 handling)
- `AcceptInvite.test.tsx` (recipient_email + branches)
- `Invitations.test.tsx` (email field + new column)

**Playwright e2e**:
- `email-verification.spec.ts` (full signup → verify → login using `FileMailer` output)
- `password-reset.spec.ts`
- `email-bound-invitation.spec.ts`

All e2e tests use the dev `FileMailer` transport so the token can be read out of the generated `.eml` file.

## 10. Out of scope / future work

1. **Email change for verified users**. Phase 4 only adds email to users who don't have one.
2. **Passwordless / magic-link login**. Email-bound login would build on this infrastructure.
3. **Admin bulk email import**. Each user adds their own via the banner.
4. **Email template customization per-org**. Global template only.
5. **Delivery analytics** (opens, clicks, bounces, complaints). If we move to SES later, SNS webhook integration for auto-suppression becomes worth adding.
6. **Rich HTML emails** (logos, dark-mode-aware inline CSS). Phase 4 templates are functional and minimal.
7. **Multi-language emails**. English-only.
8. **Eager session revocation on password reset**. Refresh tokens are invalidated via `password_changed_at`; access tokens remain valid until natural expiry.
9. **Resend invitation button**. Admin can revoke + re-mint, but no one-click "resend email for this pending invitation". Trivial follow-up: `POST /{org}/invitations/{id}/resend`.
10. **Expiration warning emails**. No "your invitation expires in 24h" nudge.
11. **DKIM signing in-app**. DKIM/SPF setup is the deployer's responsibility.
12. **Configurable rate limits**. The new rate limits are constants. Making them config is a small follow-up.
13. **Audit entries for email events**. Mailer failures are audited; successful dispatches/verifications are not.

## 11. Phasing

One feature branch (`feature/saas-phase4-email`) with logical commit boundaries per batch. The batches share state — `requires_email_verification` is set by Batch 2 and read by Batch 5; `password_changed_at` is set by Batch 4 and checked by the refresh endpoint touched in Batch 2's login work. Splitting into five feature branches would create merge conflicts and intermediate states that don't compile.

### Batch 1: Foundation (no user-visible change)

- Schema migration: `users`, `invitations`, `email_verifications`, `password_resets` (4 migration files + 1 for the invitations data migration).
- `crates/email`: `Mailer` trait + `SmtpMailer` + `FileMailer` + `NoopMailer`.
- `config.toml` `[email]` section + `AppState` wiring.
- Handlebars templates: verification, invitation, password_reset (html + txt variants).
- Tests: mailer dispatch round-trips (file transport), template rendering snapshots.

### Batch 2: Email verification on signup

- `register`: email required + `requires_email_verification=TRUE` + mint+dispatch token.
- `verify-email` endpoint (`SELECT FOR UPDATE`, set `email_verified_at` + `consumed_at`).
- `resend-verification` endpoint (anonymous, 204-always, rate-limited).
- `login` gate: `403 email_not_verified`.
- Frontend: `/signup` email field, `/verify-email/:token` route, `/check-email` route, `/login` 403 handling.
- Tests: full register → verify → login happy path; expired token; resend; duplicate email 409.

### Batch 3: Email-bound invitations

- `create_invitation`: `recipient_email` required + dispatch invitation email.
- `preview`: return `recipient_email`.
- `accept`: enforce email match (both register-with-token and POST `/accept` paths).
- Migration: revoke pending Phase 3 invitations (in same migration as schema).
- Frontend: admin form email field + `recipient_email` column, `/accept-invite` branches, `/signup` email pre-fill+lock.
- Tests: mint → preview → accept (new user, existing user, mismatch, unverified, expired).

### Batch 4: Password reset

- `password-reset/request` (anonymous, 204-always, rate-limited per IP + per email).
- `password-reset/preview` (validity check for the reset page).
- `password-reset/confirm` (`SELECT FOR UPDATE`, set password + `password_changed_at`).
- `refresh` endpoint: reject if `refresh.iat < user.password_changed_at`.
- Frontend: `/forgot-password`, `/reset-password/:token`.
- Tests: request → preview → confirm happy path; expired; consumed; rate limit; session invalidation.

### Batch 5: Existing-user migration

- `POST /auth/me/email` endpoint (set email, mint verification, dispatch — does NOT set `requires_email_verification`).
- `/auth/me` response gains `email`, `email_verified_at`, `requires_email_verification`.
- Frontend: `EmailBanner` + `AddEmailModal`, mounted in authed layout.
- Tests: banner shown/hidden/dismissed; modal success + 409; me-response shape.

### Risk callouts

- **Batch 2** is the highest-risk batch — it changes the registration and login flows. Needs thorough integration tests.
- **Batch 3 migration** revokes pending Phase 3 invitations. If there are real pending invitations in production at cutover, admins will need to re-mint. Communicate this in release notes.
- **Batch 4 rate limits** are educated guesses; watch the first week of real traffic and adjust if legitimate users are getting throttled.

### Rough scope estimate

- ~1,200 lines Rust across 5 crates.
- ~1,500 lines TypeScript across frontend.
- ~5 migration files (one per schema change, plus the data migration for pending invitations).
