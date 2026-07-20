# Platform Admin Bootstrap & Management — Design

**Date:** 2026-07-12
**Status:** Approved (pending spec review)

## Context & Motivation

The LLM Gateway uses a `platform_role` column on `users` (single variant: `platform_admin`) to gate platform-scoped endpoints (`/api/v1/admin/settings`, NATS status, etc.). Today, the **only** way to become platform_admin is to be the very first user to register against an empty database — implemented in `crates/api/src/auth.rs:register` (`user_count == 0` → auto-promote).

This has three problems:

1. **No escape hatch.** If the first admin is deleted, or if `auth.allow_registration=false` at cold start, the system is unrecoverable without manual SQL.
2. **No ongoing management.** Once the first admin is set, no one can grant or revoke platform_role through the UI. The `PATCH /admin/users/{id}` endpoint ignores `platform_role`.
3. **URL/scope mismatch.** Platform-scoped pages live under `/{slug}/admin/*` (e.g. `/test-org/admin/settings`) but `platform_role` is platform-scoped, not org-scoped. The `/{slug}/` prefix is misleading — kept for the just-shipped Platform sidebar refactor but should be fixed.

This design adds (a) a config knob + CLI subcommand for bootstrap, (b) an API + UI for ongoing grant/revoke, and (c) a URL restructure that moves platform pages to top-level `/admin/*` with a dedicated `PlatformLayout`.

## Goals

- Operators can bootstrap the first platform_admin via CLI when the auto-promotion is disabled or has been bypassed.
- Existing platform_admins can grant and revoke `platform_role` for other users through the UI.
- Platform-scoped URLs reflect their scope: `/admin/*`, no `/{slug}/` prefix.
- Backward-compat: the just-shipped `/{slug}/admin/settings` URL gets a 301 redirect to `/admin/settings`.

## Non-Goals

- Multi-role platform hierarchy (e.g., `platform_moderator`). Single role only.
- Audit logging of grant/revoke events (existing request audit captures the PATCH itself; full grant audit can follow).
- Per-org "delegate admin" role. Org admin already covers that.
- Immediate revocation effect. Revocations take effect at next access-token refresh (≤15 min staleness accepted).

## Design Decisions

| # | Decision | Rationale |
|---|---|---|
| D1 | New `auth.first_user_is_admin: bool` config field, default `true` | Preserves current behavior for self-hosted; SaaS deployments can flip it off |
| D2 | CLI subcommand `cargo run -p llm-gateway -- grant-platform-admin --username <name>` | Operator has shell access at bootstrap time; flexible enough for one-shot recovery |
| D3 | Last-admin self-revoke is **forbidden** unless CLI passes `--allow-last-admin` | Prevents accidental lockout. CLI override available for operators who know what they're doing |
| D4 | New top-level route group `/admin/*` with new `PlatformLayout` chrome | URL accurately reflects platform scope; fixes misleading `/{slug}/` prefix |
| D5 | Accept up-to-15-min staleness on revoke (next access-token expiry) | Avoids token-versioning machinery; documented in API response and UI |
| D6 | Skip `granted_by` / `granted_at` audit columns on `users.platform_role` for v1 | YAGNI. Request audit captures the PATCH event; full grant audit is a follow-up |

## Architecture

### Backend

**`crates/gateway/src/cli.rs`** (new) — `clap`-derived `Cli` struct with `Commands::GrantPlatformAdmin { username, allow_last_admin }`. Reads DB URL from config, opens pool, calls storage method, prints result. Lives alongside the existing server entry point.

**`crates/gateway/src/main.rs`** — Parses CLI before starting the server. If a subcommand was given, run it and exit. Otherwise start the gateway as today.

**`crates/storage/src/types.rs`** — `AppConfig.auth` gains `first_user_is_admin: bool` (default `true`).

**`crates/storage/src/lib.rs`** — Storage trait gains:

```rust
async fn set_user_platform_role(
    &self,
    target_user_id: Uuid,
    actor_user_id: Uuid,
    role: Option<PlatformRole>,
    allow_last_admin_override: bool,
) -> Result<(), SetPlatformRoleError>;
```

Where `SetPlatformRoleError` is an enum: `UserNotFound`, `LastPlatformAdmin`. The implementation runs `COUNT(*) WHERE platform_role = 'platform_admin'` inside a transaction; if demoting-self and count is 1, returns `LastPlatformAdmin` (unless `allow_last_admin_override` is set, in which case proceeds with a stderr-printed warning).

**`crates/api/src/auth.rs:register`** — Reads `config.auth.first_user_is_admin`. When `false`, skips the auto-promotion block even if `user_count == 0`.

**`crates/api/src/management/admin_users.rs`** (new) — Two handlers:

