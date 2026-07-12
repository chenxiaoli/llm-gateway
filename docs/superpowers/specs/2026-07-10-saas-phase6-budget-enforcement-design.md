# SaaS Phase 6: Budget Enforcement — Design

**Targets release:** v2.3.0
**Built on top of:** Phase 5 (`org_settings` kv defaults + rate-limit enforcement, `2026-07-09-saas-phase5-org-defaults-and-rate-limit-enforcement-design.md`) and Phase 1 (`org_settings` table, `2026-07-07-saas-multi-tenant-orgs-design.md`)
**Date:** 2026-07-10

## Problem

Phase 5 closed the "stored but never enforced" gap for **rate limits**. The same gap exists for **budgets**:

- `api_keys.budget_monthly` is a `BIGINT` column (10⁸ subunits per USD, migrated in `20260502000000_monetary_integer_subunits.sql`). Populated, surfaced in UI, never checked.
- `default_budget_monthly_usd` (Phase 5 org_settings kv) is likewise populated and surfaced in the OrgSettings UI — with a `budgetHelp` text that explicitly says "Not currently enforced."

A key with `budget_monthly = $50` will run forever. Phase 6 closes that gap.

## Goal

Make budgets enforce, mirroring Phase 5's shape:

- Per-key budgets AND org-default budgets enforce at request time.
- Resolution order: `api_key.budget_monthly ?? org.default_budget_monthly_usd ?? unlimited`.
- Exceeding returns 429 with a structured body explaining the limit.
- OrgSettings UI text updated to reflect enforcement (no other UI changes).

## Non-Goals

- **Hard org-level ceiling** (sum spend across all keys). Phase 6 uses default-only semantics, matching Phase 5.
- **Pre-dispatch estimation** (reject based on input cost estimate). Output-token cost dominates; rejected for accuracy reasons.
- **MTD-vs-budget dashboard** (visual progress bar, approaching-limit warnings). Genuine UX work; separate phase.
- **Per-key MTD display** in the keys table. Same — UX-driven, separate phase.
- **Budget alerts** (email/Slack when MTD crosses thresholds). Requires notification infra.
- **Counter reconciliation job** (periodic recomputation to catch drift). Deferred — under normal operation, transactional write prevents drift.
- **Budget reset notifications**. Separate concern.
- **Month-rollover time-mock testing**. Hard to test without mocking time; deferred.

## Decisions Locked (from brainstorming)

