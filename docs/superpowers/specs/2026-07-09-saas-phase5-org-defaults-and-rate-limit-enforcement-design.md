# SaaS Phase 5: Org Defaults + Rate-Limit Enforcement — Design

**Targets release:** v2.2.0
**Built on top of:** Phase 1 (`org_settings` table, `2026-07-07-saas-multi-tenant-orgs-design.md`) and Phase 4 (`crates/email`, `2026-07-09-saas-phase4-email-and-email-bound-invitations-design.md`)
**Date:** 2026-07-09

## Problem

Two intertwined gaps:

1. **Org-wide rate-limit / budget defaults were deferred** to "Phase 4+" by the parent SaaS design (`2026-07-07-saas-multi-tenant-orgs-design.md:29, 1013`). The `org_settings(org_id, key, value)` table has existed since Phase 1, but no settings keys are consumed by business logic. Different orgs cannot configure their own quotas.

2. **Per-key rate limits are stored but never enforced.** `api_keys.rate_limit` is a populated column. `RateLimiter::check_and_increment` is constructed in `AppState`. But `check_and_increment` has zero production call sites (grep across `crates/api/src/`, `crates/gateway/src/` — only test callers in `crates/ratelimit/src/lib.rs:70-95`). The 429 handling at `proxy.rs:1478` is for *upstream* provider 429s, not our own enforcement. Same for `budget_monthly` on `api_keys` — stored, never checked.

Phase 5 closes both gaps with a single coherent theme: **make org-level rate-limit defaults real, and make rate limits actually enforce**.

## Goal

- Admins can set org-wide defaults for rate limit (RPM) and monthly budget (USD) via API and UI.
- API keys whose `rate_limit` is `null` inherit the org default.
- Per-key rate limits AND org-default rate limits are enforced at request time, returning 429 with a `Retry-After` header when exceeded.
- Org-level budget defaults are stored and surfaced in UI but **not enforced** in this phase (parity with existing per-key budget).

## Non-Goals

- **Budget enforcement** (per-key OR per-org). Stored only. Future phase.
- **Per-model rate limits** (`key_model_rate_limits` table). Same gap pattern; separate phase.
- **Hard org-level ceilings** (sum usage across keys, reject when org total exceeded). Out of scope — Phase 5 uses "default only" semantics.
- **Cross-org billing aggregation.** Not a Phase 5 item.
- **Soft delete + janitor.** Separate future phase.
- **SSO / SCIM / SAML.** Separate spec per parent design.

## Decisions Locked (from brainstorming)

| Decision | Choice | Alternatives rejected |
|---|---|---|
| Theme | Org-wide defaults + enforcement | Soft-delete janitor; email follow-ups; combined mega-phase |
| Rate-limit semantics | "Default only" (no ceiling/aggregation) | Hard org ceiling; default + ceiling combo |
| Budget scope | Stored only (no enforcement) | Default + real enforcement; drop budget entirely |
| Architecture | Typed `defaults` endpoints, generic kv storage | Generic kv pass-through; dedicated `org_defaults` table |
| Org-default fetch strategy | Per-request storage read | TTL cache (correctness risk on policy change) |
| Rate-limit bucketing | Per-key (collapse model dimension via `""`) | Per-(key, model); per-key-only via limiter refactor |
| Audit logging | Deferred to future work (no management-action audit surface exists today) | Bolt on a one-off `org_defaults.update` audit row |
| Counting semantics | Count request on dispatch, regardless of upstream outcome | Count only successful upstream; count after upstream |

## Architecture

### Resolution order

At request time, after auth resolves the `ApiKey` and `org_id`:

```
effective_rpm = api_key.rate_limit ?? org.default_rate_limit_rpm ?? None
```

`None` → unlimited (no rate-limiter call). `Some(n)` → call `RateLimiter::check_and_increment(api_key.id, "", Some(n), None, None)` (empty model string = per-key bucket). On `false`, return 429.

### Component boundaries