- `list_platform_users` — `GET /api/v1/admin/platform-users`. Returns `{ admins: Vec<UserBrief>, candidates: Vec<UserBrief> }`. Admins = users with `platform_role IS NOT NULL` (always at least 1 — the caller, since the page is gated). Candidates = search-driven; empty by default; the UI queries with `?q=<query>` for substring match on username/email. Gated by `require_platform_admin`.
- `patch_platform_role` — `PATCH /api/v1/admin/users/:id/platform-role`. Body `{ platform_role: "platform_admin" | null }`. Same gate. Calls `storage.set_user_platform_role(...)`. The handler **always passes `allow_last_admin_override = false`** to the storage layer regardless of request body — the override flag is CLI-only by design (a defense-in-depth measure against a compromised or buggy frontend).

**`crates/api/src/management/mod.rs`** — Registers the two new routes under `/admin/*`. Also adds a client-side redirect from `/{orgSlug}/admin/settings` → `/admin/settings` via React Router's `<Navigate replace>` (not an HTTP 301, since both URLs are served by the SPA) for backward compat with the just-shipped URL.

### Frontend

**`web/src/components/PlatformLayout.tsx`** (new) — Mirrors `Layout`'s aesthetic: top bar with logo + "Back to {currentOrg.name}" link + user dropdown; left sidebar with the Platform group (Settings, Platform Users); footer. No `OrgSwitcher` (platform pages don't depend on org context). Uses the same `isPlatformAdmin(user)` gate.

**`web/src/pages/PlatformUsers.tsx`** (new) — Table of current platform_admins (avatar, username, email, revoke button). Search box above the table for adding from candidates (calls `GET /admin/platform-users?q=...` with debounce). Empty state when no candidates match: "No users found." The page always has at least one admin (the caller) since it's gated, so the admin list is never empty. Revoke button is hidden when the target is the only admin (last-admin guard surfaced as the row being non-interactive).

**`web/src/pages/Settings.tsx`** — Untouched. Route just moves.

**`web/src/App.tsx`** — Restructure routes:

```
REMOVED: /{orgSlug}/admin/settings (the route, kept as 301 redirect to /admin/settings)
ADDED:   /admin (PlatformLayout wrapper, RequirePlatformAdmin guard)
           ├── /admin/settings        → Settings.tsx
           └── /admin/platform-users  → PlatformUsers.tsx (new)
```

**`web/src/components/Layout.tsx`** — The Platform sidebar group's Settings link changes from `/{slug}/admin/settings` to `/admin/settings`. Adds a Platform Users link to `/admin/platform-users`. The sidebar group still lives inside the org `Layout` (chrome stays consistent) but the URLs it points to are top-level.

## Data Flow — Granting a Role via UI

```
PlatformUsers.tsx                          admin_users.rs (PATCH)            storage
   │                                            │                              │
   │─ setPlatformRole(userId, "platform_admin")─>│                              │
   │                                            │─ require_platform_admin()    │
   │                                            │─ extract caller user_id      │
   │                                            │─ set_user_platform_role(...) ─>
   │                                            │                              │
   │                                            │              BEGIN           │
   │                                            │              COUNT(*) admins │
   │                                            │              last-admin chk  │
   │                                            │              UPDATE users    │
   │                                            │              SET platform_role
   │                                            │              COMMIT          │
   │                                            │<─ Ok ────────────────────────│
   │<─ 200 { id, username, platform_role } ────│                              │
   │                                            │                              │
   │─ refetch listPlatformUsers()              │                              │
```

The affected user's existing JWT still carries the old `platform_role` until they refresh their access token (≤15 min). UI shows a small note: "Changes take effect on the user's next login."

## Error Handling

