# Users → Members Refactor Design

## Goal

Eliminate the `Users` page and its concept. Fold its useful capabilities (balance, recharge, usage, enable/disable) into the per-org `Members` page. Migrate the `accounts` table from per-user to per-membership so that balance/threshold/transactions are scoped to (user, org), matching the system's conceptual model.

## Mental Model

- **Platform scope** administers **orgs** and platform-level grants. Existing pages: `Settings`, `Platform Users`. Unchanged by this work.
- **Org scope** administers **members** of a single org: their role (owner/admin/member), group, balance, usage, and enable/disable status. Existing page: `Members`. This page absorbs capabilities previously on `Users`.
- **"Users" as a standalone page** disappears from every nav group. The `users` table remains the global identity store; it just stops being directly administered through a dedicated page.

## Context

The current `/{slug}/admin/users` page conflates platform-level user identity with org-level membership. Its backend handler (`crates/api/src/management/users.rs`) already carries a TODO comment:

```rust
// TODO(Task 11): the frontend should call a dedicated /members endpoint
// rather than piggybacking on /users).
```

The handler uses `OrgContext` and joins `users` to `members` for the current org, then exposes fields that mix both: username (global), role (per-org), group (per-org), enabled (global), balance (global today, per-org after this work), created (global).

Today's `accounts` table is 1:1 with `users`:

```sql
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    balance BIGINT NOT NULL DEFAULT 0,
    threshold BIGINT NOT NULL DEFAULT 100000000,
    currency TEXT NOT NULL DEFAULT 'USD',
    ...
);
```

This forces balance to be a single global number per user — wrong if a user belongs to multiple orgs with separate budgets.

## Architecture

### Sidebar groups (after)

| Group | Audience | Items |
|---|---|---|
| Console | All users | (unchanged) |
| Admin | Org admins (`isAdminOrAbove`) | dashboard, channels, providers, models, pricing-policies, **members (expanded)**, invitations, groups, logs |
| Platform | Platform admins (`isPlatformAdmin`) | settings, platform-users (both unchanged) |

`Users` is removed from every group.

### Page roster

| Page | Action |
|---|---|
| `web/src/pages/Users.tsx` | **Delete.** |
| `web/src/pages/Members.tsx` | **Expand.** Add columns: balance, status (enabled/disabled toggle). Add row actions: detail drawer (combines current UserDrawer capabilities — recharge, adjust, recent transactions), usage drawer. The existing role-change and remove actions stay. |
| `web/src/pages/AccountBalance.tsx` (`/{slug}/admin/users/:userId/balance`) | **Move** to `/{slug}/admin/members/:userId/balance`. Same component; just the URL changes. The link from Members row lives inside the detail drawer. |
| `web/src/pages/PlatformUsers.tsx` | Unchanged. |
| `web/src/pages/Settings.tsx` | Unchanged. |

### Backend route changes

**Removed handlers** (`crates/api/src/management/users.rs`):
- `list_users`, `update_user`, `delete_user`

**Removed routes** (`crates/api/src/management/mod.rs`):
- `GET    /admin/users`
- `PATCH  /admin/users/{id}`
- `DELETE /admin/users/{id}`

**Moved routes** (same handlers, new path prefix):
- `GET    /admin/users/{id}/balance`   → `GET    /admin/members/{user_id}/balance`
- `POST   /admin/users/{id}/recharge`  → `POST   /admin/members/{user_id}/recharge`
- `POST   /admin/users/{id}/adjust`    → `POST   /admin/members/{user_id}/adjust`
- `PATCH  /admin/users/{id}/threshold` → `PATCH  /admin/members/{user_id}/threshold`

These four handlers (`crates/api/src/management/accounts.rs`) get updated to look up the account by `(user_id, ctx.org_id)` instead of by `user_id` alone.

**Expanded handler** (`crates/api/src/management/members.rs:list_members`):
- Return shape gains `balance`, `threshold`, `enabled`, `email`, `created_at` per member row. Storage method `list_members` already joins `users`; extend the SELECT to also pull the per-membership account row.

**New handlers** in `members.rs`:
- `PATCH /admin/members/{user_id}` (currently only `change_member_role`) expands to also accept `enabled` (the user-row toggle) — moves the enable/disable capability off the deleted `update_user` handler.
- Member deletion (currently `DELETE /admin/members/{user_id}` → `remove_member`) keeps its current semantics: revoke membership in this org. The destructive "delete user account entirely" operation moves to platform scope as a separate follow-up (see Out of Scope).

### Storage layer changes

`crates/storage/src/postgres.rs` and `crates/storage/src/types.rs`:

- Add `org_id` column to `accounts`. Drop `UNIQUE` on `user_id`; add `UNIQUE(user_id, org_id)`. Add FK to `orgs(id) ON DELETE CASCADE`.
- Backfill: for each existing account, set `org_id` to one of the user's memberships (see Backfill Strategy below for the multi-org case).
- Update storage trait methods:
  - `get_account(user_id)` → `get_account(user_id, org_id)`
  - `recharge`, `adjust`, `set_threshold` — same signature change
  - `list_users_paginated(org_id)` → **delete** (replaced by `list_members` expansion)
  - `list_members(org_id)` → return enriched shape with balance/threshold/enabled
- Account-creation logic: when `invite_member` or `accept_invite` creates a membership row, also create the matching `accounts` row with default threshold.