| Component | Responsibility |
|---|---|
| `crates/storage/src/types.rs` | New `OrgDefaults { default_rate_limit_rpm: Option<i64>, default_budget_monthly_usd: Option<i64> }` (cents) |
| `crates/storage/src/lib.rs` | New trait methods: `get_org_defaults(org_id)`, `set_org_defaults(org_id, defaults)` |
| `crates/storage/src/postgres.rs` | Wraps existing `get_org_setting`/`set_org_setting` kv calls |
| `crates/api/src/management/orgs.rs` | `GET`/`PUT /api/v1/orgs/{id}/defaults` handlers; USD↔cents conversion at API boundary |
| `crates/api/src/management/mod.rs` | Mounts the two new routes |
| `crates/api/src/proxy.rs` | New "rate-limit check" step between auth and balance check |
| `crates/api/src/auth.rs` | Reuses existing `can_manage_org_settings` for write permission |
| `crates/api/src/error.rs` | Extends `ApiError::RateLimited` to emit `Retry-After` header |
| `web/src/api/orgs.ts` | `getOrgDefaults`, `updateOrgDefaults` |
| `web/src/pages/OrgSettings.tsx` | New "Defaults" section between General and Danger Zone |
| `web/src/i18n/{en,zh}.json` | `orgSettings.defaults.*` keys |

## Data Model

**No schema migration.** All storage fits in the existing `org_settings` table:

| Key | Value format | Notes |
|---|---|---|
| `default_rate_limit_rpm` | decimal integer as text, e.g. `"100"` | Absent → unlimited |
| `default_budget_monthly_usd` | integer cents as text, e.g. `"5000"` for $50.00 | Absent → no budget; follows the project's monetary-integer-subunits convention |

Existing orgs have neither key → both default to `None` (unlimited / no budget). No data backfill needed.

## API Surface

### `GET /api/v1/orgs/{org_id}/defaults`

**Permission:** org membership (member+). Members can read defaults (parity with "General" section visibility).

**200 OK:**
```json
{
  "default_rate_limit_rpm": 100,
  "default_budget_monthly_usd": 50.00
}
```

Fields are `number | null`. `null` = "not set" (unlimited RPM / no budget).

### `PUT /api/v1/orgs/{org_id}/defaults`

**Permission:** `can_manage_org_settings` (admin+ and platform_admin). Matches rename/slug edit permission in existing General section.

**Request body** — both fields required (caller echoes unchanged values):
```json
{
  "default_rate_limit_rpm": 100,
  "default_budget_monthly_usd": 50.00
}
```

`null` clears that key. Validation:
- `default_rate_limit_rpm`, if `Some(n)`: must be `n >= 1`, else 400
- `default_budget_monthly_usd`, if `Some(n)`: must be `n >= 0`, else 400

**200 OK** returns the updated object (same shape as GET).

**Audit:** _Deferred to future work._ Building a management-action audit surface is its own concern — no existing handler in `crates/api/src/management/` writes audit rows today (only proxy traffic does, via NATS). A future phase should add a uniform management-action audit API and backfill it across all existing handlers, not bolt one on here for `org_defaults.update` alone.

### Error responses

| Status | When |
|---|---|
| 400 | Validation failure (RPM < 1, budget < 0, non-integer, malformed JSON) |
| 403 | Caller lacks required role (PUT only) |
| 404 | Org does not exist or caller is not a member |

## Proxy Enforcement

### Insertion point

In `crates/api/src/proxy.rs`, the existing per-request handler has these steps in order:

1. Auth (resolve api_key, user, org_id)
2. Balance check
3. Channel selection
4. Upstream proxy

**New step — Rate-limit check — inserts between (1) and (2).** Earlier than balance check so a throttled caller doesn't even consume a DB read for balance. After auth because we need the resolved `api_key.id` and `org_id`.

### Org-default fetch strategy

When `api_key.rate_limit.is_none()`, fetch `org.default_rate_limit_rpm` via a direct storage call (one extra `SELECT` on `org_settings` for that key — 2 rows indexed on `(org_id, key)`).

**Per-request, no TTL cache.** Rationale: policy changes (admin tightens a rate limit) must take effect immediately for operational safety. A 60s TTL would race the policy change. The cost is one indexed DB read per unmatched-key request — acceptable given the request path is already DB-heavy (api_key + user + account loads).

