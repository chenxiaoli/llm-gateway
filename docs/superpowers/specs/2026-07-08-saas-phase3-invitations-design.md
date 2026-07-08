# SaaS Multi-Tenant Phase 3 — Wizard-Gated Signup + Invitations Design

**Date:** 2026-07-08
**Status:** Draft (pending user review)
**Targets release:** v1.11.0
**Predecessors:** [SaaS Multi-Tenant (Orgs) Design](./2026-07-07-saas-multi-tenant-orgs-design.md) (parent spec, esp. Decision 7), Phase 1 / Phase 2.1 / 2.2 / 2.3 (all merged to develop).

## Problem

Phases 1 and 2 made the gateway multi-tenant at the data and routing layers. Every user belongs to one or more orgs, all management routes are scoped under `/api/v1/{org_slug}/...`, and the frontend lives at `/{orgSlug}/*`. But the signup flow still assumes the Phase 1 world:

- `POST /api/v1/auth/register` creates a user but does not put them in any org (Phase 1 stopped auto-assigning the default org when default-org bootstrapping was scoped to the migration only).
- A brand-new user lands in the frontend with zero orgs. Every `:orgSlug` route is physically inaccessible — there is no slug to navigate to. They see a broken app.
- The only way today to add a member is `POST /api/v1/{org_slug}/members` invite-by-username, which 404s if the username doesn't exist yet. So an admin in Org A cannot invite a coworker who hasn't already signed up.

The original parent spec sketched Phase 3 as "signup auto-creates a personal org + invitations." After brainstorming, we are shipping a different shape (see Decisions Locked below). This spec replaces the Phase 3 section of the parent spec; the parent spec's Phase 1 and Phase 2 sections stand.

## Goal

Make signup and invitations first-class. After Phase 3:

- A brand-new user signs up, sees a wizard, and ends the wizard as the owner of a freshly-created org OR as a member of an org they were invited to. They never see a "no org" broken state in the management UI.
- An admin in Org A can mint a single-use magic invite link. Anyone (logged-in or not) who presents the link to the gateway can sign up + auto-join Org A in one flow.
- The username-based invite path from Phase 2.2 still works for adding already-existing users to a new org. The two paths coexist.

## Decisions Locked (from brainstorming)

These override or refine the parent spec's Phase 3 sketch.

1. **Wizard-first, no auto-create.** A brand-new user is NOT auto-assigned a personal org on signup. They land at `/onboarding` and must explicitly choose to either create an org or join one via invite. They cannot reach any `:orgSlug` route until they complete the wizard. *Overrides parent spec Decision 7 ("Signup auto-creates a personal org").*
2. **Two wizard branches.** "Create org" and "Join via invite." No personal-vs-work distinction — the first org the user creates is just an org; they can rename it later. *Simplifies the parent spec's three-branch sketch.*
3. **Slug collisions are rejected, not auto-suffixed.** The Create-org form pre-fills the slug from the username and live-validates. On submit, if the slug is taken, the field shows "that slug is taken" and the user picks another. *Overrides the parent spec's `-2 / -3` auto-suffix wording.*
4. **Generic single-use magic links for invitations.** An invitation token is NOT bound to a target identity. The first person to present the token (via signup or login) consumes it and joins the inviting org. Single-use, 7-day expiry. *Simpler than username-bound; acceptable B2B threat model.*
5. **Invite-aware landing page.** A logged-out clicker of `/accept-invite?token=...` sees a public page showing "You've been invited to join {org} as {role}" with Sign-up / Log-in buttons. After auth, they auto-accept. A logged-in clicker sees Accept / Decline. Wizard is bypassed whenever a valid invite is accepted at signup.
6. **No email delivery (still).** Admins copy the invite URL from the UI and share it out-of-band (Slack, etc.). Email verification, SMTP, and email-based token delivery remain explicitly out of scope.

## Architecture

### Signup flow + JWT shape

`POST /api/v1/auth/register` is modified. Today it creates a user and returns a JWT. After Phase 3:

