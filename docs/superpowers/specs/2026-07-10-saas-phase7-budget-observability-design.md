# SaaS Phase 7: Budget Observability — Design

**Targets release:** v2.4.0
**Built on top of:** Phase 6 (`budget_counters` table + budget enforcement, `2026-07-10-saas-phase6-budget-enforcement-design.md`) and Phase 5 (`default_budget_monthly_usd` org_settings kv + rate-limit enforcement, `2026-07-09-saas-phase5-org-defaults-and-rate-limit-enforcement-design.md`)
**Date:** 2026-07-10

## Problem

Phase 6 made monthly budgets enforce. A key with `budget_monthly = $50` now returns `429 budget_exceeded` once month-to-date spend crosses $50. But operators have no way to *see* how close they are to the limit:

- The OrgSettings page has a `default_budget_monthly_usd` input but no display of the current MTD total.
- The Keys page shows `budget_monthly` per key but not the accrued spend for the current month.
- A 429 surfaces in the proxy response — but the operator's first instinct ("how did we get here?") has no answer in the UI.

Phase 7 closes this gap. Same data source as Phase 6 (`budget_counters`), surfaced in the two places operators configure budgets.

## Goal

Make month-to-date (MTD) spend visible at both layers of the budget resolution order:

- **OrgSettings page** gains a "Budget status" subsection showing org-default budget vs. sum of all key MTDs, with a color-coded progress bar.
- **Keys table** gains an "MTD this month" column showing per-key spend with the same color coding.

Both surfaces are read-only. Enforcement behavior is unchanged from Phase 6.

## Non-Goals

- **Per-user budget views.** Members don't have direct budget authority; org-level + per-key is enough for v1.
- **Historical views** (last N months, day-by-day breakdown). Just current UTC month.
- **Pre-dispatch cost estimation.** Still deferred per Phase 6.
- **Hard org-level ceiling** (cross-key enforcement). Still deferred per Phase 6.
- **Budget alerts** (email/Slack when MTD crosses thresholds). Requires notification infra.
- **Auto-refresh / live polling.** Page-load only for v1. (The data is already a snapshot — auto-refresh would just churn the bar.)
- **Inline "approaching limit" warnings** (textual "you've used 85% of your budget"). Color coding carries enough signal for v1; warnings are easy to add later.
- **API for setting MTD.** MTD is derived data, computed from `usage_records` via `budget_counters`. Never user-settable.
- **Counter reconciliation job.** Still deferred per Phase 6.
- **MTD breakdown by model / channel / day.** Future dashboard work.

## Decisions Locked (from brainstorming)

| Decision | Choice | Alternatives rejected |
|---|---|---|
| Where the dashboard lives | **Both OrgSettings + Keys table** — surfaces MTD where budgets are configured | OrgSettings only (Keys page would have no per-key visibility); Keys only (org rollup has no home); New dedicated page (extra nav entry, duplicates existing pages) |
| OrgSettings detail level | **Org rollup only** — sum of MTDs across all keys + progress bar | Rollup + per-key breakdown table (duplicates Keys page); Rollup + inline warnings (deferred to future phase) |
| API shape | **Extend `/keys` + new `/budget-status`** | Extend `/keys` only + client-side sum (incomplete if keys are paginated); New single `/budgets` endpoint returning everything (over-fetching, less focused) |
| Visual richness | **Color-coded progress bars** — green/yellow/orange/red by % of budget | Plain bars (no at-a-glance signal); Color + inline warnings + auto-refresh (more polish than v1 needs) |
| Org rollup computation | **Read-time SUM over `budget_counters`** | Materialized `org_budget_counters` table (write amplification, drift risk; not worth it for typical org sizes) |
| Keys listing MTD | **LEFT JOIN in existing query** | Separate batch query (N+1 risk, pagination edge cases) |
| Currency formatting | **USD, 2 decimal places** (`$50.00`) | 4 decimals (over-precise); no symbol (ambiguous) |
| Month definition | **UTC calendar month** | Local time (inconsistent with Phase 6 enforcement); Rolling 30 days (unintuitive) |

## Architecture

### Data flow

```
OrgSettings page
  → useGetOrgDefaults(slug)       [Phase 5 hook] — returns default_budget_monthly_usd
  → useGetBudgetStatus(slug)      [NEW hook]     — returns { accrued_units, month_bucket }
  → composed: budget (from defaults) + accrued (from status) → render BudgetStatusCard

Keys page
  → useGetKeys(slug)              [existing hook, extended response] — returns [{ ..., budget_monthly, mtd_units }, ...]
  → per-row: budget_monthly + mtd_units → render MTD cell
```