| Decision | Choice | Alternatives rejected |
|---|---|---|
| Counting semantic | **Post-completion** — let current request finish, block future ones once MTD exceeds budget | Pre-dispatch (input estimate, ignores output cost); Hybrid (two paths, complex) |
| Org-level scope | **Default only** — per-key + inherited org default; no cross-key ceiling | Default + hard org ceiling (new key, ~30% more scope); Per-key only (leaves Phase 5 budget UI misleading) |
| MTD computation | **Materialized counter table** — `budget_counters(key_id, month_bucket, accrued)` updated on each usage_records insert | Per-request SUM (degrades at 100K+ rows/key/month); TTL cache (rejected for rate limits — same staleness risk) |
| Month definition | **UTC calendar month** — `YYYY-MM` bucket via `to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM')` | Rolling 30 days (unintuitive for budgets); Key anniversary (weird for org defaults) |
| Counter write path | **App-level in `record_usage`, same transaction as the usage_records INSERT** | DB trigger (harder to test/evolve); Background reconciliation only (doesn't enforce write-side) |
| HTTP status | **429** with `error.type = "budget_exceeded"`, no `Retry-After` | 402 Payment Required (semantically correct but rarely used); 403 (generic) |
| UI surface | **Minimal** — update `budgetHelp` text ("Now enforced."); no MTD-vs-budget dashboard | Full MTD dashboard (separate phase) |
| Reconciliation job | **Deferred** — counter drift possible if usage_records ever backfills out-of-band | Bundled (scope creep) |

## Architecture

### Resolution order

At request time, after Phase 5's rate-limit check resolves (Step 1.5), and before the balance check (Step 2):

```
effective_budget = api_key.budget_monthly ?? org.default_budget_monthly_usd ?? None
```

`None` → unlimited (no budget check). `Some(n)` → read `budget_counters.accrued` for the current UTC month; if `accrued > n`, reject with `ApiError::BudgetExceeded`.

### Post-completion semantic

The check uses MTD that **excludes the current request's cost**. The current request's `record_usage` happens after the response completes (via the existing audit worker pipeline). So:

- Request N arrives. MTD = $49, budget = $50. Check passes (`49 <= 50`). Request dispatched. After response, $3 recorded. MTD now $52.
- Request N+1 arrives. MTD = $52, budget = $50. Check fails (`52 > 50`). 429 returned.

**Leak:** at most one request can push MTD over budget. The request that crosses the line is allowed; the next one is rejected. This is the industry-standard pattern (Stripe, OpenAI).

### Component boundaries

| Component | Responsibility |
|---|---|
| `crates/storage/migrations/postgres/20260802000000_budget_counters.sql` | New `budget_counters` table (timestamp chosen to follow the latest existing migration `20260801000000_members_last_seen.sql`) |
| `crates/storage/src/types.rs` | (No new type — counter values are bare `i64`) |
| `crates/storage/src/lib.rs` | New trait method `get_month_to_date_spend(key_id) -> i64` |
| `crates/storage/src/postgres.rs` | Wraps the new query; modifies `record_usage` to update counter in same tx |
| `crates/api/src/proxy.rs` | New "Step 1.6: Budget check" between rate-limit (1.5) and balance (2) |
| `crates/api/src/error.rs` | New `BudgetExceeded` variant with body rendering |
| `web/src/i18n/{en,zh}.json` | Update `orgSettings.defaults.budgetHelp` text |

## Data Model

### New table

```sql
CREATE TABLE IF NOT EXISTS budget_counters (
    key_id       TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    month_bucket TEXT NOT NULL,  -- 'YYYY-MM' UTC
    accrued      BIGINT NOT NULL DEFAULT 0,  -- 10^8 subunits per USD (project convention)
    updated_at   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (key_id, month_bucket)
);

CREATE INDEX idx_budget_counters_month ON budget_counters(month_bucket);
```

**Design notes:**

- **PK `(key_id, month_bucket)`** serves the per-key lookup at request time.
- **`idx_budget_counters_month`** supports future queries like "all counters for 2026-07" (operational/audit).
- **No `org_id` column** — default-only semantics means no cross-key aggregation. (A future phase adding org ceilings can introduce `org_budget_counters`.)
- **`ON DELETE CASCADE`** — when an api_key is deleted, its counters go too. Matches the existing `usage_records.key_id` FK behavior.
- **No data backfill** — counter starts empty. New requests populate it. Historical spend (pre-Phase-6) is not retroactively counted; this is operationally acceptable since enforcement is a new behavior.

### Existing tables — no changes

- `api_keys.budget_monthly`: already BIGINT (10⁸ subunits). No type change.
- `usage_records.cost`: already BIGINT (10⁸ subunits). Same.
- `org_settings(org_id, key, value)`: already used by Phase 5 for `default_budget_monthly_usd`. No schema change.

## API Surface

### No new HTTP routes

Default-only semantics means the existing `GET`/`PUT /api/v1/orgs/{slug}/defaults` (Phase 5) is the only management surface for org-level budgets. Per-key budgets continue to flow through the existing `POST`/`PATCH /api/v1/{slug}/keys` endpoints.

### New `ApiError::BudgetExceeded`

In `crates/api/src/error.rs`, new variant:

```rust
pub enum ApiError {
    // ... existing ...
    RateLimited { retry_after_secs: i64 },
    BudgetExceeded {
        key_id: String,
        month_bucket: String,
        limit_units: i64,    // 10^8 subunits per USD
        accrued_units: i64,  // 10^8 subunits per USD
    },
}
```

### Response shape

Subunits → USD float at the API boundary via the existing `units_to_usd` helper in `crates/storage/src/money.rs`:

```json
HTTP/1.1 429 Too Many Requests
Content-Type: application/json

{
  "error": {
    "type": "budget_exceeded",
    "message": "Monthly budget exceeded. Spend: $52.30 / Limit: $50.00. Month: 2026-07.",
    "key_id": "key_abc123",
    "month_bucket": "2026-07",
    "limit": 50.0,
    "accrued": 52.3
  }
}
```

**No `Retry-After` header.** Budget exceeded is not a wait scenario — the caller must either wait until next calendar month (UTC) or have an admin raise the budget.

### Error status choice

**429** (not 402) for consistency with Phase 5's rate-limit response. Callers handling gateway errors branch on one status, with `error.type` distinguishing the cause. 402 "Payment Required" is conventionally reserved for payment flows (Stripe, etc.) and rarely used by API gateways. OpenAI/Anthropic both use 429 for budget-exceeded.

## Proxy Enforcement

### Insertion point

In `crates/api/src/proxy.rs`, Phase 5 added "Step 1.5: Rate-limit check" between auth and balance check. Phase 6 inserts **"Step 1.6: Budget check"** between Step 1.5 and Step 2:

1. Auth (resolve api_key, user, org_id)
2. Balance check ← *unchanged*
3. ~~Channel selection~~ (actual existing order has these after balance)
4. Upstream proxy

(Existing pre-Phase-5 ordering preserved; the new step slots in alongside Phase 5's.)

**Why after rate-limit, not before:** rate-limit check is in-memory (essentially free); budget check is a DB read. Throttled callers shouldn't consume a DB read.

### Enforcement code (pseudo-code)

```rust
// === Step 1.6: Budget check ===
// Post-completion: this check uses MTD that EXCLUDES the current request.
// Resolution order mirrors rate limits:
//   effective_budget = api_key.budget_monthly ?? org.default_budget_monthly_usd ?? None
let effective_budget = match api_key.budget_monthly {
    Some(units) => Some(units),
    None => match state.storage
        .get_org_setting(&api_key.org_id, "default_budget_monthly_usd")
        .await
    {
        Ok(Some(raw)) => raw.parse::<i64>().ok(),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(error = %e, org_id = %api_key.org_id,
                "org default budget lookup failed; failing open");
            None
        }
    },
};

if let Some(budget) = effective_budget {
    // Fail-open on counter read error (matches rate-limit posture).
    let accrued = state.storage
        .get_month_to_date_spend(&api_key.id)
        .await
        .unwrap_or(0);
    if accrued > budget {
        let month_bucket = format!("{}", Utc::now().format("%Y-%m"));
        return Err(ApiError::BudgetExceeded {
            key_id: api_key.id.clone(),
            month_bucket,
            limit_units: budget,
            accrued_units: accrued,
        });
    }
}
```

### Fail-open

If `get_month_to_date_spend` errors (storage failure inside the lookup), allow the request. Matches the project's fail-open posture for non-correctness-critical policy checks (parity with Phase 5's rate-limit org-default lookup).

### Atomic counter update

`crates/storage/src/postgres.rs::record_usage` is modified to wrap both writes in a single transaction:

```rust
async fn record_usage(&self, org_id: &str, usage: &UsageRecord) -> Result<(), DbErr> {
    let mut tx = self.pool.begin().await?;

    // Existing insert, unchanged.
    sqlx::query("INSERT INTO usage_records (...) VALUES ($1, ...)")
        .bind(...)
        .execute(&mut *tx).await?;

    // New: atomic counter upsert. Month bucket derived from created_at (UTC).
    let month_bucket = format!("{}", usage.created_at.format("%Y-%m"));
    sqlx::query(
        "INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (key_id, month_bucket)
         DO UPDATE SET accrued = budget_counters.accrued + EXCLUDED.accrued,
                       updated_at = NOW()"
    )
    .bind(&usage.key_id)
    .bind(&month_bucket)
    .bind(usage.cost)
    .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}
```

**Atomicity guarantee:** both writes succeed or both roll back. No drift under normal operation.

**Race-condition safety:** `ON CONFLICT DO UPDATE` is the Postgres-standard pattern for atomic increment. Concurrent `record_usage` calls serialize on the PK conflict; no lost updates.

**Write amplification:** one extra `INSERT ... ON CONFLICT DO UPDATE` per request. The PK index makes the conflict check O(log n). Negligible vs the existing 17-column insert.

## Frontend

### i18n text update

The only frontend change. In `web/src/i18n/en.json` and `web/src/i18n/zh.json`, update `orgSettings.defaults.budgetHelp`:

**Before:**
```json
"budgetHelp": "Stored for display. Not currently enforced. Empty = no budget."
```

**After (en):**
```json
"budgetHelp": "Enforced per calendar month (UTC). Empty = no budget."
```

**After (zh):**
```json
"budgetHelp": "按公历月（UTC）强制执行。留空 = 无预算。"
```

### No other UI changes

- No MTD-vs-budget dashboard (separate phase).
- No per-key MTD in the keys table (separate phase).
- No new toasts or alerts when budget is hit (the 429 surfaces through the existing error-handling path).
- No changes to the keys page for `api_keys.budget_monthly` — that input has no "not enforced" copy today and needs no change. Enforcement is silent from the UI's perspective.

## Testing

### Storage unit (`crates/storage/src/postgres.rs`)

- `record_usage` updates `budget_counters` correctly (round-trip: insert N records, verify counter == sum of costs).
- Month bucketing: records with different `created_at` months go to different buckets.
- Atomicity: simulated failure between the two writes rolls back both (e.g., make the second statement fail via type mismatch in a test scenario).
- `ON DELETE CASCADE`: deleting an api_key removes its counter rows.
- `get_month_to_date_spend` returns 0 for unknown key (no counter row).
- `get_month_to_date_spend` returns correct value for key with single row.
- Concurrent inserts: spawn N parallel `record_usage` calls for the same key + month; final accrued == N × per-call cost (no lost updates).

### API integration (`crates/api/tests/phase6_budget_error.rs` — new file)

- Construct `ApiError::BudgetExceeded` directly, render to response, assert 429 status, body has all 5 fields with correct values, no `Retry-After` header, USD floats not subunits.

### Proxy integration (`crates/api/tests/phase6_enforcement.rs` — new file)

- **Per-key enforces:** seed key with `budget_monthly = $5`. Insert a prior usage_record via `record_usage` so MTD = $3. Send request → allowed. Manually bump MTD to $6 (insert another record). Send request → 429 with `budget_exceeded` body.
- **Org-default enforces:** seed org with `default_budget_monthly_usd = $10`, key has no budget. Same flow as above using org default.
- **Unlimited path:** no per-key, no org default. Send 20 requests → no 429.
- **Per-key overrides org default:** key `budget_monthly = $5`, org `default_budget_monthly_usd = $10`. Per-key wins (5 < 10, so MTD=$6 rejects).
- **Fail-open:** mock `get_month_to_date_spend` to error → request allowed.
- **Body field verification:** on rejection, assert all 5 body fields (`type`, `message`, `key_id`, `month_bucket`, `limit`, `accrued`) are present with correct values.

### E2E (`web/e2e/budget-enforcement.spec.ts` — new file)

- Login as admin → set org `default_budget_monthly_usd = $0.01` (tiny) → fire one request (allowed, MTD was 0) → fire another (rejected, MTD now > $0.01) → assert 429 with `budget_exceeded` body → cleanup (restore defaults to null).

### No frontend unit tests needed

The only frontend change is a text string update. Not worth a snapshot test.

## CHANGELOG Entry

Under `## [Unreleased] → Added`:

> - **Phase 6 (budget enforcement):**
>   - **Behavior change:** per-key monthly budgets (`api_keys.budget_monthly`) and org-default budgets (`default_budget_monthly_usd` from Phase 5) are now **enforced**. Resolution order: `key.budget_monthly ?? org.default_budget_monthly_usd ?? unlimited`. Exceeding returns `429` with `error.type = "budget_exceeded"` and body `{ key_id, month_bucket, limit, accrued }` (USD floats). No `Retry-After` — caller must wait until next month or have budget raised.
>   - New `budget_counters` table materializes month-to-date spend per key (UTC calendar month), updated atomically with each `usage_records` insert via app-level transaction in `record_usage`.
>   - Counting semantic is **post-completion**: the check uses MTD that excludes the current request's cost. The request that pushes MTD over budget is allowed; the next request is rejected. Industry-standard leak (matches Stripe, OpenAI).
>   - OrgSettings `budgetHelp` text updated — the previous "Not currently enforced" disclaimer is removed.
>   - **Upgrade note:** any existing `api_keys` rows with non-null `budget_monthly`, or orgs with `default_budget_monthly_usd` set, will start receiving 429s on requests once their month-to-date spend exceeds the budget. Audit existing values before deploying.

## Out of Scope / Future Work

1. **Hard org-level budget ceiling** (sum spend across all keys, reject when org total exceeded). Phase 6 uses default-only semantics, matching Phase 5. A future phase would add `ceiling_budget_monthly_usd` as a separate key and a per-org counter table.
2. **MTD-vs-budget dashboard** in OrgSettings (visual progress bar, "approaching limit" warnings). Genuine UX work; separate phase.
3. **Per-key MTD display** in the keys table. Same — UX-driven, separate phase.
4. **Budget alerts** (email/Slack when MTD crosses 80% of budget). Requires notification infrastructure; out of scope.
5. **Pre-dispatch estimation** (reject before doing upstream work). Rejected for Phase 6 due to output-token cost uncertainty; could be revisited if budget leaks become operationally painful.
6. **Counter reconciliation job** (periodic `accrued = SUM(cost)` recomputation to catch drift). Deferred — under normal operation, the transactional write in `record_usage` prevents drift. Worth adding if/when usage_records gets backfilled out-of-band.
7. **Budget reset notifications** (email when month rolls over and accrual resets). Separate concern.
8. **Redis-backed distributed counter** — `budget_counters` is a Postgres table, so multi-node correctness is automatic (unlike Phase 5's in-memory rate limiter). No issue here.