When `api_key.rate_limit.is_some()`, skip the org-default fetch entirely — the per-key value wins.

### Bucketing semantic

`check_and_increment` takes `(key_id, model, rpm_limit, tpm_limit, input_tokens)` — `key_id` and `model` are both bucketing keys. For Phase 5 we pass `model = ""` so the bucket collapses to **per-key RPM** (one counter per `api_key.id`, regardless of which model the client requested). This matches what `api_keys.rate_limit` intuitively means ("this key can do N RPM total", not "N RPM × number of distinct models").

The `client_requested_model` string parsed at `proxy_inner:949` is NOT used as a bucketing key — that would let a caller escape the limit by varying model strings. Per-model limits are a separate concern (see `key_model_rate_limits` in Non-Goals).

### Enforcement code (pseudo-code)

```rust
// api_key.rate_limit is Some(n) → use it. Otherwise fall back to org default.
let effective_rpm = match api_key.rate_limit {
    Some(n) => Some(n),
    None => storage.get_org_setting(&api_key.org_id, "default_rate_limit_rpm")
        .await
        .ok().flatten()
        .and_then(|s| s.parse::<i64>().ok()),
};

if let Some(rpm) = effective_rpm {
    let allowed = state.rate_limiter
        .check_and_increment(&api_key.id, "", Some(rpm), None, None)
        .await;
    if !allowed {
        return Err(ApiError::RateLimited {                  // 429 + Retry-After
            retry_after_secs: state.system_info.rate_limit_window_secs,
        });
    }
}
```

### 429 response

`ApiError::RateLimited` currently maps to `(StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded", None)`. Extend to emit a `Retry-After: <rate_limit_window_secs>` header on the response — equals the configured window size (`state.rate_limit_window_secs`). Caller can retry after the sliding window resets.

Response body unchanged.

### Counting semantics

`check_and_increment` runs **before** any upstream work. The counter increments on dispatch, regardless of whether upstream succeeds, fails, or streams partially. Rationale: "you made N requests this minute" is the natural semantic — caller consumed capacity by asking. This is not a regression vs today (today has no counting at all) but is a new behavior that callers will observe.

### Fail-open

If `check_and_increment` itself errors (storage failure inside the limiter, poisoned mutex, etc.), allow the request. Matches the project's general fail-open posture for non-correctness-critical policy checks.

## Frontend

### `web/src/pages/OrgSettings.tsx`

Insert a new **Defaults** section between "General" (rename/slug) and "Danger zone" (delete). Same layout pattern as General: admins get editable inputs + Save; members see disabled inputs (read-only).

**Form:**
```
Defaults
┌──────────────────────────────────────────────────────┐
│ Default rate limit (RPM)                             │
│ [ input: number | placeholder: "Unlimited" ]        │
│ Applies to API keys without their own limit.         │
│                                                      │
│ Default monthly budget (USD)                         │
│ [ input: number | placeholder: "No budget" ]        │
│ Stored for display. Not currently enforced.          │
│                                                      │
│                              [ Cancel ] [ Save ]    │
└──────────────────────────────────────────────────────┘
```

**Behavior:**
- On mount, fetch via `useGetOrgDefaults` (React Query). Empty input if value is `null`.
- Save calls `updateOrgDefaults` with both fields (PUT semantics).
- Success toast: `"Defaults saved."` / `"默认值已保存。"`
- Error toast via `getErrorMessage`.
- Save button disabled while submitting or when inputs equal loaded values (no-op guard).

The "Not currently enforced" help text on the budget input is **required** — admins must not believe they've set a hard ceiling.

### `web/src/api/orgs.ts`

```ts
export type OrgDefaults = {
  default_rate_limit_rpm: number | null;
  default_budget_monthly_usd: number | null;
};

export async function getOrgDefaults(orgId: string): Promise<OrgDefaults>;
export async function updateOrgDefaults(
  orgId: string,
  defaults: OrgDefaults,
): Promise<OrgDefaults>;
```

### i18n