- New user row: `current_org_id = NULL`.
- Returned JWT: `current_org_id: None`. `orgs: []`.
- User is now in **limbo** — authenticated, but with no org to operate in.

JWT shape is structurally unchanged. `JwtClaims.current_org_id` was already `Option<String>`; it is simply permitted to be `None`. All existing token-validation code keeps working. Existing users (from Phase 1 migration) have non-null `current_org_id` and are unaffected.

**Frontend behavior in limbo:**
- A React Router guard in `<RequireAuth/>`: if `user.orgs.length === 0` AND not on `/onboarding` or `/accept-invite`, redirect to `/onboarding`.
- Existing `:orgSlug` routes are physically inaccessible — there is no slug to navigate to. The guard also catches direct URL visits.
- After wizard completion (either branch), the relevant endpoint returns a reissued JWT with `current_org_id` set; the frontend redirects to `/{slug}/dashboard`.

**Backend enforcement:**
- The existing middleware chain `AuthLayer → OrgResolveLayer → MembershipLayer` already rejects limbo users on `:orgSlug` routes (no membership → 403). The frontend guard is the friendly version; the backend is the enforcer.
- A new endpoint `GET /api/v1/me/onboarding` returns `{ needs_onboarding: bool }` so the frontend can pick the right initial route during bootstrap.

### Onboarding wizard

**Route:** `/onboarding`. Requires authenticated session. Limbo-only.

**Entry conditions:**
- After signup without invite → frontend guard routes here automatically.
- Direct navigation to `/onboarding` while in limbo → wizard renders.
- Page reload while in limbo → bootstrap re-routes to `/onboarding`.

**Exit conditions:**
- Create branch succeeds → JWT reissued, redirect to `/{slug}/dashboard`.
- Join branch succeeds → token consumed, JWT reissued, redirect to `/{inviting org slug}/dashboard`.
- User logs out → back to `/login`.

**Two-branch UI:**

```
┌─────────────────────────────────────────────┐
│  Welcome to TokenVis. Let's set you up.     │
│                                             │
│  ┌─────────────────┐  ┌─────────────────┐   │
│  │  Create an org  │  │ Have an invite? │   │
│  │   (1 min form)  │  │  Paste link/token│  │
│  └─────────────────┘  └─────────────────┘   │
└─────────────────────────────────────────────┘
```

**Create branch** (single form, no personal-vs-work distinction):
- Fields: `name` (required), `slug` (required, pre-filled from `username`, editable, live-validated).
- Submit: `POST /api/v1/orgs` (existing Phase 2.2 endpoint, already org-agnostic). On 409 (slug collision) → field error "that slug is taken".
- Backend then sets `users.current_org_id = new_org.id` and reissues the JWT (new behavior — Phase 2.2 did not auto-switch current_org on create).
- Frontend redirects to `/{slug}/dashboard`.

**Join branch:**
- Single field: paste invite token or full URL. Frontend extracts `?token=` if URL pasted.
- Submit: `POST /api/v1/invitations/accept` with `{ token }`.
- Backend validates token, looks up org + role, creates Member row, marks invitation consumed, reissues JWT with `current_org_id = inviting org`.
- Frontend redirects to `/{inviting org slug}/dashboard`.

**Skip / dismiss:** no skip. The user MUST complete one branch to leave the wizard. Closing the tab and reloading returns to the wizard.

### Invitation tokens (schema + lifecycle)

**New table `invitations`:**

```sql
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
    revoked_at      TIMESTAMPTZ
);
CREATE INDEX invitations_org_id_pending_idx ON invitations (org_id)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;
```