| Failure mode | Where detected | Response | UI behavior |
|---|---|---|---|
| Target user doesn't exist | Storage query returns 0 rows | `404 Not Found` `{ error: "user_not_found" }` | Toast: "User not found" |
| Caller not platform_admin | `require_platform_admin()` | `403 Forbidden` | Redirect to dashboard |
| Self-revoke when last admin (and caller isn't CLI with override) | Storage guard | `409 Conflict` `{ error: "last_platform_admin", message: "..." }` | Toast + revoke button hidden in UI |
| Self-revoke when NOT last admin | No guard | `200 OK` | UI confirmation modal: "You will lose platform-admin access" |
| CLI: target user not found | Same storage check | Exit 2, stderr: `error: user 'foo' not found` | N/A |
| CLI: target already admin | Idempotent no-op | Exit 0, stdout: `user 'foo' is already platform_admin (no change)` | N/A |
| CLI: target is last admin, no `--allow-last-admin` | Storage guard | Exit 2, stderr: `error: cannot demote last platform admin (pass --allow-last-admin to override)` | N/A |
| CLI: DB connection failure | sqlx error | Exit 1, stderr: `error: failed to connect to database: <err>` | N/A |
| `first_user_is_admin=false` and first user registers | `register` handler skips promotion | User created with `platform_role = NULL`. Silent (no warning). | N/A |

## Testing

### Backend

| Test | Proves |
|---|---|
| `storage::set_user_platform_role_grants_role` | Promote non-admin → role set |
| `storage::set_user_platform_role_revokes_role` | Demote admin when ≥2 exist → role is NULL |
| `storage::set_user_platform_role_blocks_last_admin_self_demote` | 1 admin, demote self → `Err(LastPlatformAdmin)` |
| `storage::set_user_platform_role_allows_last_admin_with_override` | 1 admin, `--allow-last-admin=true` → succeeds |
| `storage::set_user_platform_role_idempotent_grant` | Re-grant to admin → no error |
| `storage::set_user_platform_role_404_for_missing_user` | Nonexistent UUID → `Err(UserNotFound)` |
| `api::admin_users::patch_requires_platform_admin` | Caller without claim → 403 |
| `api::admin_users::patch_returns_updated_user` | Response body shape correct |
| `api::register::first_user_is_admin_false_skips_promotion` | First user on empty DB + config false → role is NULL |
| `api::register::first_user_is_admin_true_promotes` | Default config → first user gets admin |
| `cli::grant_platform_admin_promotes_user` | Spawn CLI, assert role changed in DB |
| `cli::grant_platform_admin_blocks_self_when_last` | Exit 2 without override flag |
| `cli::grant_platform_admin_with_allow_flag_succeeds` | Exit 0 with flag |
| `cli::grant_platform_admin_idempotent` | Re-run on already-admin user → exit 0 |
| `cli::grant_platform_admin_user_not_found` | Nonexistent username → exit 2 |

### Frontend

| Test | Proves |
|---|---|
| `PlatformUsers: renders current admins` | API returns 2 admins → 2 rows |
| `PlatformUsers: revoke hidden for last admin` | Only 1 admin (self) → revoke button absent |
| `PlatformUsers: revoke shows modal when not last` | 2 admins → clicking revoke opens modal |
| `PlatformUsers: search-to-add finds candidate and grants` | Mock search, type, click Add → PATCH called, list refetches |
| `PlatformLayout: renders Platform sidebar with Settings and Platform Users links` | Visual chrome |
| `PlatformLayout: shows back-to-org link when currentOrg set` | `currentOrg.name` present → back link visible |
| `App routing: /admin/settings renders PlatformLayout, not org Layout` | Navigate → `PlatformLayout` markers present, `Layout` sidebar absent |
| `App routing: /{slug}/admin/settings redirects to /admin/settings` | Client-side redirect via React Router `<Navigate replace>` |

### Manual smoke test (in implementation plan)

1. Start gateway on empty DB → register user A → A is platform_admin (default config).
2. From a second shell: CLI demote A → A's `platform_role` becomes NULL. A's existing JWT still works until expiry.
3. Set `first_user_is_admin=false`, drop DB, register user A → A is NOT admin.
4. CLI promote A → A is admin. Log in as A → Platform sidebar visible.

## Migration / Compatibility

- **Config change is additive.** Existing deployments without the key default to `true` — no behavior change for current users.
- **URL change for `/admin/settings`:** the just-shipped `/{slug}/admin/settings` URL becomes a client-side redirect to `/admin/settings` via React Router `<Navigate replace>`. Anyone with the URL bookmarked still lands on the page.
- **No DB migration.** The `users.platform_role` column already exists. No schema changes.
- **Backward compatibility for revocations:** UI shows "Changes take effect on the user's next login."

## Files Touched

### New files
- `crates/gateway/src/cli.rs`
- `crates/api/src/management/admin_users.rs`
- `web/src/components/PlatformLayout.tsx`
- `web/src/pages/PlatformUsers.tsx`
- `web/src/pages/PlatformUsers.test.tsx`
- `web/src/components/PlatformLayout.test.tsx`

### Modified files
- `crates/gateway/Cargo.toml` (add `clap` dep)
- `crates/gateway/src/main.rs` (parse CLI, dispatch subcommand)
- `crates/storage/src/types.rs` (config field + error enum)
- `crates/storage/src/lib.rs` (trait method declaration)
- `crates/storage/src/pg.rs` (trait impl)
- `crates/api/src/auth.rs:register` (config gate on first-user promotion)
- `crates/api/src/management/mod.rs` (route registration + 301 redirect)
- `web/src/App.tsx` (route restructure)
- `web/src/components/Layout.tsx` (sidebar links point to top-level URLs)
- `web/src/api/client.ts` or new `web/src/api/admin.ts` (endpoint wrappers)
- `web/src/i18n/en.json`, `web/src/i18n/zh.json` (new UI strings: "Platform Users", "Revoke", "Back to", etc.)