Both pages fetch in parallel via React Query. No cross-page state shared.

### Component boundaries

| Component | Responsibility |
|---|---|
| `crates/storage/src/lib.rs` | New trait method `get_org_month_to_date_spend(org_id) -> Result<i64, DbErr>` |
| `crates/storage/src/postgres.rs` | SQL impl: `SELECT COALESCE(SUM(bc.accrued), 0) FROM budget_counters bc JOIN api_keys ak ON ak.id = bc.key_id WHERE ak.org_id = $1 AND bc.month_bucket = $2` |
| `crates/api/src/management.rs` | New `GET /{slug}/budget-status` handler. Extends existing `list_keys` handler with LEFT JOIN against `budget_counters`. |
| `web/src/api/orgs.ts` | `getBudgetStatus(slug)` client + `BudgetStatus` type |
| `web/src/hooks/useBudgetStatus.ts` | New `useGetBudgetStatus` hook (React Query) |
| `web/src/components/ui/ProgressBar.tsx` | New color-coded progress bar — small, focused, reusable |
| `web/src/lib/budgetColor.ts` | Pure helper `budgetBarColor(usedPct: number \| null): string` — shared by OrgSettings card + Keys column |
| `web/src/pages/OrgSettings.tsx` (existing) | New "Budget status" subsection below the Defaults inputs |
| `web/src/pages/Keys.tsx` (existing) | New MTD column in the existing keys table |
| `web/src/i18n/{en,zh}.json` | New labels: `orgSettings.budgetStatus.*`, `keys.columns.mtdThisMonth` |

### Why no org_budget_counters table

The org rollup is `SUM(accrued) GROUP BY org_id` over the existing `budget_counters` rows (one row per (key, month)). We already have the per-key data from Phase 6; aggregating at read time is one indexed query against ~keys-in-org rows.

Materializing at write time would require:
- New `org_budget_counters(org_id, month_bucket, accrued)` table
- A second write in the `record_usage` transaction (more write amplification)
- Drift risk between per-key and per-org counters (now two tables to keep consistent)
- A reconciliation job to catch the drift

Not worth it for v1. If an org grows to thousands of keys and the SUM becomes slow, revisit.

### Why LEFT JOIN in list_keys

Single SQL round-trip. No N+1 risk. No pagination edge cases (the join is against the per-key budget_counters row, fetched by PK). Cost: one index seek per key row, sub-ms for typical orgs.

## Data Model

**No schema changes.** Phase 7 reads Phase 6's existing tables:

- `budget_counters(key_id, month_bucket, accrued, updated_at)` — read by `get_org_month_to_date_spend` (SUM) and `list_keys` (LEFT JOIN)
- `api_keys(id, org_id, name, ..., budget_monthly, ...)` — read by `list_keys` (existing)
- `org_settings(org_id, key, value)` — read by the existing `get_org_setting` for `default_budget_monthly_usd`

**No migrations.** No new columns, no new tables, no backfill.

## API Surface

### New: `GET /api/v1/{slug}/budget-status`

```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "accrued_units": 5230000000,
  "month_bucket": "2026-07"
}
```

- **Auth:** org-admin gated (parity with `GET /{slug}/defaults` from Phase 5).
- **`accrued_units`:** `i64`, 10⁸ subunits per USD. Frontend converts to USD via the existing `unitsToUsd` helper — matches Phase 5/6 API boundary convention (subunits in transit, USD at the rendering boundary).
- **`month_bucket`:** `"YYYY-MM"` UTC calendar month (matches Phase 6 bucketing).
- **No `budget` field:** frontend composes the budget from `default_budget_monthly_usd` fetched via the existing `/defaults` endpoint (fetched in parallel). Keeps endpoints focused: one source of truth for each datum.

### Extended: `GET /api/v1/{slug}/keys`

Each key object in the response array gains one additive field:

```json
{
  "id": "key_abc",
  "name": "production",
  "enabled": true,
  "rate_limit": null,
  "budget_monthly": null,
  "mtd_units": 0,
  "created_at": "2026-07-01T..."
}
```

- **`mtd_units`:** `i64`, 10⁸ subunits per USD. `0` when no `budget_counters` row exists for this key/month (via SQL `COALESCE(bc.accrued, 0)`).
- **Backward-compatible:** additive field. Existing API consumers ignore it. No existing fields changed.
- **No filtering by month:** the response reflects the current UTC month only. Historical MTD is out of scope.