**Field rationale:**
- `token` is the lookup key. 32 bytes (256 bits) of CSPRNG entropy, base64url-encoded. Opaque (not a JWT) so revocation is a row update, not a crypto problem.
- `role` is constrained to `member` / `admin` at the DB level via a new CHECK constraint `CHECK(role IN ('member', 'admin'))` on `invitations.role` (the existing `members.role` column has `CHECK(role IN ('owner','admin','member'))`; `invitations.role` deliberately drops `owner`). Owner-via-invite is forbidden — owner is a Phase 2.2 self-promotion flow only.
- `expires_at` is checked lazily at accept time. No background sweeper job — keeps the schema simple. A 1-year-old accepted row is just audit data.
- `accepted_by` lets the admin UI show "accepted by alice on 2026-07-10". `ON DELETE SET NULL` preserves the audit trail if the user is later deleted.
- The partial index keeps the admin's "pending invites" list fast even after the table accumulates history.

**Lifecycle:**
1. **Mint:** admin POSTs `/api/v1/{org_slug}/invitations` with `{ role }`. Backend generates token, inserts row, returns `{ id, token, url, role, expires_at, created_at }`. The URL is constructed server-side so it is correct regardless of where the admin views it.
2. **Share:** admin copies the URL from the UI, shares via Slack/email/whatever. Phase 3 has no email delivery — explicitly out of scope.
3. **Accept:** recipient POSTs `/api/v1/invitations/accept` with `{ token }`. Single SQL transaction: `SELECT ... FOR UPDATE`, validate `accepted_at IS NULL AND revoked_at IS NULL AND expires_at > NOW()`, insert `members` row, set `accepted_at = NOW()`, `accepted_by = user.id`. JWT is reissued with `current_org_id` = inviting org.
4. **Revoke:** admin DELETE `/api/v1/{org_slug}/invitations/{id}`. Sets `revoked_at = NOW()`. Irreversible.
5. **List:** admin GET `/api/v1/{org_slug}/invitations` returns pending + recently-accepted (last 30 days) for an audit view.

**Concurrency:** `SELECT FOR UPDATE` in the accept transaction means two simultaneous clicks on the same token serialize. First wins; second gets 409 "already accepted".

**Rate limiting:** token generation is admin-only (gated by `can_administer`), so no extra throttle. Token *acceptance* is auth-gated (logged-in or signed-up-in-same-session). The public preview endpoint is not separately rate-limited — see "Edge cases & security" below; 256-bit token entropy is the sole defense.

**Cleanup:** old accepted/expired/revoked rows are retained for audit. A future janitor could prune >1-year-old rows; not in Phase 3.

### Accept-invite flow

**Public route:** `/accept-invite?token=...`. No auth required to render.

**Backend endpoint:** `POST /api/v1/invitations/accept` with body `{ token: string }`. Auth required (Bearer JWT). Single transaction.

**Logged-out clicker experience:**

```
┌──────────────────────────────────────────────┐
│  You've been invited to join Acme Corp        │
│  as a member.                                 │
│                                              │
│  [ Sign up to accept ]   [ Log in ]          │
│                                              │
│  Invite from bob · expires in 6 days         │
└──────────────────────────────────────────────┘
```

- The landing page calls `GET /api/v1/invitations/preview?token=...` (public, no auth) to fetch `{ org_name, org_slug, role, inviter_username, expires_at, already_member }`. Renders the banner.
- If the token is invalid/expired/revoked/already-accepted → preview returns 410 Gone with a reason; page shows "This invitation is no longer valid" + a contact-the-admin message.
- "Sign up to accept" → `/register?invite=...` → on successful register, frontend auto-calls `POST /api/v1/invitations/accept` with the stashed token, then redirects to `/{org_slug}/dashboard`.
- "Log in" → `/login?next=/accept-invite%3Ftoken%3D...` → after login, lands back on `/accept-invite?token=...`, now in the logged-in branch.

**Logged-in clicker experience:**

```
┌──────────────────────────────────────────────┐
│  You've been invited to join Acme Corp        │
│  as a member.                                 │
│                                              │
│  You're currently signed in as alice in       │
│  Globex.                                     │
│                                              │
│  [ Accept ]               [ Decline ]         │
└──────────────────────────────────────────────┘
```

