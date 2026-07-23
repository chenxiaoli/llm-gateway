# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added — `model=auto` capability-aware routing

- **`model=auto` request routing**: clients can send `model=auto` on
  `/v1/chat/completions` and `/v1/messages` requests, and the gateway
  resolves a model from a per-key admin-defined pool based on capabilities
  the request actually needs. Vision is required when the body contains an
  `image_url` (OpenAI) or `image` (Anthropic) content block; tools is
  required when the body carries a non-empty `tools` array. The existing
  channel priority + weighted routing then runs over the resulting
  candidate pool, with failover across models on 5xx/429/conn-error.
- New `auto_route_configs` platform-level table (mirrors `model_fallbacks`)
  + `api_keys.auto_route_id` FK for binding a config to a key. Management
  endpoints at `/api/v1/{slug}/auto-route-configs` (CRUD).
- New `supports_vision` / `supports_tools` BOOLEAN columns on `models`,
  populated by admin manual entry on the Models page (no upstream call, no
  sync job, no auto-discovery — admin ticks the checkboxes for each model
  they want eligible for auto-routing).
- New `Auto Routes` admin page + sidebar entry under Console.
- New API errors: `auto_not_configured` (400, key has no `auto_route_id`),
  `auto_no_matching_model` (400, pool has 0 models with the required
  capabilities), `auto_all_candidates_failed` (502, every (model, channel)
  candidate failed), `model_name_reserved` (400, model creation rejects the
  name `auto`).

### Added — Platform admin bootstrap & management

- **Platform-admin management UI** (`/admin/platform-users`): list current
  platform admins, search non-admin users by username/email, grant or revoke
  `platform_role = platform_admin` from the browser. Last-admin guard refuses
  to demote the only platform admin (409 from the backend).
- **CLI subcommand** `cargo run -p llm-gateway -- grant-platform-admin
  --username <name> [--revoke] [--allow-last-admin]`: operator escape hatch
  for bootstrap when the first-user auto-promotion is disabled or when the
  last admin needs to be demoted outside the UI.
- **Config flag** `auth.first_user_is_admin` (default `true`, preserves
  existing behavior). Set to `false` to disable the silent first-user
  promotion — useful for SaaS deployments that want to bootstrap via the CLI.
- **Top-level `/admin/*` routes** with dedicated `PlatformLayout` chrome
  (sidebar with Settings + Platform Users links, header with back-to-org
  link). Backend `/api/v1/admin/settings` and `/api/v1/admin/platform-users`
  are no longer org-scoped.

### Added — User nickname field

- **User nickname field** + `POST /api/v1/auth/me/nickname` endpoint + new
  `/{slug}/profile` page. Optional display name; NULL by default for existing
  rows. Non-unique, validated to 1–32 UTF-8 chars after trim (empty string =
  clear, writes NULL). Rejects ASCII control chars, Unicode Cc/Cf category
  chars (including bidi overrides like U+202E), and zero-width chars. The
  frontend `displayName()` helper falls back `nickname → username → email`.
  The `list_members` pipeline joins `users.nickname`, so the Members table
  surfaces the nickname alongside `username`/`email`.

### Changed

- `/{slug}/admin/settings` → `/admin/settings`. A client-side `<Navigate>`
  preserves bookmarks from the just-shipped prior URL scheme.

### Changed — Users → Members refactor (v2.1.0)

- The `accounts` table is now per-membership (1:1 with `(user_id, org_id)`)
  instead of per-user. Each org membership carries its own balance and
  threshold; a user belonging to multiple orgs has one account per org.
- Account-action routes moved from `/api/v1/{slug}/admin/users/{id}/*` to
  `/api/v1/{slug}/admin/members/{user_id}/*` (`balance`, `recharge`, `adjust`,
  `threshold`). Old routes return 410 Gone.
- `PATCH /api/v1/{slug}/members/{user_id}` now accepts `{role?, enabled?,
  group_id?}` (was `change_member_role` only — role-only). Last-owner guard
  still triggers when stripping the org's last owner role.
- `list_members` response enriched: each member now includes `enabled`,
  `balance`, `threshold`, and `group_name`. Frontend `Member` type mirrors
  this; `created_at` replaces the prior `joined_at` field.
- The per-org `Members` page absorbs all capabilities of the former standalone
  `Users` page — balance drawer, recharge/adjust modals, per-member usage
  drawer, status toggle, group assignment.

### Removed — Users → Members refactor (v2.1.0)

- `GET /api/v1/{slug}/admin/users`, `POST …/admin/users`,
  `PATCH/DELETE …/admin/users/{id}` — replaced by the `/members/*` family.
- Storage methods `list_users_paginated` and `delete_user` (and the
  `PgUserWithBalanceRow` row struct).
- Frontend `pages/Users.tsx`, `api/users.ts`, `hooks/useUsers.ts`, and the
  `UserResponse` / `UpdateUserRequest` types.
- Sidebar entry `/{slug}/admin/users`; i18n keys `sidebar.users` and
  `toasts.user{Updated,Deleted,UpdateFailed,DeleteFailed}`. The
  `users.{drawer,rechargeModal,adjustModal,usageDrawer}.*` keys are kept —
  Members.tsx still consumes them.

### Fixed