### No write endpoints

This phase is read-only. Budget values are still set via:
- Existing `PUT /api/v1/{slug}/defaults` (Phase 5) for org default
- Existing `POST`/`PATCH /api/v1/{slug}/keys` for per-key budgets

### Error responses

Standard 401 (unauthenticated), 403 (not a member), 404 (org not found). No new error variants. Phase 6's `BudgetExceeded` (429) only surfaces through the proxy path, not through these management endpoints.

## UI Design

### OrgSettings — new "Budget status" subsection

Placed immediately below the existing "Defaults" section.

```
┌────────────────────────────────────────────┐
│  Budget status                  2026-07     │  ← month_bucket shown small
├────────────────────────────────────────────┤
│                                            │
│  $52.30 used of $50.00                     │
│  ████████████████████░░░░░  104.6%         │  ← bar color: red (over 100%)
│  Over budget by $2.30                       │
│                                            │
└────────────────────────────────────────────┘
```

#### Variants by state

| State | Display | Bar color |
|---|---|---|
| Budget set, used <60% | `$X used of $Y` / bar / `Z%` | green (`bg-emerald-500`) |
| Budget set, used 60-80% | same shape | yellow (`bg-amber-500`) |
| Budget set, used 80-100% | same shape | orange (`bg-orange-500`) |
| Budget set, used >100% | same shape + `Over budget by $X` | red (`bg-red-500`) |
| No budget set (`default_budget_monthly_usd` is null) | `Unlimited — no monthly cap` (no bar) | none (`bg-muted`) |
| No spend this month (MTD = 0) | `$0.00 used of $Y` / empty bar | green |

### Keys table — new MTD column

Placed immediately after the existing "Budget" column.

```
| Name        | Prefix   | Budget | MTD this month        | ... |
|-------------|----------|--------|-----------------------|-----|
| production  | sk-abcd  | $50.00 | $52.30 ████ 105%      | ... |  ← red (over)
| staging     | sk-efgh  | $10.00 | $3.00   ▌    30%      | ... |  ← green
| infra       | sk-ijkl  |   —    | $0.00   ·     —       | ... |  ← gray (no budget)
| test        | sk-mnop  | $5.00  | $4.20   ███  84%      | ... |  ← orange
```

- MTD column shows: USD value + inline mini-bar + percentage.
- Keys with `budget_monthly IS NULL` show `—` for budget, `$X` for MTD, no percentage, gray.
- Color matches OrgSettings thresholds (shared helper).

### Color thresholds (shared helper)

```ts
// web/src/lib/budgetColor.ts
export function budgetBarColor(usedPct: number | null): string {
  if (usedPct === null) return 'bg-muted';       // no budget set
  if (usedPct > 100) return 'bg-red-500';
  if (usedPct >= 80) return 'bg-orange-500';
  if (usedPct >= 60) return 'bg-amber-500';
  return 'bg-emerald-500';
}

export function budgetUsedPct(accruedUnits: number, budgetUnits: number | null): number | null {
  if (budgetUnits === null || budgetUnits === 0) return null;
  return (accruedUnits / budgetUnits) * 100;
}
```

Tailwind classes per project convention (`web/src/lib/cn.ts` for class merging).

### i18n new keys

```json
"orgSettings": {
  "budgetStatus": {
    "title": "Budget status",
    "usedOf": "{{accrued}} used of {{limit}}",
    "unlimited": "Unlimited — no monthly cap",
    "overBudget": "Over budget by {{amount}}"
  }
},
"keys": {
  "columns": {
    "mtdThisMonth": "MTD this month"
  }
}
```

Mirror in `zh.json`:

```json
"orgSettings": {
  "budgetStatus": {
    "title": "预算状态",
    "usedOf": "已使用 {{accrued}} / {{limit}}",
    "unlimited": "无限制 — 无月度上限",
    "overBudget": "超出预算 {{amount}}"
  }
},
"keys": {
  "columns": {
    "mtdThisMonth": "本月消费"
  }
}
```

## Testing

### Storage unit (`crates/storage/src/postgres.rs`, sibling to Phase 6 tests)