- Same preview call.
- "Accept" → `POST /api/v1/invitations/accept` with token → membership added, JWT reissued with `current_org_id = inviting org` → redirect to `/{org_slug}/dashboard`.
- "Decline" → close tab or navigate away. Token stays pending; the inviter can revoke if they want.

**Edge cases:**
- Logged-in user is already a member of the inviting org → preview returns `already_member: true`; page shows "You're already a member of Acme Corp" + link to switch to it.
- Logged-in user is in limbo (signed up moments ago, no org yet) → same as logged-out-clicker experience minus the "log in" button. Accepting escapes limbo. This is the wizard's Join branch in disguise.
- Token in URL is malformed/missing → "Invalid invitation link" + link to `/login`.
- User signs up via `/register?invite=...`, completes signup, but invitation was consumed in the same 5 seconds by someone else → accept endpoint returns 409; frontend shows "This invitation was just used. Please request a new link."

**Token-in-URL safety:** the token is a query param, so it appears in browser history, referrer headers, etc. This is the standard magic-link trade-off and is acceptable for B2B invite flows where the link is shared privately. We add `Referrer-Policy: no-referrer` to the accept-invite page to limit leakage.

## API surface (delta from Phase 2.3)

**New endpoints (5):**

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `POST` | `/api/v1/{org_slug}/invitations` | admin+ | Mint token. Body `{ role }`. Returns `{ id, token, url, role, expires_at, created_at }`. |
| `GET` | `/api/v1/{org_slug}/invitations` | admin+ | List pending + recently-accepted (30d). |
| `DELETE` | `/api/v1/{org_slug}/invitations/{id}` | admin+ | Revoke. 404 if not in this org. |
| `GET` | `/api/v1/invitations/preview` | public | Query `?token=...`. Returns org name/slug, role, inviter, expiry, already_member. 410 if invalid/expired/consumed/revoked. No per-IP rate limit (256-bit token entropy is the sole defense). |
| `POST` | `/api/v1/invitations/accept` | user | Body `{ token }`. Single transaction. Returns reissued JWT + member row. 409 if already accepted (race). |

**New endpoint (1) — onboarding state:**

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `GET` | `/api/v1/me/onboarding` | user | Returns `{ needs_onboarding: bool }` — true if `user.orgs.length === 0`. Used by frontend bootstrap to decide between `/:orgSlug/dashboard` and `/onboarding`. |

**Modified endpoints (2):**
- `POST /api/v1/auth/register` — creates user with `current_org_id = NULL`. No other change.
- `POST /api/v1/orgs` — when called by a limbo user (`current_org_id IS NULL`), after creating the org, also set `users.current_org_id = new_org.id` and reissue JWT with the new `current_org_id`. When called by a user who already has orgs, behaves as today (creates org, user becomes owner, `current_org_id` is NOT switched — they can switch via OrgSwitcher later).

**Existing endpoint staying put:**
- `POST /api/v1/{org_slug}/members` (Phase 2.2 invite-by-username) — unchanged. This is the "user already exists" path; invitations is the "user doesn't exist yet" path. Both coexist.

**Removed/changed behaviors:** none. All existing Phase 2.3 endpoints keep their contracts.

**Type updates:**
- Backend `crates/storage/src/types.rs`: add `Invitation`, `InvitationPreview`, `CreateInvitationRequest`, `AcceptInvitationRequest`.
- Storage trait: add `create_invitation`, `get_invitation_by_token`, `list_invitations_for_org`, `revoke_invitation`, `accept_invitation` (the last is transactional and updates both `invitations` and `members`).
- Frontend `web/src/types/index.ts`: mirror the new types.
- Frontend `web/src/api/invitations.ts` (new): wrap the 5 invitation endpoints + preview.

## Frontend routes, components, and testing

**New routes (3):**

| Path | Component | Access |
|---|---|---|
| `/onboarding` | `<OnboardingPage/>` | Authenticated; limbo-only (frontend guard redirects here from any non-onboarding route if `user.orgs.length === 0`) |
| `/accept-invite` | `<AcceptInvitePage/>` | Public (renders different UI for logged-out vs logged-in) |
| `/:orgSlug/settings/invitations` | `<InvitationsPage/>` | Admin+ in the org |