### Frontend plumbing

- `web/src/api/users.ts` — **delete**.
- `web/src/hooks/useUsers.ts` — **delete**.
- `web/src/api/members.ts` — **expand**: add `getMemberBalance`, `rechargeMember`, `adjustMember`, `setMemberThreshold`, `updateMember` (for enable/disable).
- `web/src/hooks/useMembers.ts` — **expand**: add the corresponding hooks. Drop slug from query keys where applicable (Members page is already org-scoped through the URL).
- `web/src/api/accounts.ts` — **rename** endpoints from `/admin/users/{id}/...` to `/admin/members/{user_id}/...`. (File rename optional.)
- `web/src/components/Layout.tsx` — remove `users` from `adminItems`; the `members` entry stays where it is.
- `web/src/App.tsx` — remove `admin/users` and `admin/users/:userId/balance` routes; add `admin/members/:userId/balance`; add redirect from old URLs.
- `web/src/i18n/{en,zh}.json` — remove `sidebar.users` key. Keep `sidebar.members`.
- `web/src/test/server.ts` — remove `/admin/users` handlers; add `/admin/members/:userId/balance|recharge|adjust|threshold` handlers; expand the existing `/admin/members` GET handler to include the new fields.
- Tests that import from `pages/Users` or `api/users` — rewrite to use Members.

### Test strategy

- **Backend**: existing tests in `crates/api/tests/test_users.rs` move to `test_members.rs` (the file already exists; we extend it). Cover: enable/disable via `PATCH /members/{user_id}`, balance/recharge/adjust under the new route prefix, last-admin guard for enable/disable. `test_accounts.rs` (if it exists) updates URLs. Storage-layer tests for the new account-lookup-by-(user,org) signature.
- **Frontend**: `Members.test.tsx` expands to cover the new columns, drawers, and actions. `Users.test.tsx` is deleted. `AccountBalance.test.tsx` updates the URL it mounts at. `Layout.test.tsx` updates to assert `Users` is gone and `Members` is present.
- **Migration test**: a migration test that seeds an old-shape `accounts` row, runs the migration, and asserts the new shape is correct including the backfill choice.

## Backfill Strategy

The non-trivial part. Existing rows in `accounts` are 1:1 with `users`. After migration, accounts become 1:1 with memberships. For users with a single org membership, the backfill is unambiguous: copy `org_id` from the membership row.

For users with multiple org memberships, the current single balance number has no clear "owner" org. Three options:

1. **Assign to the oldest membership.** Backfill `accounts.org_id` to the membership with the earliest `created_at`. Other orgs start at balance 0. Conservative; one org "inherits" the prior balance.
2. **Assign to the user's primary org.** Add a `is_primary` flag to `members` (or use the first membership) and backfill to that. Requires a "primary org" concept that doesn't exist today.
3. **Duplicate to every membership.** Each of the user's orgs starts with the full prior balance. Inflates total system liability; probably wrong.

Recommend **Option 1** (oldest membership inherits) as the default. It's deterministic, requires no schema additions, and matches the intuition that the user's "first" org is their primary financial context. Operators with multi-org users will need to manually reconcile after the migration — call this out in the CHANGELOG.

The migration ships with a runtime guard: a startup check that fails loudly if any `accounts` row lacks an `org_id` after backfill, so a botched migration can't silently produce NULL org_id accounts.

## Open Questions

These can be decided during planning or implementation; none should block the spec.

1. **Enable/disable scope.** Currently `users.enabled` is a global flag — disabling a user disables them across all orgs. Is that still the desired behavior, or should `enabled` migrate to `members` too? (Recommend: keep global. A disabled account can't log in; that's a platform-level concern, not per-org.)
2. **AccountBalance page deep-link.** Should the URL be `/{slug}/admin/members/:userId/balance` (consistent with members route prefix) or stay as a drawer inside Members? (Recommend: keep the standalone page; some users bookmark it.)
3. **Transactions table.** `transactions.account_id` references `accounts.id`. After the migration, transactions are still per-account, but accounts are now per-membership — so transactions effectively become per-membership. No schema change needed, but worth noting in the spec.
4. **Currency per account vs per org.** Today `accounts.currency` defaults to USD. After the migration, should currency move to `orgs`? Out of scope here; flag for follow-up.

## Out of Scope

- "Delete user account entirely" (today's `DELETE /admin/users/{id}`) — moves to a future platform-admin page. For this refactor, only the membership can be removed; the underlying user row persists.
- Per-org currency.
- Re-designing Members.tsx visually — only the data and capabilities change; the visual layout matches the current style.
- Mobile UX changes beyond what falls out of the new columns.
- Renaming the `users` or `accounts` tables.
- Auditing or changing how `users.role` (the legacy `'admin'|'user'` column) is used elsewhere. That column is already legacy; it stays for now.

## Rollout

Single PR — no feature flag. The migration runs on app start. If the backfill encounters a NULL `org_id` after running, the app refuses to start (caught by the runtime guard above). Operators should run the migration in a staging environment first if they have multi-org users with non-trivial balances.

CHANGELOG entry under "Changed" and "Removed":
- **Changed**: `Accounts` are now per-membership (`(user_id, org_id)` unique) instead of per-user. Balance management moves from the deleted Users page to the Members page.
- **Removed**: `Users` page and the `/admin/users*` routes (replaced by `/admin/members*`).