New keys under `orgSettings.defaults.*` added to both `en.json` and `zh.json`:
`title`, `description`, `rateLimitLabel`, `rateLimitHelp`, `budgetLabel`, `budgetHelp`, `save`, `cancel`, `saveSuccess`, `saveError`.

## Testing

### Storage unit (`crates/storage/src/postgres.rs`)

- Read-write-read round-trip on `OrgDefaults`
- `None` preserved on both fields
- No interference between two keys on the same org
- No interference between two orgs

### API integration (`crates/api/tests/phase5_org_defaults.rs` — new file)

- `GET` on org with no defaults → both fields `null`
- `PUT` sets both → `GET` reflects
- `PUT` with `null` clears that field
- `PUT` validation: RPM < 1 → 400; budget < 0 → 400; non-integer → 400
- `PUT` as non-admin member → 403
- `GET`/`PUT` as non-member → 404

### Proxy integration (`crates/api/tests/phase5_enforcement.rs` — new file)

- Org default = 5; key has no per-key limit; send 6 requests as the key → 6th returns 429 with `Retry-After: <window>`
- Org has no default; key has `rate_limit = 10`; send 11 requests → 11th returns 429 (proves per-key path also wired)
- Org has no default; key has no per-key limit; send 20 requests → no 429 (proves unlimited path)
- Org default = 5; key has `rate_limit = 10`; send 6 requests → all succeed (per-key wins, org default overridden)

### Frontend unit (`web/src/pages/OrgSettings.test.tsx`)

Existing test file already covers rename/slug; extend with:
- Admin render → inputs editable, Save present
- Member render → inputs disabled, no Save button
- Load failure → error state rendered, inputs not shown
- Save success → toast called, query invalidated
- Save with 4xx → error toast, inputs preserved

### E2E (`web/e2e/org-defaults.spec.ts` — new file)

- Admin signs in → OrgSettings page
- Sets default rate limit to 3
- Switches to a key in the org, fires 4 proxy requests with that key
- Asserts 4th response is 429 with `Retry-After`

## CHANGELOG Entry

Under `## [Unreleased] → Added`:

> **Phase 5 (per-org defaults + rate-limit enforcement):**
> - New: `GET`/`PUT /api/v1/orgs/{id}/defaults` for org-wide rate-limit RPM and monthly budget defaults. UI lives in Org Settings → Defaults.
> - **Behavior change:** per-key rate limits (`api_keys.rate_limit`) are now enforced at request time via the existing in-memory rate limiter — previously stored but never checked. Resolution order: `key.rate_limit ?? org.default_rate_limit_rpm ?? unlimited`. Exceeding returns `429` with `Retry-After` set to the rate-limit window size.
> - Org-level `default_budget_monthly_usd` is stored but **not enforced** in this phase (parity with existing per-key budget — both will be enforced in a future phase).
>
> **Upgrade note:** any existing `api_keys` rows with non-null `rate_limit` will start receiving 429s on requests beyond their limit. Audit existing keys before upgrading if any have low values set.

## Out of Scope / Future Work

1. **Budget enforcement** (per-key and per-org) — separate phase. Requires usage rollup, decision point in proxy, accounting integration.
2. **Per-model rate limits** (`key_model_rate_limits` table) — same gap pattern; separate phase.
3. **Hard org-level ceilings** (sum across keys) — explicitly out of scope per "default only" decision.
4. **Rate-limit response body enrichment** — current body is plain `"Rate limit exceeded"`. Could carry `retry_after`, `limit`, `remaining` for parity with industry conventions. Future enhancement.
5. **Redis-backed distributed rate limiter** — current `RateLimiter` is in-memory per-node. In a multi-node deployment, two nodes would each see half the traffic and apply the limit independently (effectively doubling throughput). Phase 5 ships as-is; multi-node correctness is a future infra item.
6. **Management-action audit logging** — no handler in `crates/api/src/management/` currently writes audit rows; only proxy traffic is audited (via NATS). A future phase should add a uniform management-action audit API, backfill it across all existing handlers (`update_org`, `update_key`, `update_member_roles`, etc.), AND surface `org_defaults.update`. Bolt-on audit for just `org_defaults.update` would be inconsistent.
7. **Audit log UI** for any future management-action events — depends on (6).