**Modified routes:**
- `/register` — accepts optional `?invite=...` query param. Stashes token; on successful register, calls `/invitations/accept` before redirecting.
- `/login` — `?next=` param accepts `/accept-invite?token=...` (already does; verifying it survives a redirect-encoded query string).

**New components:**
- `<OnboardingPage/>` — the two-branch wizard.
  - `<CreateOrgCard/>` — name + slug form with live validation, calls `POST /orgs`.
  - `<JoinByInviteCard/>` — single token/URL field, calls `POST /invitations/accept`.
- `<AcceptInvitePage/>` — public preview + Sign-up/Log-in or Accept/Decline depending on auth state.
- `<InvitationsPage/>` — admin view. Lists pending invitations with revoke buttons + "Generate new invitation" form (role select + generate button + copyable URL display).
- `<CopyableInviteLink/>` — small reusable component that shows the URL with a copy button + "expires in N days" badge.

**Auth store changes (`useAuthStore`):**
- Add `pendingInviteToken: string | null` (transient — stashed during the register flow if `?invite=...` was present).
- Add `needsOnboarding()` selector: returns `user.orgs.length === 0`.
- Bootstrap flow: after `useAuthBootstrap`, if `needsOnboarding()` and not on `/onboarding` or `/accept-invite` → navigate to `/onboarding`.

**Routing guard:**
```typescript
// in <RequireAuth/> wrapper
if (user && user.orgs.length === 0 && !isOnOnboardingOrAcceptInvite(location)) {
  return <Navigate to="/onboarding" replace />;
}
```

**i18n:** keys under `onboarding.*`, `acceptInvite.*`, `invitations.*` in `web/src/i18n/en.json` + `zh.json`.

**Testing:**

Backend (Rust):
- `invitations` table migration self-check (token uniqueness, FK cascade on org delete).
- Unit tests for token generation (length, entropy, no duplicates in 10k samples).
- Integration tests for the 5 new endpoints (mint, list, revoke, preview, accept) — happy path + failure modes (expired, revoked, already-accepted, wrong-org admin, non-admin mint, malformed token).
- Limbo-state test: register → JWT has null `current_org_id` → `:orgSlug` route returns 403 → after `POST /orgs` → JWT reissued with org.
- Concurrency test for accept: two simultaneous accepts, exactly one wins, the other gets 409.

Frontend (Vitest + MSW):
- `<OnboardingPage/>` test: create branch success, slug collision → error, join branch success.
- `<AcceptInvitePage/>` test: logged-out preview, logged-in preview, expired token, accept success.
- `<InvitationsPage/>` test: list rendering, generate flow, revoke.
- Routing guard test: limbo user redirected to `/onboarding`.

E2E (Playwright): full happy path — signup → wizard → create org → land in dashboard. Plus invite flow — admin mints token → second user signs up via link → both end up in same org.

**Migration:** one new file `crates/storage/migrations/postgres/20260710000000_invitations.sql` with the table + indexes above, plus a `20260710000000_invitations.down.sql` for rollback. No data backfill (no invitations exist yet). Existing users unaffected.

## Edge cases & security