| Test | Verifies |
|---|---|
| `get_org_mtd_returns_zero_for_unknown_org` | Empty result → 0 |
| `get_org_mtd_sums_across_keys` | 3 keys, each with $5 spend → $15 total |
| `get_org_mtd_excludes_other_months` | Spend from last month not counted (40-day backdated record) |
| `get_org_mtd_no_cross_org_leak` | Key in org A's spend not visible to org B |
| `list_keys_includes_mtd_units` | Each key in response has `mtd_units` field, values match seeded spend |
| `list_keys_mtd_zero_when_no_spend` | Fresh key returns `mtd_units: 0` |

Reuses Phase 6's `seed_org_with_budget_and_key` + `seed_usage_record` test helpers.

### API integration (`crates/api/tests/phase7_budget_status.rs` — new file)

| Test | Verifies |
|---|---|
| `get_budget_status_returns_accrued` | 200 with `{ accrued_units, month_bucket }`, correct values after seeding usage |
| `get_budget_status_zero_for_fresh_org` | 200 with `accrued_units: 0` |
| `get_budget_status_403_for_non_member` | Non-member of org → 403 |
| `get_budget_status_401_unauthenticated` | No bearer → 401 |
| `list_keys_includes_mtd_field` | Response payload has `mtd_units` per key (extending existing keys list test) |

### Frontend unit

- `web/src/lib/budgetColor.test.ts`: thresholds (0%, 30%, 60%, 80%, 100%, 105%, null budget).
- `web/src/components/ui/ProgressBar.test.tsx`: renders with given %, applies correct color class.
- `web/src/pages/OrgSettings.test.tsx` (extend if exists): renders Budget status section correctly in each state (under/over/unlimited/no-spend).
- `web/src/pages/Keys.test.tsx` (extend if exists): MTD column renders correctly with/without budget.

### E2E (`web/e2e/budget-status.spec.ts` — new file)

- Login as admin → set org default budget → fire one request (allowed) → navigate to OrgSettings → assert Budget status section renders with non-zero accrued → navigate to Keys → assert MTD column non-zero
- Same graceful-degradation pattern as Phase 6 e2e (logs skip-notice if upstream unreachable and no cost recorded)

### No new error variants to test

The read endpoints return standard 401/403/404 — covered by existing middleware tests.

### Test isolation

Each test uses unique slug via `RUN_TAG` (matches Phase 5/6 pattern). No cross-test collision.

## CHANGELOG Entry

Under `## [Unreleased] → Added`, after the Phase 6 block:

```markdown
- **Phase 7 (budget observability):**
  - **New UI:** OrgSettings gets a "Budget status" subsection showing org MTD total (sum across all keys) against the org-default budget, with a color-coded progress bar (green <60%, yellow 60-80%, orange 80-100%, red >100%). The Keys table gets an "MTD this month" column showing per-key spend with the same color coding.
  - **New endpoint:** `GET /api/v1/{slug}/budget-status` returns `{ accrued_units, month_bucket }` (i64 subunits, UTC calendar month). Org-admin gated.
  - **Extended endpoint:** `GET /api/v1/{slug}/keys` now includes `mtd_units: i64` per key. Additive, non-breaking — existing API consumers ignore the new field.
  - **New storage method:** `Storage::get_org_month_to_date_spend(org_id)`. Reads the existing `budget_counters` table (from Phase 6) — no schema changes.
  - **No behavior change:** enforcement remains as shipped in Phase 6 (post-completion, fail-open on storage errors). This phase is purely read-side observability.
```

**No upgrade note** — this phase is additive and read-only. No existing API contract changes; no DB migration; no enforcement behavior change. Deploy-safe.

## Out of Scope / Future Work

1. **Per-user budget views.** Members don't have direct budget authority; org-level + per-key is enough for v1.
2. **Historical views** (last N months, day-by-day breakdown). Genuine dashboard work; separate phase.
3. **Inline "approaching limit" warnings** (textual "you've used 85% of your budget"). Easy add once we see if color coding is sufficient.
4. **Auto-refresh / live polling.** v1 is page-load only. If staleness becomes a real pain, add a 30s polling hook.
5. **Pre-dispatch cost estimation.** Still deferred per Phase 6.
6. **Hard org-level ceiling.** Still deferred per Phase 6.
7. **Budget alerts** (email/Slack). Requires notification infra.
8. **Counter reconciliation job.** Still deferred per Phase 6.
9. **MTD breakdown by model / channel / day.** Future dashboard work.
10. **Materialized `org_budget_counters` table.** Only worth it if an org grows large enough that the SUM becomes slow. Measure first.