- `/api/v1/{slug}/admin/logs` time filter (`since`/`until`) no longer returns 400 with `operator does not exist: timestamp with time zone >= text`. `query_logs_paginated` was binding `since`/`until` as RFC3339 strings via a homogeneous `Vec<String>`, so sqlx sent them as postgres `text` and the `audit_logs.created_at >= $N` comparison failed. Filter values are now bound as typed `DateTime<Utc>` (matching the `query_usage_paginated` pattern), so postgres receives `timestamptz`. Same fix applied to the v1.x maintenance line as v1.8.5.
- `/api/v1/{slug}/admin/logs` time filter now accepts date-only values (`YYYY-MM-DD`, as produced by the frontend's `<input type="date">` picker). Previously the filter deserialized `since`/`until` with `DateTime::parse_from_rfc3339`, which rejects date-only strings with `Failed to deserialize query string: premature end of input`. `since` is parsed as that day's UTC midnight, `until` as that day's UTC end-of-day (23:59:59.999999) so logs from the entire day are included. RFC3339 input continues to work as before. Same fix applied to the v1.x maintenance line as v1.8.6.
- Frontend `orgPrefix()` helper returned `/api/v1/${slug}` while `apiClient` already had `baseURL: '/api/v1'`, so axios combined them into `/api/v1/api/v1/${slug}/...` for every org-scoped endpoint. The backend's v2.1.0 `legacy_gone` 410 fallback then surfaced this as "endpoint moved in v2.1.0" on every channel/member/key/usage/etc. call. `orgPrefix()` now returns `/${slug}` and lets `baseURL` carry the `/api/v1` prefix.

### Fixed — Users → Members refactor (v2.1.0)

- `storage::add_balance` INSERT arity bug: 9 columns but only 8 `$N`
  placeholders. Broke every recharge / adjust call since the SaaS
  multi-tenant refactor; regression test added.

### Added — Phase 4: Email + Email-Bound Invitations (v2.1.0)

- **Email subsystem** (`crates/email`, crate name `llm-gateway-email`): a
  standalone mailer crate with three transports — `NoopMailer` (default, drops
  mail silently), `FileMailer` (writes RFC-822 `.eml` files to a directory
  for local dev / test token extraction), and `SmtpMailer` (real delivery via
  `lettre`). Transport is selected by `[email] transport` in `config.toml`;
  `file_output_dir` and `[email] from` configure the file/sender address.
  Handlebars templates (`verification`, `password_reset`, `invitation`) render
  both plain-text and HTML bodies.
- **Schema additions** (migrations `20260711000001`–`20260711000004`):
  `users.email` / `users.email_verified_at` / `users.password_changed_at`;
  `invitations.recipient_email` (required at mint time); new
  `email_verifications` and `password_resets` tables, both token-indexed with
  expiry + consumed-at columns. Each migration ships a `.down.sql` companion.
- **Email verification on signup.** `POST /api/v1/auth/register` now requires
  an `email` field and dispatches a verification email (best-effort; delivery
  failure never blocks registration). Brand-new users have
  `email_verified_at = NULL`; `POST /api/v1/auth/login` rejects them with
  `403 email_not_verified` until they click through. `POST /auth/verify-email`
  and `POST /auth/verify-email/resend` complete / re-trigger the flow.
- **Password reset.** `POST /auth/password-reset/request` is always-204
  (doesn't leak whether the email is registered). `GET /auth/password-reset/
  preview` validates a token without consuming it; `POST /auth/password-reset/
  confirm` sets the new password and single-use consumes the token. The
  confirm handler stores the new password and marks `password_changed_at` in
  one atomic transaction (SELECT FOR UPDATE on the reset row), so a partial
  failure can never leave a token re-usable. Refresh tokens issued before
  the reset are rejected on the next `/auth/refresh` (epoch check on the
  refresh JWT's `iat` vs `users.password_changed_at` → `401 unauthorized`),
  forcing re-login on every active session.
- **Email-bound invitations.** `POST /orgs/{slug}/invitations` now requires
  `recipient_email`; the invitation is bound to that address. `POST
  /invitations/accept` (logged-in accept) enforces `email_mismatch` /
  `email_verification_required` (403). `POST /auth/register` with an invite
  token runs the accept server-side and rejects on `email_mismatch` /
  `email_required`. The Invitations admin page adds a recipient-email input
  (Generate disabled until valid); the table shows the recipient column.
- **`POST /api/v1/auth/me/email`** — lets an existing user (legacy account
  with no email) set and verify an address without blocking login. Dispatch
  is fully best-effort; a 204 is returned regardless of mailer outcome.
- **New `ApiError` codes** (cross-ref `crates/api/src/error.rs`):
  `email_required` (400), `email_in_use` (409), `email_mismatch` (400 on
  register / 403 on accept), `email_not_verified` (403, login gate),
  `email_verification_required` (403, accept gate), `verification_expired`
  (410), `verification_not_found` (404), `reset_expired` (410),
  `reset_consumed` (410), `reset_not_found` (404).
- **Frontend**: `/check-email`, `/verify-email/:token`, `/forgot-password`,
  `/reset-password/:token` routes and pages; Login page surfaces an inline
  resend panel on `email_not_verified`; Register redirects to `/check-email`
  post-signup; AcceptInvite branches on the signed-in user's email state
  (missing / mismatch / unverified / verified-match); EmailBanner +
  AddEmailModal prompt legacy users to add an email; new i18n keys under
  `verify_email`, `check_email`, `forgot_password`, `reset_password`,
  `emailBanner`, `addEmailModal`, and `acceptInvite`.

- **Phase 5 (per-org defaults + rate-limit enforcement):**
  - Added: `GET`/`PUT /api/v1/orgs/{slug}/defaults` for org-wide rate-limit RPM and monthly budget defaults. UI lives in Org Settings → Defaults.
  - **Behavior change:** per-key rate limits (`api_keys.rate_limit`) are now **enforced** at request time via the existing in-memory rate limiter — previously stored but never checked. Resolution order: `key.rate_limit ?? org.default_rate_limit_rpm ?? unlimited`. Exceeding returns `429` with `Retry-After` set to the configured rate-limit window size.
  - Org-level `default_budget_monthly_usd` is stored but **not enforced** in this phase (parity with existing per-key budget — both will be enforced in a future phase).
  - **Upgrade note:** any existing `api_keys` rows with non-null `rate_limit` will start receiving 429s on requests beyond their limit. Audit existing keys before deploying if any have low values set.
  - Implicit fix: `api_keys.rate_limit` Postgres decode (INT4 column was decoded as `Option<i64>`, now correctly `Option<i32>`); invisible until enforcement made the column live.
- **Phase 6 (budget enforcement):**
  - **Behavior change:** per-key monthly budgets (`api_keys.budget_monthly`) and org-default budgets (`default_budget_monthly_usd` from Phase 5) are now **enforced**. Resolution order: `key.budget_monthly ?? org.default_budget_monthly_usd ?? unlimited`. Exceeding returns `429` with `error.type = "budget_exceeded"` and body `{ key_id, month_bucket, limit, accrued }` (USD floats). No `Retry-After` — caller must wait until next month or have budget raised.
  - New `budget_counters` table materializes month-to-date spend per key (UTC calendar month), updated atomically with each `usage_records` insert via app-level transaction in `record_usage`.
  - Counting semantic is **post-completion**: the check uses MTD that excludes the current request's cost. The request that pushes MTD over budget is allowed; the next request is rejected. Industry-standard leak (matches Stripe, OpenAI).
  - OrgSettings `budgetHelp` text updated — the previous "Not currently enforced" disclaimer is removed.
  - **Upgrade note:** any existing `api_keys` rows with non-null `budget_monthly`, or orgs with `default_budget_monthly_usd` set, will start receiving 429s on requests once their month-to-date spend exceeds the budget. Audit existing values before deploying.
- **Phase 7 (budget observability):**
  - **New UI:** OrgSettings gets a "Budget status" subsection showing org MTD total (sum across all keys) against the org-default budget, with a color-coded progress bar (green <60%, yellow 60-80%, orange 80-100%, red >100%). The Keys table gets an "MTD this month" column showing per-key spend with the same color coding.
  - **New endpoint:** `GET /api/v1/{slug}/budget-status` returns `{ accrued_units, month_bucket }` (i64 subunits, UTC calendar month). Member-gated (parity with `GET /{slug}/defaults`).
  - **Extended endpoint:** `GET /api/v1/{slug}/keys` now includes `mtd_units: i64` per key. Additive, non-breaking — existing API consumers ignore the new field.
  - **New storage methods:** `Storage::get_org_month_to_date_spend(org_id)` and `Storage::list_keys_paginated_with_mtd(org_id, page, page_size)`. Both read the existing `budget_counters` table (from Phase 6) — no schema changes.
  - **No behavior change:** enforcement remains as shipped in Phase 6 (post-completion, fail-open on storage errors). This phase is purely read-side observability.

### Changed
- `POST /api/v1/auth/register` now requires `email`; without an invitation
  token the new user starts in the unverified limbo state (cannot log in
  until `/verify-email` completes).
- Default `config.toml` ships with `[email] transport = "noop"` — production
  deployments must switch to `"smtp"` (or `"file"`) to actually deliver mail.

### Removed
- The ability to mint an invitation without a `recipient_email`. **Pending
  pre-Phase-4 invitations (NULL recipient) are revoked by the
  `20260711000002_invitations_recipient_email.sql` migration** — admins must
  re-mint them with a recipient address. Accepted and already-revoked
  historical rows are retained unchanged. The old generic-token flow was
  effectively unauthenticated, so this is a deliberate security hardening.

### Added — Phase 3: Wizard-gated signup + invitations

- Wizard-first signup: brand-new users land at `/onboarding` and create or join
  an org before reaching any org-scoped UI. Pre-existing users are unaffected.
- Generic single-use magic-link invitations. Org admins can mint a 7-day
  invitation URL at `/{org_slug}/admin/invitations` and share it out-of-band
  (Slack, etc.); the first user to present the token joins the org.
- `/accept-invite?token=...` landing page renders invite metadata for logged-out
  visitors (Sign up / Log in) and logged-in users (Accept / Decline).
- `GET /api/v1/auth/me/onboarding` returns `{ needs_onboarding: bool }` so the
  SPA can detect limbo users (signed in, zero org memberships) without
  round-tripping the full `me` payload.
- `POST /api/v1/orgs` reissues the access token with the caller's effective
  current org. Auto-switches `current_org` only when the caller was in the
  limbo state (preserves the working context of users adding a second org).

### Changed
- `POST /api/v1/auth/register` no longer auto-assigns a default-org membership.
  Brand-new users have `current_org_id = NULL` and `orgs = []` until they
  complete the onboarding wizard.

### Removed
- The "default org" bootstrap on first-user signup (was a Phase 1 holdover).
  The migration-time default org still exists for pre-Phase-3 data.

## [2.0.0] - 2026-07-07

Phase 1 of SaaS multi-tenant support. The schema now models organizations (tenants) as first-class entities; existing single-tenant deployments continue to work — every existing row is moved into a default `org_default` tenant, and the API surface for the default org is unchanged. Future phases will expose org switching and per-org admin surfaces in the UI.

### Changed (BREAKING — schema migration required)
- **Database schema is now multi-tenant.** New `orgs`, `members`, and `org_settings` tables. Every tenant-scoped table (`api_keys`, `channels`, `channel_models`, `model_fallbacks`, `audit_logs`, `transactions`, `rate_limit_buckets`, `user_groups`, etc.) gains a non-null `org_id` column. Catalog tables (`providers`, `models`, `pricing_policies`, `groups`) gain a nullable `owner_org_id` (NULL = platform-wide, visible to all orgs; non-NULL = org-private). The `20260708000000_saas_orgs.sql` migration creates `org_default` and moves every existing row into it. Back-roll available via `20260708000000_saas_orgs.down.sql` (documented semantic gaps — old `users.role` cannot be perfectly reconstructed).
- **JWT claims shape changed.** Tokens now carry `current_org_id` (active tenant for the session) and `platform_role` (`null` or `"platform_admin"`). Existing JWTs are rejected — every web user must re-authenticate once after upgrade. API keys (separate from JWTs) continue to work and resolve to `org_default`.
- **Two-layer role model.** `users.role` is removed. Tenant role lives in `members.role` (`owner` / `admin` / `member`); platform-wide admin lives in `users.platform_role`. First user in the DB becomes `owner` of `org_default`; subsequent users become `member` (preserves the prior first-user-is-admin behavior).
- **Web frontend version** jumps from `0.16.7` to `2.0.0` to align with backend versioning.

### Added
- `POST /api/v1/auth/orgs` — create a new org. Caller becomes its `owner`. Slug must match `^[a-z0-9-]{3,64}$`.
- `GET /api/v1/auth/orgs` — list orgs the current user is a member of.
- `POST /api/v1/auth/orgs/{id}/switch` — switch the current session's active org. Returns a fresh JWT scoped to the new org.
- `current_org` and `orgs` fields on `AuthResponse` and `MeResponse`.
- `org` crate: `resolve_org_context` extractor and permission helpers (`can_create_org_catalog`, `can_create_platform_catalog`, etc.) used by catalog handlers to enforce the `owner_org_id IS NULL OR owner_org_id = $1` visibility filter.

### Fixed
- `query_logs` had a parameter-binding bug when partial filters were applied — hard-coded `$2`–`$5` didn't match the dynamic bind order, producing wrong results or PG errors. Now uses the same dynamic-index pattern as the paginated sibling query.
- `set_provider_models` now rejects caller attempts to reassign catalog rows to a different org (defense-in-depth against ownership drift).
- `nats-publisher` events now default `org_id` to `"org_default"` via `#[serde(default = "default_org_id")]`. The previous `#[serde(default)]` on a `String` field defaulted to `""`, breaking the FK on replay of pre-migration events.
- Cross-org authorization gaps in `/admin/users/{id}`, `/admin/logs/{id}`, and `/admin/requests/{id}` — members can no longer read or mutate other orgs' resources (returns 404 to avoid existence leak).

## [1.8.4] - 2026-07-04

### Changed
- `proxy_inner` split: once-only work (auth via `get_key_by_hash`, balance check via `get_account_by_user_id`, user role via `get_user`, body parse, `request_id` generation) now runs in `proxy_inner` proper, which is not recursive. Routing, failover, fallback, and audit dispatch live in a new `proxy_route_and_forward` which is safe to recurse into. `try_model_fallback` now recurses into `proxy_route_and_forward` instead of `proxy_inner`.

### Fixed
- Each fallback attempt no longer re-runs `get_key_by_hash`, `get_account_by_user_id`, and `get_user`. A request that fans out across N fallback models used to make (N+1)× auth/balance/role DB calls; now it is 1× each per HTTP request.
- `audit_log.routes` now records the full fallback chain in a single row. Previously each `proxy_route_and_forward` call had its own local `routes` Vec, and silent early-returns (model not in registry, no enabled channels) recorded nothing — a request that fell back across N models produced either one row whose `routes` contained only the last fallback's channel attempts, or N partial rows. The `routes` Vec is now threaded through the recursion via `&mut`, silent exits push pseudo `RouteAttempt` entries (so e.g. "no enabled channels" is visible), and the all-channels-failed dispatch is gated by `fallback_depth == 0` so deeper fallback attempts don't produce duplicate rows.
- `RouteAttempt` now carries a `provider_id` field (populated for every real upstream attempt, empty string for pseudo entries that mark silent routing exits).

### Removed
- The `client_model: Option<String>` parameter on `proxy_inner` (added in 1.8.3 as a band-aid) is gone. `client_requested_model` is now a `String` threaded from `proxy_inner`.

## [1.8.3] - 2026-07-04

### Fixed
- `audit_log.original_model` was still wrong when a request hit the model-fallback path. The 1.8.2 fix captured `model_name` after `try_model_fallback` had rewritten the body's `model` field, so fallback-resolved requests recorded the substituted model under `original_model`. The proxy now threads an explicit `client_model: Option<String>` parameter through `proxy_inner`: HTTP handlers pass `None` (the body is unmodified and `model_name` is the client's verbatim request), and `try_model_fallback` passes `Some(original_model)` into the recursive call so fallback substitution doesn't poison the audit record. Verified live: a client request for an unmapped model that fell back to `glm-5.1` now correctly records `original_model=<client's model>`, `model_name=glm-5.1`.

## [1.8.2] - 2026-07-03

### Fixed
- `audit_log.original_model` was effectively `NULL` for every real request despite being declared in 1.8.0. The proxy captured the client's request model in an unused underscore-prefixed variable and then reassigned `model_name` to the channel's canonical name on the next line; the audit task picked up `model_name` (now the channel's name) under the label `original_model` only when an upstream mapping existed. The field is now always populated with the client's original request.

### Changed
- `/admin/logs` table replaces the `Input` and `Output` token columns with an `Original Model` column showing the client-requested model directly. When the client-requested model differs from the routed channel model, the cell renders `original → channel` (with the channel model in primary color) so the override is visible at a glance. Token counts are still available in the log-detail drawer.

## [1.8.1] - 2026-07-03

### Fixed
- `GET /api/v1/admin/logs/{id}` returned 500 with `"no column found for name: routes"` because the `get_log` SELECT in `crates/storage/src/postgres.rs` was not updated alongside `get_audit_by_request_id` in v1.8.0. The `routes` column is now read on both code paths.

## [1.8.0] - 2026-07-01

### Changed (BREAKING for custom SQL on audit_logs)
- One audit row per **client request** instead of per upstream attempt. A request that fans out across N channels via failover produces one row whose new `routes` JSONB array contains all N attempts. The previous per-attempt row design (added in 1.7.2) is gone.
- Top-level `audit_logs.model_name`, `channel_id`, `channel_name`, `status_code`, `request_body`, `response_body`, `input_tokens`, `output_tokens`, `latency_ms` now reflect the **final** attempt (success, or last failure if all routes failed). Use the `routes` array to inspect the full attempt history.
- Previously, the all-failed case (every channel returned an error) produced no audit row at all. It now produces exactly one row with `routes` containing every attempt.

### Added
- `audit_logs.routes` JSONB column. Each entry has: `model`, `channel_id`, `channel_name`, `status_code` (0 = connection error), `error_message`, `latency_ms`, `started_at`.
- New `/admin/logs` "Routes" column with click-to-expand modal showing each attempt's model, channel, status, latency, started-at, and error message. Status is color-coded (green/amber/red) and `CONN` indicates a connection error.
- `AuditEvent.routes` field on the NATS audit event (with `#[serde(default)]` for forward/backward compatibility with the 1.7.x worker during rolling deploys).
- Null-byte (U+0000 / U+FFFD) sanitization in `routes[*].error_message` (parity with the 1.7.2 `response_body` fix).

## [1.7.2] - 2026-06-24

### Fixed
- Audit log now records every upstream attempt, including 5xx server errors and connection failures. Previously only successful and 4xx responses were audited, so a single client request that fanned out across multiple upstream channels via failover would leave failures invisible. One client request may now produce multiple audit rows sharing the same `request_id`, one per upstream attempt.
- `AuditLogger::log_request` now sanitizes U+0000 and U+FFFD characters from the response body before writing to Postgres. The audit-worker was Nak-retrying legacy events containing null bytes indefinitely because Postgres TEXT columns reject U+0000.

### Added
- Users admin page now shows a `Group` column. Backend already returned `user.group_name` since 1.7.0; this exposes it in the table (was previously drawer-only).

## [1.7.1] - 2026-06-24

### Fixed
- `GET /api/v1/admin/groups` now returns a paginated response (`{ items, total, page, page_size }`) matching the spec and the `list_users` pattern. The previous raw-array response caused the Groups admin page to crash with `groups.map is not a function` when the response object was treated as an array.

## [1.7.0] - 2026-06-24

### Added
- User and channel groups for access control. Admins can create groups and assign users and channels. A user in group X can only access channels in group X (or channels with no group). Users with no group remain unrestricted.
- New `Groups` admin page and `/api/v1/admin/groups` CRUD endpoints.

### Changed
- `channels.group` column refactored from free-form TEXT to `channels.group_id` foreign key to a new `groups` table. Existing channel-group values are migrated automatically.
- `Channel` API responses use `group_id` and `group_name` (replaces `group`).
- `User` API responses include `group_id` and `group_name`.
- Routing now filters candidate channels by the requesting user's group (admin role bypasses).

## [1.6.2] - 2026-06-24

### Fixed
- Registration and login failures now surface the actual backend error message (e.g. "Username already exists", "Account locked") instead of always showing the generic i18n fallback. Two-layer fix: `authStore` re-throws the original Axios error instead of wrapping it in a generic `Error`, and `getErrorMessage` now reads the correct path (`data.error.message`) matching the backend's `{ error: { message, type } }` shape. As a side effect, all other error toasts that use the shared helper (model fallbacks, accounts, pricing, models, channel models, change password) also show real reasons instead of generic fallbacks.

## [1.6.1] - 2026-06-24

### Fixed
- Settings page About tab test: replace stale `/GitHub/i` assertion (GitHub link card was removed in v1.6.0) with section-title check on "GATEWAY INFORMATION"

## [1.6.0] - 2026-06-24

### Changed
- Rebrand frontend UI from "LLM Gateway" to "TokenVis" across home/docs/login/register headers, console sidebar and footer, browser tab title, and the brand logo mark
- Remove open-source references from UI: GitHub links in home, settings, and sidebar; "Open Source" footer text; "Star on GitHub" CTA

## [1.5.2] - 2026-06-04

### Changed
- Docker Compose: use `:${IMAGE_TAG:-latest}` image tag (was hardcoded `:develop`) and relative `./config.toml` volume path (was absolute `/opt/dev/...`)

## [1.5.1] - 2026-05-29

### Fixed
- Remove duplicate language toggle button from landing page header

## [1.5.0] - 2026-05-28

### Added
- Documentation site (`/docs`) with bilingual (Chinese/English) support, route-based language switching, and MDX content rendering
- Language toggle on landing page header
- Midnight-crossing time range support for channel available hours (e.g. 00:00–09:00 UTC+8)

### Fixed
- Handle midnight-crossing available hours in both backend routing and frontend status indicator — ranges where start > end now use OR logic instead of AND
- Add missing circuit breaker trait methods to MockChannelRegistry for test compilation

## [1.4.7] - 2026-05-27

### Added
- Channel-model level circuit breaker: 429/SSE error disables specific (channel, model) combination in memory, not entire channel. Immediate effect, auto-recovery via `Instant` expiry
- Optional token authentication for NATS connections (`[nats] token = "..."` in config.toml)
- Audit logging for upstream 429 rate limit responses
- Timezone-aware available hours: display and edit in browser timezone with utility functions and tests

### Changed
- Usage worker skips recording when cost is 0 (reduces noise for free requests)

### Fixed
- Filter forwarded headers (`x-forwarded-*`, `content-type`, `sec-*`, etc.) to prevent duplicate headers causing upstream 400 errors
- Handle `hour12: false` returning 24 at midnight in available hours check
- Use plain local time for new available hours slot defaults, convert in useEffect

## [1.4.6] - 2026-05-23

### Fixed
- Remove broken chart-render unit test (SVG rendering not supported in jsdom)

## [1.4.5] - 2026-05-23

### Added
- Request ID column on /console/usage table (first 8 chars) with copy-to-clipboard button
- Request ID column on /admin/logs table (first 8 chars) with copy-to-clipboard button
- Request ID exact-match filter on /admin/logs page
- Shared `CopyButton` UI component with visual feedback (copy → check icon toggle)
- i18n keys for request ID labels and copy toast (English + Chinese)

### Changed
- `request_id` included in usage API response (`UsageRecordResponse`)
- `request_id` fields in `UsageRecord`, `PgUsageRow`, `UsageRecordResponse` changed to `Option<String>` for legacy data compatibility

### Fixed
- Backend logs query supports filtering by `request_id` (exact match via `WHERE request_id = $1`)
- `CopyButton` uses project `cn()` utility for className composition

## [1.4.4] - 2026-05-21

### Added
- SSE error auto-recovery: when upstream returns `event: error` in SSE stream, channel is automatically disabled until recovery time
- `disabled_until` column on channels table with `disable_channel_until()` storage method
- `parse_recovery_timestamp()` extracts reset time from error messages (supports Chinese pattern `将在 YYYY-MM-DD HH:MM:SS 重置` and ISO 8601)
- ChannelRegistry skips channels where `disabled_until > now()` during reload

### Changed
- `SseAuditParams` now includes `disable_duration_secs` (default 5 minutes)
- Added `regex-lite` dependency for timestamp parsing

### Fixed
- All channel SELECT queries now include `disabled_until` column
- `Channel` struct initialization in management API includes new `disabled_until` field

## [1.4.3] - 2026-05-17

### Added
- Daily usage chart respects browser timezone — frontend sends IANA timezone (e.g. `Asia/Shanghai`), SQL groups by local date via `AT TIME ZONE`

### Fixed
- Daily usage chart date grouping uses explicit `AT TIME ZONE` instead of relying on PostgreSQL session timezone
- Backfill script parses JSONB config field from string to dict (psycopg2 returns JSONB as str)

## [1.4.2] - 2026-05-17

### Changed
- `usage_records.pricing_policy` now stores the complete PricingPolicy object (id, name, billing_type, config) instead of only the config JSON — historical usage data is self-contained for cost analysis

### Fixed
- Channel test handlers updated for `ChannelTestResult[]` API return type (was accessing `.success` on array)

### Added
- Python backfill script (`scripts/backfill_pricing_policy.py`) to populate pricing_policy and weighted_tokens for existing rows

## [1.4.0] - 2026-05-17

### Added
- OpenAI `/v1/responses` transparent proxy endpoint (passes requests through to upstream provider)
- Daily token usage line chart on dashboard with 7/30 day toggle (weighted tokens, theme-aware colors)
- `pricing_policy` and `weighted_tokens` columns on usage records (migration, types, API, storage)
- `calculate_weighted_tokens` function in billing crate for normalized token cost calculation
- `GET /usage/daily` endpoint aggregating daily token counts with all token type breakdowns

### Fixed
- Channel test SSE mode detects `error` in SSE data lines (not just event type)
- Channel test endpoint iteration compile error
- Strip `/v1` prefix for all OpenAI protocol endpoints (prevents doubled paths)
- Preserve `/v1` prefix for non-chat-completions OpenAI endpoints
- Daily usage chart rendering — replaced ResponsiveContainer with explicit dimensions for reliable display

## [1.3.20] - 2026-05-15

### Added
- Channel test supports SSE mode (`stream` query parameter) with streaming request and SSE preview

## [1.3.19] - 2026-05-15

### Fixed
- Channel test detects top-level `error` field in HTTP 200 JSON responses and marks as failed
- Channel test detail shows in Modal instead of inline

### Added
- Channel filter on `/admin/logs` audit log page
- `response_data` field in `ChannelTestResult` returned from backend
- Test result detail in Modal with latency, model, error, and formatted JSON response body

## [1.3.18] - 2026-05-14

### Fixed
- Cache-miss routing path now checks channel available_hours — channels outside scheduled hours trigger model fallback instead of being selected as candidates
- Model fallback triggers correctly when all channels for the primary model are outside their available hours

### Added
- Enable/disable toggle button on channel rows in admin Channels page (replaces static ACTIVE/OFF badge)

## [1.3.17] - 2026-05-13

### Fixed
- Model fallback now logs the specific reason when skipped (missing config on key, config not found in DB, no matching group, body parse failure) instead of silently returning None

## [1.3.16] - 2026-05-13

### Fixed
- Resolve duplicate migration version `20260508000000` causing CI test failures (renamed `drop_account_currency` to `20260509000000`)

## [1.3.15] - 2026-05-13

### Fixed
- Model fallback no longer causes infinite recursion when both models in a fallback group fail (was stack overflowing via A→B→A→B→… loop)

## [1.3.14] - 2026-05-10

### Fixed
- Usage page (`/console/usage`) now always filters by current logged-in user (admins previously saw all users' data)
- Key filter on usage page now works correctly alongside user filter (was ignored due to `else if` logic)
- Added index on `usage_records(user_id, created_at)` for query performance

## [1.3.13] - 2026-05-08

### Added
- Multi-currency display support (USD/CNY) as a system-level setting
- Currency selector in Settings > General for admins
- `currency` field in `/auth/config` and settings API responses
- Frontend currency store (Zustand) with symbol-aware formatting across all pages

### Changed
- Removed per-account `currency` field in favor of global system currency
- All monetary amounts now display using the configured currency symbol

## [1.3.12] - 2026-05-07

### Added
- Retry next available channel when upstream returns 429 rate limit

## [1.3.11] - 2026-05-05

### Fixed
- NATS stream pending messages now shows stream message count when no consumers exist (was incorrectly 0)

## [1.3.10] - 2026-05-05

### Fixed
- Channel test now uses the same URL construction logic as proxy, fixing 404 on providers with non-standard version segments (e.g. /v4)

## [1.3.9] - 2026-05-05

### Changed
- Refactor integration tests to use `sqlx::test` pattern with per-test isolated databases

## [1.3.4] - 2026-05-05

### Added
- NATS stream status pills on admin dashboard showing USAGE and AUDIT pending message counts

## [1.3.3] - 2026-05-05

### Changed
- Remove estimated pending bytes from NATS stream status, keep only exact pending message count

## [1.3.2] - 2026-05-05

### Added
- Show pending message size (estimated bytes) alongside pending count in NATS stream status

## [1.3.1] - 2026-05-05

### Added
- Show unconsumed (pending) message count in NATS stream status cards

## [1.3.0] - 2026-05-04

### Added
- Channel group field for logical grouping of channels (backend migration, API, frontend forms/display)
- Per-endpoint test buttons on channel detail page with Anthropic protocol support
- NATS JetStream status endpoint (`GET /api/v1/admin/nats/status`) showing real-time stream stats
- NATS stream status cards in Settings System tab (messages, size, consumers, retention)

## [1.2.2] - 2026-05-05

### Fixed
- OpenAI-compatible providers with versioned base URLs (e.g. `/v4`, `/v1`) no longer produce doubled paths
- Anthropic-compatible providers on non-standard hosts correctly append `/v1/messages`

## [1.2.1] - 2026-05-05

### Fixed
- Show API key ID on audit log table rows and detail drawer
- Fix Register test placeholder capitalization after i18n migration

## [1.2.0] - 2026-05-04

### Added
- Per-request balance deduction replaces batch settlement — usage worker deducts immediately after recording each request
- Shared `request_id` across `usage_records`, `audit_logs`, and `transactions` for 1:1:1 traceability
- `GET /api/v1/admin/requests/:request_id` endpoint to look up usage record, audit log, and transaction by request_id
- Frontend transaction drill-down — click a debit transaction to see usage details (model, tokens, cost, latency)
- Gateway auto-injects `stream_options: { include_usage: true }` for OpenAI streaming requests missing the field

### Changed
- Batch settlement worker (`crates/api/src/settlement.rs`) removed — no more 60s interval aggregation

### Fixed
- OpenAI-compatible streaming requests without `stream_options` in the body no longer silently skip billing

## [1.1.0] - 2026-05-03

### Added
- Frontend internationalization (i18n) with English and Simplified Chinese support
- Language toggle (Globe icon) in sidebar header — instant switch, persists to localStorage
- Browser language auto-detection (falls back to English)
- 850 translation keys across 25 sections covering all pages, components, hooks, and toast messages
- `react-i18next` + `i18next` with bundled JSON translation files

### Changed
- **SQLite removed** — PostgreSQL is now the only database driver
- NATS JetStream is required (no mpsc fallback) — gateway fails to start without `[nats]` config
- NATS streams renamed from `GATEWAY_*` to `LLM_GATEWAY_*` (`LLM_GATEWAY_USAGE`, `LLM_GATEWAY_AUDIT`)
- Audit and usage workers extracted into independent binaries (`llm-gateway-usage-worker`, `llm-gateway-audit-worker`)
- Docker builds now produce 3 binaries with `entrypoint` override for worker services
- Production docker-compose includes NATS service with JetStream
- Integration tests use PostgreSQL service container instead of SQLite

### Fixed
- ResolveJsonModule added to tsconfig for JSON imports
- ConfirmDialog i18n defaults resolve at render time (not module load)
- Test render helper imports i18n for component test compatibility

## [1.0.0] - 2026-05-03

### Added
- NATS JetStream integration for decoupled audit and usage event processing
- `nats-publisher` crate with `UsageEvent` and `AuditEvent` types, stream management, and push consumers
- Two separate JetStream streams: `GATEWAY_USAGE` (7d retention) and `GATEWAY_AUDIT` (30d retention)
- In-process NATS consumers write to DB; external consumers can attach independently
- Backward compatible — when `[nats]` config is absent, falls back to in-process mpsc channel
- Console Models page — read-only model listing for all authenticated users with search, pricing display (per_token and context_tiered)
- `GET /api/v1/user/models` endpoint for console model data
- Channel Test button on admin Channels page — tests upstream connectivity with inline status feedback
- `POST /api/v1/admin/channels/{id}/test` endpoint for channel testing

### Fixed
- Normalize request model name to database canonical form for consistent usage/audit records
- Console Models page only shows live (available) models
- Price conversion (subunits → USD) for all billing types on console model cards
- Channel test endpoint upstream URL missing /v1 prefix
- Removed /v1 from seed provider endpoints to prevent URL path doubling

## [0.14.1] - 2026-05-03

### Fixed
- Normalize request model name to database canonical form for consistent usage/audit records regardless of request casing
- Console Models page now only shows live (available) models
- Add context_tiered pricing display with tier-by-tier breakdown on model cards
- Fix price conversion (subunits → USD) for all billing types on console model cards
- Channel test endpoint upstream URL was missing /v1 prefix
- Removed /v1 from seed provider endpoints to prevent URL path doubling

## [0.14.0] - 2026-05-03

### Added
- Console Models page — read-only model listing visible to all authenticated users, showing name, type, pricing, and availability status
- `GET /api/v1/user/models` endpoint for console model data (admin-only details excluded)
- Channel Test button on admin Channels page — sends a minimal chat completion request through the channel's upstream provider and reports success/failure with latency
- `POST /api/v1/admin/channels/{id}/test` endpoint for channel connectivity testing
- `ChannelTestResult` type (backend + frontend)

### Fixed
- Channel test endpoint upstream URL was missing `/v1` prefix, causing 404 errors on OpenAI-compatible providers
- Removed `/v1` from seed provider endpoints (OpenAI, MiniMax, Alibaba) to prevent URL path doubling
- Console Models page now handles non-array API responses gracefully

## [0.13.5] - 2026-05-02

### Fixed
- `apiClient` (used by keys, model-fallbacks, usage, accounts) was not attaching Bearer token to requests — all non-`/admin/*` authenticated endpoints returned 401

## [0.13.4] - 2026-05-02

### Fixed
- Channel usage summary query now groups by both `channel_id` and channel name (PostgreSQL GROUP BY requirement)

## [0.13.3] - 2026-05-02

### Fixed
- Add placeholder migration for `20260424000000` — fixes `VersionMissing` crash on startup for databases that already had this migration applied

## [0.13.2] - 2026-05-02

### Added
- `created_by` column on channels table — tracks which admin user created each channel
- `created_by` field in channel API responses

## [0.13.1] - 2026-05-02

### Added
- Channel usage summary API endpoint (`GET /api/v1/usage/channel-summary`) — server-side aggregation of usage_records by channel_id with channel names
- `ChannelUsageSummaryRecord` storage type and `query_channel_usage_summary` method for SQLite and PostgreSQL
- Frontend `useChannelUsageSummary` hook and API client

### Changed
- Admin dashboard channel usage section now uses server-side aggregation instead of client-side aggregation from 200 audit log entries

## [0.13.0] - 2026-05-02

### Added
- Admin dashboard — system status, metrics, top models, channel usage breakdown, recent requests
- Provider models management — add/edit/remove models per provider via modals (pricing policy, upstream name)
- `PUT /api/v1/admin/providers/{id}/models` endpoint for updating provider model assignments
- Pricing policy column in provider_models table (migration: `20260505000000_provider_models_pricing`)
- Channel usage section on admin dashboard showing per-channel request distribution, latency, and error rate
- Dashboard nav item in admin sidebar

### Changed
- Provider cards now show models as clickable badges (click to edit) with pricing indicator dots
- ChannelDetail crash on non-array API responses fixed with `Array.isArray()` guard

## [0.12.0] - 2026-05-02

### Added
- Provider models catalog — new `provider_models` table records which models each provider supports
- Model dropdown in "Add Channel Model" modal now filters by channel's provider
- Upstream model name auto-filled from provider catalog when selecting a model
- `GET /api/v1/admin/providers/{id}/models` endpoint for provider's model catalog
- Seed data populates provider_models for all built-in providers

## [0.11.0] - 2026-05-01

### Added
- Weighted round-robin channel routing — channels at the same priority tier distribute traffic proportionally by weight (default 100)
- Weight configuration on channel create/edit forms
- Weight display on channel list and detail pages

## [0.10.7] - 2026-05-01

### Added
- Real-time availability indicator on channel list page — shows "Available" (green) or "Outside Hours" (gray) based on current UTC time against channel schedule

## [0.10.6] - 2026-05-01

### Fixed
- Channel list API now includes `available_hours` in response (was missing from `ChannelWithModels`, causing list to always show "24/7")
- Improved model badge and day abbreviation font sizes on channel list page for readability

## [0.10.5] - 2026-05-01

### Fixed
- Channel list page now shows detailed available hours (time ranges and days) instead of schedule count

## [0.10.4] - 2026-05-01

### Added
- Show available hours indicator on channel list page (schedule count or 24/7)

## [0.10.3] - 2026-05-01

### Added
- Show channel name on audit log list page and detail drawer (LEFT JOIN channels instead of showing truncated UUIDs)

### Fixed
- Model card pricing now correctly converts from subunits to USD
- Channel detail page refreshes after editing available hours

## [0.10.2] - 2026-05-01

### Fixed
- Home page curl example now correctly displays full URL with configured server host

## [0.10.1] - 2026-05-01

### Fixed
- Validate database driver at startup — unknown values now fail with clear error instead of silently falling back to SQLite
- About tab no longer hardcodes "SQLite", reads actual driver from config

## [0.10.0] - 2026-05-01

### Added
- Channel Available Hours — restrict channels to specific days and time ranges (UTC), with routing automatically filtering out channels outside their scheduled hours
- Frontend Available Hours card on Channel Detail page with day toggles and time inputs

### Fixed
- Clear schedule now works correctly (send `[]` to clear — `Option<Option<Vec<TimeSlot>>>` bug fixed to single Option)

## [0.9.7] - 2026-04-30

### Fixed
- Seed pricing policies deserialization — camelCase JSON keys now match `#[serde(rename_all = "camelCase")]`
- Pricing policy seeding decoupled from model seeding (independent table check)
- Reduced seed pricing policies to glm-5.1, minimax-m2.7, minimax-m2.7-highspeed only

## [0.9.6] - 2026-04-30

### Fixed
- Seed models loaded independently from providers (N:N model-provider architecture)

## [0.9.5] - 2026-04-30

### Fixed
- Version passed explicitly via build arg in Dockerfile
- Settings test updated for version-agnostic matching

## [0.9.4] - 2026-04-30

### Added
- Monetary integer subunits — all money values stored as integer microdollars (1 USD = 1,000,000 units) to eliminate floating-point errors
- `money` module with `usd_to_units` / `units_to_usd` / `bps_to_ratio` / `ratio_to_bps` conversion helpers
- SQLite and PostgreSQL migrations to convert existing monetary columns to INTEGER/BIGINT
- API boundary conversion: management handlers accept/return USD floats, storage layer uses i64 integers
- Billing, settlement, and workers updated to integer arithmetic throughout

### Fixed
- PostgreSQL type compatibility (BIGINT for SUM aggregates, TIMESTAMPTZ for timestamps)
- PostgreSQL migrations made idempotent for existing databases
- Context-tiered billing support in frontend pricing display

## [0.9.3] - 2026-04-29

### Fixed
- Quote reserved keyword `window` in PostgreSQL rate_limit_counters query

## [0.9.2] - 2026-04-29

### Fixed
- Correct PostgreSQL 18 data path in production docker-compose
- Use list form for `depends_on` in docker-compose

## [0.9.1] - 2026-04-29

### Fixed
- Add version field to all docker-compose files

## [0.9.0] - 2026-04-29

### Added
- Cache creation pricing support
- Key prefix display in API key list
- Sidebar improvements and font switch (Outfit + JetBrains Mono)

## [0.8.3] - 2026-04-28

### Changed
- Redesigned Audit Logs page with consistent design patterns: animated header, section cards, improved filter bar with active count badge, proper table styling, and detail drawer with structured sections

## [0.8.2] - 2026-04-28

### Added
- Admin Settings page rebuilt with tabbed layout (General, Security & Audit, System Info, About)
- System Info tab showing infrastructure configuration reference
- About tab with version and GitHub link

### Fixed
- Version display showing "vv0.8.0" instead of "v0.8.0" in sidebar, header footer, home page, and settings

## [0.8.0] - 2026-04-27

### Added
- `user_id` column on `usage_records` and `audit_logs` tables (denormalized from `api_keys.created_by`)
- Account balance card on Dashboard with low-balance warning
- User-scoped audit log queries (non-admin users can now view their own logs)
- `query_usage_cost_by_user` storage method for efficient settlement

### Changed
- Usage API (`/api/v1/usage`, `/api/v1/usage/summary`) now properly scopes data to the current user for non-admin requests (was returning all users' data)
- Settlement worker replaced N+1 key-lookup loop with single `GROUP BY user_id` query
- Usage page key filter dropdown now only shows keys belonging to the current user

## [0.7.0] - 2026-04-27

### Added
- Runtime database driver selection (PostgreSQL or SQLite via `config.toml`)
- Docker image build and push on CI release (GHCR with semver tags)
- Production docker-compose with PostgreSQL 18
- `useReducedMotion` hook — respects `prefers-reduced-motion` system preference
- Global CSS reduced-motion media query
- GLM seed data with Anthropic and OpenAI endpoint URLs
- Keyboard navigation and focus-visible rings on model cards

### Changed
- **Home page redesign**: fixed nav, value-driven hero, 3-step flow, terminal-style quick start, CTA section
- **Dashboard redesign**: animated metric cards, server-side usage summary (replaces client-side aggregation), loading skeletons, status pills
- **Models page**: active card redesign — clean neutral styling, emerald status badge, clickable cards with keyboard support, form label accessibility
- Body text across Home/Dashboard/Models bumped to 16px for readability

### Fixed
- PostgreSQL storage module synced with current data model (removed stale fields)
- Removed background glow animations that bypassed reduced-motion

## [0.6.1] - 2026-04-26

### Fixed
- Update EndpointsEditor test to expect `default` as first protocol

## [0.6.0] - 2026-04-26

### Added
- Provider proxy URL — route upstream requests through configurable HTTP proxy (`proxy_url` field on providers)
- Audit log detail endpoint (`GET /api/v1/admin/logs/{id}`) for fetching full request/response bodies on demand
- Git flow workflow documented in CLAUDE.md

### Changed
- Audit log list API now returns `AuditLogSummary` (excludes `request_body` and `response_body`) for performance
- EndpointsEditor protocol options: `default`, `openai`, `anthropic` (removed azure, google, custom)
- Token storage normalized across protocols (see CLAUDE.md "Token Storage Convention")

### Fixed
- Return `ProviderWithEndpoints` from create/update provider handlers (endpoints were blank after save)
- Add `default` endpoint key to provider forms
- Proxy `/v1` routes in Vite dev server
- SQLite compatibility and font scaling improvements

## [0.5.1] - 2026-04-24

### Fixed
- Use `type` column name in transactions SQL for SQLite compat

## [0.5.0] - 2026-04-22

### Added
- Initial release with OpenAI and Anthropic compatible endpoints
- API key management, provider/channel configuration, billing, rate limiting
- React frontend with dashboard, logs, usage tracking
## [1.3.8] - 2026-05-05

### Fixed
- Fix compilation errors in integration tests (stale AppState fields)

## [1.3.7] - 2026-05-05

### Changed
- Language switch now shows current language code (EN/中) instead of bare icon

## [1.3.6] - 2026-05-05

### Fixed
- Admin pages no longer redirect to console dashboard on page refresh

## [1.3.5] - 2026-05-05

### Fixed
- Move NATS status pills from user dashboard to admin dashboard