- **Token enumeration:** preview endpoint returns identical 410 responses for invalid / expired / revoked / already-accepted tokens. Combined with 256-bit entropy, brute-force enumeration is computationally infeasible (on the order of 10^38 average attempts per valid token). No per-IP rate limit is added in Phase 3; this can be revisited if any abuse pattern emerges.
- **Privilege escalation via role field:** `role` column has a DB-level CHECK constraint `CHECK(role IN ('member','admin'))` (note: 'owner' deliberately excluded). Owner-via-invite is impossible at the storage layer.
- **Cross-org token use:** the accept endpoint looks up the token's `org_id` directly; it does NOT consult the URL slug. A user could be invited to Org A and accept while their current_org is Org B — this is fine, the new membership is added to Org A and current_org is switched to A.
- **Self-invite:** an admin could mint a token and accept it themselves to "switch orgs." Pointless (they could just use OrgSwitcher), but harmless. No special handling.
- **Invitation spam:** admins in a single org can mint unbounded tokens. The existing `can_administer` check is the only gate. A future per-org quota could be added if abuse emerges; not in Phase 3.
- **Limbo user calling `POST /api/v1/orgs`:** must succeed (it is the wizard's Create branch). The endpoint already requires only "authenticated user," not "current_org_id set." Confirm during implementation that the layer chain does not 403 limbo users on this path.

## Out of Scope / Future Work

- **Email delivery** — admins copy URLs manually. SMTP integration, email verification, and email-based token delivery remain explicitly out of scope.
- **Email-bound invitations** — Phase 3 ships generic magic links. Username/email-bound tokens (more secure, less prone to link leakage) are a Phase 4+ candidate.
- **Multi-use invitations** — every token is single-use. Batch invites (one token, N uses) are a Phase 4+ candidate.
- **Onboarding skip / deferral** — wizard is hard-gated. A future "remind me later" could allow deferral; not in Phase 3.
- **Invitation expiry sweep** — accepted/expired/revoked rows are retained for audit. A future cleanup job could prune old rows.
- **In-app notification of pending invite** — N/A; Phase 3 has no email and no in-app notification system. Admins must share URLs out-of-band.

## Verification checklist

- [ ] Migration `20260710000000_invitations.sql` applies cleanly on a Phase 2.3 database; down migration reverses it.
- [ ] `POST /api/v1/auth/register` returns a JWT with `current_org_id: null` and `orgs: []`.
- [ ] A limbo user hitting any `:orgSlug` management route receives 403 from the backend and is redirected to `/onboarding` by the frontend guard.
- [ ] `POST /api/v1/orgs` called by a limbo user creates the org AND sets `users.current_org_id` AND returns a reissued JWT.
- [ ] Slug collision on `POST /api/v1/orgs` returns 409 with a clear error; user can retry with a different slug.
- [ ] Wizard's Join branch consumes a token, adds membership, reissues JWT, redirects to the inviting org's dashboard.
- [ ] `POST /api/v1/{org_slug}/invitations` mints a token; only admin+ can call (403 for member).
- [ ] `GET /api/v1/invitations/preview?token=...` returns org metadata for a valid pending token; returns 410 with identical body for invalid / expired / revoked / accepted.
- [ ] `POST /api/v1/invitations/accept` is single-transaction; concurrent accepts of the same token result in exactly one 200 and one 409.
- [ ] `DELETE /api/v1/{org_slug}/invitations/{id}` revokes; subsequent accept returns 410.
- [ ] A logged-out clicker of `/accept-invite?token=...` sees the invite-aware landing page; signing up via the page auto-accepts.
- [ ] A logged-in clicker of `/accept-invite?token=...` sees Accept / Decline; accepting switches current_org.
- [ ] `Referrer-Policy: no-referrer` is set on `/accept-invite`.
- [ ] `member_role` CHECK on `invitations.role` (excluding `owner`) is enforced at the DB level; inserting an invitation with role='owner' fails.
- [ ] All Rust tests pass; all frontend Vitest tests pass; Playwright invitation E2E passes.

## Phasing

Phase 3 is shipped as a single release (v1.11.0). The implementation plan will likely decompose into:

1. **Schema + types** — migration, storage types, storage trait methods.
2. **Invitation CRUD backend** — mint / list / revoke endpoints + tests.
3. **Invitation accept + preview backend** — accept transaction + public preview endpoint + tests.
4. **Register + `POST /orgs` modifications** — limbo state, current_org_id reissue.
5. **Onboarding wizard frontend** — page, components, routing guard.
6. **Accept-invite frontend** — public landing page, logged-in/out branches.
7. **Invitations admin page frontend** — list, generate, revoke.
8. **E2E + integration tests** — full flow coverage.

Each task is independently shippable to develop on a feature branch.
