# Phase 7 — Budget Observability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface Phase 6's month-to-date (MTD) spend in two read-only UI surfaces (OrgSettings rollup card with color-coded progress bar, Keys table per-key MTD column) backed by one new read-only endpoint (`GET /{slug}/budget-status`) and one extended endpoint (`GET /{slug}/keys` gains `mtd_units`).

**Architecture:** No schema changes — Phase 7 reads Phase 6's `budget_counters` table directly. New storage trait method `get_org_month_to_date_spend(org_id)` does a `SUM(accrued)` join against `api_keys`; new storage method `list_keys_paginated_with_mtd` LEFT JOINs `budget_counters` for the keys listing. New `/budget-status` endpoint is org-admin gated (matches Phase 5 `/defaults`); `KeyResponse` gains additive `mtd_units: i64`. Frontend gets a pure `budgetColor.ts` helper + reusable `ProgressBar.tsx`, then OrgSettings renders a `BudgetStatusSection` and the Keys table gets a new MTD column.

**Tech Stack:** Rust (sqlx Postgres, Axum, chrono for UTC month bucketing); React/TypeScript (React Query, i18next, Tailwind, framer-motion, vitest + Playwright).

**Spec:** `docs/superpowers/specs/2026-07-10-saas-phase7-budget-observability-design.md`

---

## File Structure

**Backend (Rust):**

- `crates/storage/src/lib.rs` — add two trait methods: `get_org_month_to_date_spend`, `list_keys_paginated_with_mtd`
- `crates/storage/src/postgres.rs` — impl both methods; add new row struct `PgKeyWithMtdRow`; add Phase 7 storage tests
- `crates/api/src/management/keys.rs` — extend `KeyResponse` with `mtd_units: i64`; switch `list_keys` to the new storage method
- `crates/api/src/management/mod.rs` — register `/budget-status` route
- `crates/api/src/management/budget.rs` — **new file**: `BudgetStatusResponse` type + `get_budget_status` handler
- `crates/api/src/auth.rs` — re-export `get_budget_status` (route handler lives in management/budget.rs but the org-defaults handlers already live in auth.rs, so follow whichever import path `mod.rs` already uses; concretely: handler lives in management/budget.rs and we import it from there)
- `crates/api/tests/phase7_budget_status.rs` — **new file**: API integration tests

**Frontend (React/TypeScript):**

- `web/src/api/orgs.ts` — add `BudgetStatus` type + `getBudgetStatus` function
- `web/src/types/index.ts` — extend `ApiKey` interface with `mtd_units: number`
- `web/src/hooks/useBudgetStatus.ts` — **new file**: `useGetBudgetStatus` React Query hook
- `web/src/lib/budgetColor.ts` — **new file**: `budgetBarColor` + `budgetUsedPct` pure helpers
- `web/src/lib/budgetColor.test.ts` — **new file**: vitest unit tests for thresholds
- `web/src/components/ui/ProgressBar.tsx` — **new file**: reusable color-coded bar
- `web/src/components/ui/ProgressBar.test.tsx` — **new file**: vitest unit tests
- `web/src/pages/OrgSettings.tsx` — add `<BudgetStatusSection />` below `<DefaultsSection />`
- `web/src/pages/Keys.tsx` — add MTD column to the existing table
- `web/src/i18n/en.json` + `web/src/i18n/zh.json` — new keys for budget status + Keys column

**Tests/E2E:**

- `web/e2e/budget-status.spec.ts` — **new file**: login → seed usage → assert OrgSettings card + Keys column render non-zero values

**Docs:**

- `CHANGELOG.md` — Phase 7 entry under `[Unreleased] → Added`

---

## Task 1: Storage — `get_org_month_to_date_spend` trait method + impl

Add a SUM-over-budget_counters method so the `/budget-status` endpoint can compute org-wide MTD in one indexed query.

**Files:**
- Modify: `crates/storage/src/lib.rs` (add trait method declaration)
- Modify: `crates/storage/src/postgres.rs` (add impl)
- Test: tests inline in `crates/storage/src/postgres.rs` (extend Phase 6 test module)

- [ ] **Step 1: Write the failing storage test**

Open `crates/storage/src/postgres.rs`. Find the `phase6_tests` mod closing brace (search for `budget_counters_concurrent_inserts` — the test directly above it is the last test in that mod). Add these new tests **inside the same `phase6_tests` mod** (sibling to the Phase 6 tests), reusing the existing `make_test_org`, `make_test_key_for_budget`, and `mk_usage` helpers:

```rust
    /// Unknown org → 0 (no keys, no counters).
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_returns_zero_for_unknown_org(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        // Brand-new org has no keys → SUM must be 0, not null, not an error.
        let org = make_test_org(&storage, "org-mtd-empty", "Empty Org").await;
        let mtd = storage
            .get_org_month_to_date_spend(&org.id)
            .await
            .expect("get_org_month_to_date_spend on empty org");
        assert_eq!(mtd, 0, "empty org must report 0 MTD");
    }

    /// 3 keys, each with $5 spend this month → org total = $15.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_sums_across_keys(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-mtd-sum", "Sum Org").await;

        let five_usd = crate::money::usd_to_units(5.0);
        for n in 0..3 {
            let key_id = make_test_key_for_budget(&storage, &org.id, &format!("key-mtd-{n}")).await;
            storage
                .record_usage(
                    &org.id,
                    &mk_usage(&org.id, &key_id, five_usd, chrono::Utc::now()),
                )
                .await
                .expect("record_usage");
        }

        let mtd = storage
            .get_org_month_to_date_spend(&org.id)
            .await
            .expect("get_org_month_to_date_spend");
        assert_eq!(mtd, five_usd * 3, "MTD must sum across all keys in the org");
    }

    /// A record dated 40 days ago lands in a prior month bucket and must NOT count.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_excludes_other_months(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-mtd-prior", "Prior Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-prior").await;

        let old_ts = chrono::Utc::now() - chrono::Duration::days(40);
        let old_cost = crate::money::usd_to_units(10.0);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, old_cost, old_ts))
            .await
            .expect("record_usage old");

        let mtd = storage
            .get_org_month_to_date_spend(&org.id)
            .await
            .expect("get_org_month_to_date_spend");
        assert_eq!(mtd, 0, "prior-month spend must not count toward current MTD");
    }

    /// Key in org A's spend must NOT bleed into org B's MTD.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_mtd_no_cross_org_leak(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org_a = make_test_org(&storage, "org-mtd-a", "Org A").await;
        let org_b = make_test_org(&storage, "org-mtd-b", "Org B").await;

        let key_a = make_test_key_for_budget(&storage, &org_a.id, "key-a").await;
        let key_b = make_test_key_for_budget(&storage, &org_b.id, "key-b").await;

        let cost = crate::money::usd_to_units(7.0);
        storage
            .record_usage(&org_a.id, &mk_usage(&org_a.id, &key_a, cost, chrono::Utc::now()))
            .await
            .expect("record_usage a");
        storage
            .record_usage(&org_b.id, &mk_usage(&org_b.id, &key_b, cost, chrono::Utc::now()))
            .await
            .expect("record_usage b");

        let mtd_a = storage
            .get_org_month_to_date_spend(&org_a.id)
            .await
            .expect("get_org_month_to_date_spend a");
        assert_eq!(mtd_a, cost, "org A's MTD must exclude org B's spend");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p llm-gateway-storage get_org_mtd -- --nocapture`
Expected: FAIL — compile error: `no method named get_org_month_to_date_spend found`

- [ ] **Step 3: Add the trait method declaration**

In `crates/storage/src/lib.rs`, find the trait `Storage` and locate the `get_month_to_date_spend` declaration (Phase 6, around line 99). Add immediately below it:

```rust
    /// Returns the org-wide month-to-date spend in 10^8 subunits per USD,
    /// summing `budget_counters.accrued` across all keys in the org for the
    /// current UTC calendar month. Returns 0 when the org has no spend this
    /// month (including when the org has no keys at all). Read-time SUM; no
    /// materialized `org_budget_counters` table — see Phase 7 design doc.
    async fn get_org_month_to_date_spend(
        &self,
        org_id: &str,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 4: Add the impl**

In `crates/storage/src/postgres.rs`, find `get_month_to_date_spend` impl (Phase 6, around line 1434). Add immediately below it:

```rust
    async fn get_org_month_to_date_spend(&self, org_id: &str) -> Result<i64, DbErr> {
        let month_bucket = format!("{}", chrono::Utc::now().format("%Y-%m"));
        // LEFT JOIN would also work, but we want 0 (not NULL) when no rows —
        // COALESCE on the inner SUM does that without an extra outer wrapper.
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COALESCE(SUM(bc.accrued), 0)
             FROM budget_counters bc
             JOIN api_keys ak ON ak.id = bc.key_id
             WHERE ak.org_id = $1 AND bc.month_bucket = $2",
        )
        .bind(org_id)
        .bind(&month_bucket)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(v,)| v).unwrap_or(0))
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p llm-gateway-storage get_org_mtd -- --nocapture`
Expected: PASS — 4 tests, 0 failures.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/postgres.rs
git commit -m "phase7: add Storage::get_org_month_to_date_spend

Org-wide SUM(accrued) over budget_counters joined to api_keys for the
current UTC month. Read-time aggregation per Phase 7 design — no
materialized org_budget_counters table."
```

---

## Task 2: Storage — `list_keys_paginated_with_mtd` trait method + impl

A second storage method that LEFT JOINs budget_counters so the keys-listing endpoint returns per-key MTD in one SQL round-trip (no N+1).

**Files:**
- Modify: `crates/storage/src/lib.rs` (add trait method)
- Modify: `crates/storage/src/postgres.rs` (add impl + new row struct)
- Modify: `crates/storage/src/types.rs` (add `ApiKeyWithMtd` struct)
- Test: tests inline in `crates/storage/src/postgres.rs`

- [ ] **Step 1: Add the `ApiKeyWithMtd` type**

Open `crates/storage/src/types.rs`. Find the `pub struct ApiKey { ... }` definition. Add immediately below it:

```rust
/// ApiKey + its current UTC-month MTD spend. The MTD field is `0` when the
/// key has no budget_counters row this month (mirrors SQL `COALESCE(..., 0)`).
/// Used by the Phase 7 keys-listing endpoint so the UI can render per-key
/// spend in one round-trip without an N+1 of `get_month_to_date_spend`.
#[derive(Debug, Clone)]
pub struct ApiKeyWithMtd {
    pub key: ApiKey,
    pub mtd_units: i64,
}
```

- [ ] **Step 2: Re-export the new type from the storage crate root**

In `crates/storage/src/lib.rs`, find the `pub use types::{` block (around lines 10-17). Add `ApiKeyWithMtd` to that export list. After the edit the block looks like (additive only):

```rust
pub use types::{
    *,
    Account, Transaction, TransactionType,
    AccountResponse, TransactionResponse,
    CreateTransaction, UpdateAccountThreshold,
    DeductBalance, DeductBalanceResult,
    AddBalance, AddBalanceResult,
    ApiKeyWithMtd,
};
```

- [ ] **Step 3: Write the failing storage test**

Open `crates/storage/src/postgres.rs`. In the `phase6_tests` mod, below the four tests added in Task 1, add:

```rust
    /// Keys without spend this month report `mtd_units: 0`.
    /// Keys with spend report the correct accrued sum.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_keys_with_mtd_includes_per_key_spend(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-keys-mtd", "Keys Mtd Org").await;

        // key-with-cost: $4 spend this month
        let key_with_cost =
            make_test_key_for_budget(&storage, &org.id, "key-with-cost").await;
        let four_usd = crate::money::usd_to_units(4.0);
        storage
            .record_usage(
                &org.id,
                &mk_usage(&org.id, &key_with_cost, four_usd, chrono::Utc::now()),
            )
            .await
            .expect("record_usage");

        // key-no-cost: no usage records
        let key_no_cost =
            make_test_key_for_budget(&storage, &org.id, "key-no-cost").await;

        let result = storage
            .list_keys_paginated_with_mtd(&org.id, 1, 50)
            .await
            .expect("list_keys_paginated_with_mtd");

        // 2 keys total
        assert_eq!(result.total, 2, "should see both keys");

        let by_id: std::collections::HashMap<String, i64> = result
            .items
            .iter()
            .map(|x| (x.key.id.clone(), x.mtd_units))
            .collect();
        assert_eq!(
            by_id.get(&key_with_cost),
            Some(&four_usd),
            "key with $4 usage must report mtd_units = $4 in subunits"
        );
        assert_eq!(
            by_id.get(&key_no_cost),
            Some(&0),
            "key with no usage must report mtd_units = 0"
        );
    }

    /// A prior-month spend must NOT show up in the current month's MTD column.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_keys_with_mtd_excludes_other_months(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-keys-prior", "Keys Prior Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-prior-mtd").await;

        let old_ts = chrono::Utc::now() - chrono::Duration::days(40);
        let old_cost = crate::money::usd_to_units(20.0);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, old_cost, old_ts))
            .await
            .expect("record_usage old");

        let result = storage
            .list_keys_paginated_with_mtd(&org.id, 1, 50)
            .await
            .expect("list_keys_paginated_with_mtd");
        let row = result
            .items
            .iter()
            .find(|x| x.key.id == key_id)
            .expect("key must be in result");
        assert_eq!(row.mtd_units, 0, "prior-month spend must not count toward current MTD");
    }
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p llm-gateway-storage list_keys_with_mtd -- --nocapture`
Expected: FAIL — compile error: `no method named list_keys_paginated_with_mtd found`

- [ ] **Step 5: Add the trait method declaration**

In `crates/storage/src/lib.rs`, find the `list_keys_paginated_for_user` declaration (around line 30) and add immediately below it:

```rust
    /// Like `list_keys_paginated` but LEFT JOINs `budget_counters` so each
    /// returned item carries its current-month MTD spend (`mtd_units`).
    /// Used by Phase 7 keys-listing endpoint. Single SQL round-trip; no N+1.
    async fn list_keys_paginated_with_mtd(
        &self,
        org_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<PaginatedResponse<crate::types::ApiKeyWithMtd>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 6: Add the impl + new row struct**

In `crates/storage/src/postgres.rs`, find `list_keys_paginated` impl (around line 784). Add immediately below it:

```rust
    async fn list_keys_paginated_with_mtd(
        &self,
        org_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<PaginatedResponse<crate::types::ApiKeyWithMtd>, DbErr> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await?;
        let offset = (page - 1) * page_size;
        let month_bucket = format!("{}", chrono::Utc::now().format("%Y-%m"));
        let rows: Vec<PgKeyWithMtdRow> = sqlx::query_as(
            "SELECT ak.id, ak.org_id, ak.name, ak.key_hash, ak.key_prefix,
                    ak.rate_limit, ak.budget_monthly, ak.enabled, ak.created_by,
                    ak.model_fallback_id, ak.created_at, ak.updated_at,
                    COALESCE(bc.accrued, 0) AS mtd_units
             FROM api_keys ak
             LEFT JOIN budget_counters bc
               ON bc.key_id = ak.id AND bc.month_bucket = $2
             WHERE ak.org_id = $1
             ORDER BY ak.created_at DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(org_id)
        .bind(&month_bucket)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(PaginatedResponse {
            items: rows.into_iter().map(crate::types::ApiKeyWithMtd::from).collect(),
            total: total.0,
            page,
            page_size,
        })
    }
```

Then find the existing `struct PgKeyRow` (around line 42). Add the new row struct **immediately below `PgKeyRow`** (and the existing `impl From<PgKeyRow> for ApiKey`):

```rust
struct PgKeyWithMtdRow {
    id: String,
    org_id: String,
    name: String,
    key_hash: String,
    key_prefix: Option<String>,
    rate_limit: Option<i32>,
    budget_monthly: Option<i64>,
    enabled: bool,
    created_by: Option<String>,
    model_fallback_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    mtd_units: i64,
}

impl From<PgKeyWithMtdRow> for crate::types::ApiKeyWithMtd {
    fn from(r: PgKeyWithMtdRow) -> Self {
        // The ApiKey-from-PgKeyRow conversion logic is mirrored here because
        // rust doesn't auto-derive struct re-shaping. Keep the field list in
        // lock-step with `impl From<PgKeyRow> for ApiKey` above.
        crate::types::ApiKeyWithMtd {
            key: ApiKey {
                id: r.id,
                org_id: r.org_id,
                name: r.name,
                key_hash: r.key_hash,
                key_prefix: r.key_prefix,
                rate_limit: r.rate_limit.map(|v| v as i64),
                budget_monthly: r.budget_monthly,
                enabled: r.enabled,
                created_by: r.created_by,
                model_fallback_id: r.model_fallback_id,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
            mtd_units: r.mtd_units,
        }
    }
}
```

> **Note for the implementer:** the `ApiKey` field list above must match the real `ApiKey` struct definition. Read `crates/storage/src/types.rs::ApiKey` first and adjust the field list if it differs. The existing `impl From<PgKeyRow> for ApiKey` (around line 60 of `postgres.rs`) is the source of truth — copy its body verbatim, only adding `mtd_units` to the outer wrapper.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p llm-gateway-storage list_keys_with_mtd -- --nocapture`
Expected: PASS — 2 tests, 0 failures.

- [ ] **Step 8: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/src/postgres.rs crates/storage/src/types.rs
git commit -m "phase7: add Storage::list_keys_paginated_with_mtd

LEFT JOIN api_keys against budget_counters for the current UTC month so
each key row carries its per-key MTD spend in one round-trip. No N+1."
```

---

## Task 3: API — `GET /{slug}/budget-status` handler + route registration

New read-only endpoint that returns org-wide accrued + month_bucket. Org-admin gated via the same `can_manage_org_settings` check Phase 5's `/defaults` PUT uses (member+ can read; admin+ can write — but this endpoint is GET-only, so the read gate is just membership, which is enforced upstream by `membership_layer`).

**Files:**
- Create: `crates/api/src/management/budget.rs`
- Modify: `crates/api/src/management/mod.rs` (export module + register route)
- Test: `crates/api/tests/phase7_budget_status.rs` (created in Task 4)

- [ ] **Step 1: Create the handler module**

Create `crates/api/src/management/budget.rs`:

```rust
//! Phase 7: read-only budget observability endpoints.
//!
//! `GET /api/v1/{org_slug}/budget-status` returns the org's current UTC-month
//! accrued spend in 10^8 subunits per USD, plus the `YYYY-MM` month bucket so
//! the frontend can display it. The org-wide default *budget* value is NOT
//! returned here — the frontend composes it from `GET /{slug}/defaults` (one
//! source of truth per datum). Read-only: no write endpoints in this phase.

use axum::extract::State;
use axum::Json;
use chrono::{Datelike, Utc};
use serde::Serialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::AppState;
use llm_gateway_org::OrgContext;

/// Response body for `GET /api/v1/{org_slug}/budget-status`.
#[derive(Debug, Serialize)]
pub struct BudgetStatusResponse {
    /// Month-to-date spend in 10^8 subunits per USD. `0` when the org has no
    /// usage this month. Frontend converts to USD at the rendering boundary
    /// via the existing `unitsToUsd` helper.
    pub accrued_units: i64,
    /// UTC calendar month in `YYYY-MM` form. Matches the bucket Phase 6's
    /// `budget_counters` rows are keyed by.
    pub month_bucket: String,
}

/// GET /api/v1/{org_slug}/budget-status — read org-wide MTD spend.
///
/// Membership is enforced upstream by `membership_layer` before this handler
/// runs (same gate as `GET /{slug}/defaults` in Phase 5). The handler itself
/// is a thin read: one indexed SUM, one bucket string.
pub async fn get_budget_status(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<BudgetStatusResponse>, ApiError> {
    let accrued_units = state
        .storage
        .get_org_month_to_date_spend(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let now = Utc::now();
    let month_bucket = format!("{:04}-{:02}", now.year(), now.month());

    Ok(Json(BudgetStatusResponse {
        accrued_units,
        month_bucket,
    }))
}
```

- [ ] **Step 2: Wire the module + route**

In `crates/api/src/management/mod.rs`, find the existing `mod keys;` / `pub mod keys;` line (near the top of the file — search for `mod auth;` to find the module declaration block) and add:

```rust
pub mod budget;
```

Then in the same file, find `fn org_scoped_routes()` (around line 167). Locate the existing `/defaults` route registration (around lines 178-181):

```rust
        .route(
            "/defaults",
            get(auth::get_org_defaults).put(auth::update_org_defaults),
        )
```

Add immediately below it:

```rust
        // Org-wide MTD spend (Phase 7). GET = member+. Read-only; no PUT.
        .route(
            "/budget-status",
            get(budget::get_budget_status),
        )
```

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo build -p llm-gateway-api`
Expected: build succeeds with no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/budget.rs crates/api/src/management/mod.rs
git commit -m "phase7: add GET /{slug}/budget-status endpoint

Read-only org-wide MTD accrued + month_bucket. Membership-gated upstream
by membership_layer (same gate as /defaults)."
```

---

## Task 4: API — extend `KeyResponse` with `mtd_units` + switch `list_keys` to use the new storage method

Additive field on the existing keys-listing response; the handler now branches on `can_manage_channels` like before, but uses the new `_with_mtd` storage method on the admin path. (Member path stays on the original `list_keys_paginated_for_user` — Phase 7 doesn't introduce MTD on the user-scoped listing to keep scope tight; members still see their own keys with `mtd_units: 0`. If a member's keys have spend, the field is still 0 in the response. This matches the spec's "non-goal" of per-user budget views.)

> **Note for the implementer:** verify whether the simpler path — always use `list_keys_paginated_with_mtd` for both branches — is feasible by reading the existing `list_keys_paginated_for_user` SQL. If the for-user variant is just `list_keys_paginated` + `AND created_by = $2`, you can skip the branch and use the with-mtd variant directly. For safety, this task keeps the existing branch and only swaps the admin path. If the implementer chooses to also swap the member path, that's fine — both paths return `ApiKeyWithMtd`.

**Files:**
- Modify: `crates/api/src/management/keys.rs` (extend `KeyResponse`; switch `list_keys` storage call)

- [ ] **Step 1: Extend `KeyResponse` and its `From` impl**

Open `crates/api/src/management/keys.rs`. Find the `pub struct KeyResponse { ... }` definition (around line 50). Add `mtd_units: i64` as a new field at the bottom (above the closing brace):

```rust
#[derive(Serialize)]
pub struct KeyResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: Option<String>,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub enabled: bool,
    pub created_by: Option<String>,
    pub model_fallback_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Phase 7: current UTC-month MTD spend in 10^8 subunits per USD. `0`
    /// when the key has no usage this month. Additive field — existing API
    /// consumers ignore it.
    pub mtd_units: i64,
}
```

The existing `impl From<ApiKey> for KeyResponse` (around line 63) cannot synthesize `mtd_units` because the source `ApiKey` doesn't carry it. **Leave that `From<ApiKey>` impl intact** (it's still used by `get_key`, `create_key`, `update_key`, and the member-path of `list_keys`) but extend it to set `mtd_units: 0`:

```rust
impl From<ApiKey> for KeyResponse {
    fn from(k: ApiKey) -> Self {
        KeyResponse {
            id: k.id,
            name: k.name,
            key_prefix: k.key_prefix,
            rate_limit: k.rate_limit,
            budget_monthly: opt_units_to_usd(k.budget_monthly),
            enabled: k.enabled,
            created_by: k.created_by,
            model_fallback_id: k.model_fallback_id,
            created_at: k.created_at,
            updated_at: k.updated_at,
            mtd_units: 0,
        }
    }
}
```

Then add a second `From` impl immediately below it for the new storage type:

```rust
impl From<llm_gateway_storage::ApiKeyWithMtd> for KeyResponse {
    fn from(x: llm_gateway_storage::ApiKeyWithMtd) -> Self {
        let k = x.key;
        KeyResponse {
            id: k.id,
            name: k.name,
            key_prefix: k.key_prefix,
            rate_limit: k.rate_limit,
            budget_monthly: opt_units_to_usd(k.budget_monthly),
            enabled: k.enabled,
            created_by: k.created_by,
            model_fallback_id: k.model_fallback_id,
            created_at: k.created_at,
            updated_at: k.updated_at,
            mtd_units: x.mtd_units,
        }
    }
}
```

- [ ] **Step 2: Switch `list_keys` admin branch to the new storage method**

In the same file, find `pub async fn list_keys` (around line 122). Currently the admin branch calls `list_keys_paginated`; the member branch calls `list_keys_paginated_for_user`. Change **only the admin branch** to use `list_keys_paginated_with_mtd`, and map through the new `From` impl:

```rust
pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<KeyResponse>>, ApiError> {
    let (page, page_size) = pagination.normalized();
    // Admin-or-above in this org (or platform_admin) sees all keys in the org
    // with their current-month MTD spend (Phase 7). A regular member sees
    // only the keys they created; their rows report mtd_units: 0 (Phase 7
    // focuses on the admin-view; per-user budget views are out of scope).
    if can_manage_channels(&ctx) {
        let result = state
            .storage
            .list_keys_paginated_with_mtd(&ctx.org_id, page, page_size)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok(Json(PaginatedResponse {
            items: result
                .items
                .into_iter()
                .map(KeyResponse::from)
                .collect(),
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        }))
    } else {
        let result = state
            .storage
            .list_keys_paginated_for_user(&ctx.org_id, &ctx.user_id, page, page_size)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok(Json(PaginatedResponse {
            items: result.items.into_iter().map(KeyResponse::from).collect(),
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        }))
    }
}
```

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo build -p llm-gateway-api`
Expected: build succeeds with no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/keys.rs
git commit -m "phase7: add mtd_units to KeyResponse; switch list_keys admin path

list_keys paginated admin branch now calls list_keys_paginated_with_mtd
so each returned key carries its current-month accrued spend. Additive
JSON field — existing consumers ignore it."
```

---

## Task 5: API integration tests — Phase 7 endpoints

Integration tests covering the new `/budget-status` endpoint and the `mtd_units` field on `list_keys`. Reuses the existing `common::seed_org_with_admin` helper.

**Files:**
- Create: `crates/api/tests/phase7_budget_status.rs`

- [ ] **Step 1: Write the test file**

Create `crates/api/tests/phase7_budget_status.rs`:

```rust
//! Integration tests for Phase 7 budget observability endpoints.
//!
//! Covers `GET /api/v1/{slug}/budget-status` (new) and the `mtd_units` field
//! on the existing `GET /api/v1/{slug}/keys` response.

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router(state.clone()).with_state(state)
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get(app: &axum::Router, uri: &str, token: &str) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", bearer(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Helper: insert a usage_records row + matching budget_counters row directly
/// (bypasses record_usage; Phase 6 already tested that path). Costs are in
/// 10^8 subunits per USD.
async fn seed_spend(pool: &PgPool, org_id: &str, key_id: &str, cost_units: i64) {
    let now = chrono::Utc::now();
    let month_bucket = format!("{}", now.format("%Y-%m"));
    sqlx::query(
        "INSERT INTO usage_records (id, org_id, request_id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost, pricing_policy, weighted_tokens, user_id, created_at)
         VALUES ($1, $2, NULL, $3, 'test-model', 'test-provider', NULL, 'openai', 0, 0, NULL, NULL, $4, NULL, 0, NULL, $5)",
    )
    .bind(format!("rec-{}", uuid::Uuid::new_v4()))
    .bind(org_id)
    .bind(key_id)
    .bind(cost_units)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed usage_records");

    sqlx::query(
        "INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (key_id, month_bucket)
         DO UPDATE SET accrued = budget_counters.accrued + EXCLUDED.accrued, updated_at = NOW()",
    )
    .bind(key_id)
    .bind(&month_bucket)
    .bind(cost_units)
    .execute(pool)
    .await
    .expect("seed budget_counters");
}

/// 1. Fresh org → 200 with accrued_units: 0.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_zero_for_fresh_org(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool).await;

    let resp = get(&app, &format!("/api/v1/{slug}/budget-status"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["accrued_units"], 0, "fresh org has 0 MTD");
    // month_bucket is YYYY-MM shaped.
    let bucket = body["month_bucket"].as_str().unwrap();
    assert!(regex_like_yyyymm(bucket), "month_bucket must look like YYYY-MM: got {bucket}");
}

/// 2. Seeded spend → 200 with correct accrued_units.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_returns_accrued(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool).await;

    // Look up the seeded org_id + create a key so we can attach spend.
    let org_id: String = sqlx::query_scalar("SELECT id FROM orgs WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("org by slug");
    let key_id = format!("key-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO api_keys (id, org_id, name, key_hash, key_prefix, enabled, created_at, updated_at)
         VALUES ($1, $2, 'test', $3, NULL, true, $4, $5)",
    )
    .bind(&key_id)
    .bind(&org_id)
    .bind(format!("{key_id:0>64}"))
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed api_key");

    let five_usd = 500_000_000_i64;
    seed_spend(&pool, &org_id, &key_id, five_usd).await;

    let resp = get(&app, &format!("/api/v1/{slug}/budget-status"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["accrued_units"], five_usd, "endpoint must reflect seeded spend");
}

/// 3. Non-member → 403.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_403_for_non_member(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (_owner_token, slug) = common::seed_org_with_admin(&pool).await;
    // Seed a second org + a member of THAT org; their JWT resolves to org B
    // so they cannot read org A's budget-status even with a valid token.
    let (other_token, _other_slug) = common::seed_org_with_admin(&pool).await;

    let resp = get(&app, &format!("/api/v1/{slug}/budget-status"), &other_token).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// 4. No bearer → 401.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_budget_status_401_unauthenticated(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (_token, slug) = common::seed_org_with_admin(&pool).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/{slug}/budget-status"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 5. list_keys response includes mtd_units field per key, value matches seeded spend.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_keys_includes_mtd_field(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool).await;
    let org_id: String = sqlx::query_scalar("SELECT id FROM orgs WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("org by slug");

    // Create a key with $3 of spend this month.
    let key_id = format!("key-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO api_keys (id, org_id, name, key_hash, key_prefix, enabled, created_at, updated_at)
         VALUES ($1, $2, 'test', $3, NULL, true, $4, $5)",
    )
    .bind(&key_id)
    .bind(&org_id)
    .bind(format!("{key_id:0>64}"))
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed api_key");
    let three_usd = 300_000_000_i64;
    seed_spend(&pool, &org_id, &key_id, three_usd).await;

    let resp = get(&app, &format!("/api/v1/{slug}/keys?page=1&page_size=50"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let items = body["items"].as_array().expect("items is array");
    let target = items
        .iter()
        .find(|k| k["id"] == key_id)
        .expect("seeded key must be in response");
    assert_eq!(target["mtd_units"], three_usd, "mtd_units must equal seeded spend");
}

/// Cheap shape check: a string of form `YYYY-MM`.
fn regex_like_yyyymm(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 7
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p llm-gateway-api --test phase7_budget_status -- --nocapture`
Expected: PASS — 5 tests, 0 failures.

If `seed_spend`'s INSERT column list doesn't match the real `usage_records` schema, fix the column list to match what `mk_usage` in `crates/storage/src/postgres.rs` populates (look for `UsageRecord { ... }` literal around line 4709). The columns there are the source of truth.

- [ ] **Step 3: Commit**

```bash
git add crates/api/tests/phase7_budget_status.rs
git commit -m "phase7: add API integration tests for budget-status + mtd_units"
```

---

## Task 6: Frontend — `getBudgetStatus` API client + `BudgetStatus` type

Add the typed client function for the new endpoint. Follows the existing `getOrgDefaults` shape exactly.

**Files:**
- Modify: `web/src/api/orgs.ts`

- [ ] **Step 1: Add the type + function**

Open `web/src/api/orgs.ts`. Add at the bottom of the file:

```typescript
/**
 * Response from `GET /api/v1/{org_slug}/budget-status` (Phase 7).
 * `accrued_units` is in 10^8 subunits per USD — convert to USD at the
 * rendering boundary via the `unitsToUsd` helper. `month_bucket` is the
 * UTC calendar month (`YYYY-MM`) the accrual is counted against.
 */
export type BudgetStatus = {
  accrued_units: number;
  month_bucket: string;
};

export async function getBudgetStatus(): Promise<BudgetStatus> {
  const { data } = await apiClient.get<BudgetStatus>(`${orgPrefix()}/budget-status`);
  return data;
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/api/orgs.ts
git commit -m "phase7: add getBudgetStatus client + BudgetStatus type"
```

---

## Task 7: Frontend — extend `ApiKey` type with `mtd_units`

The existing `ApiKey` interface needs to carry the new field so consumers (Keys page) can render it.

**Files:**
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Extend the interface**

Open `web/src/types/index.ts`. Find the `export interface ApiKey { ... }` block (line 1) and add `mtd_units: number;` to it:

```typescript
export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  key_prefix: string | null;
  rate_limit: number | null;
  budget_monthly: number | null;
  enabled: boolean;
  model_fallback_id: string | null;
  created_at: string;
  updated_at: string;
  /** Phase 7: current UTC-month MTD spend in 10^8 subunits per USD.
   *  `0` when the key has no usage this month. */
  mtd_units: number;
}
```

- [ ] **Step 2: Typecheck**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/types/index.ts
git commit -m "phase7: add mtd_units field to ApiKey type"
```

---

## Task 8: Frontend — `useGetBudgetStatus` hook

React Query wrapper around `getBudgetStatus`. Mirrors `useGetOrgDefaults` exactly.

**Files:**
- Create: `web/src/hooks/useBudgetStatus.ts`

- [ ] **Step 1: Create the hook file**

Create `web/src/hooks/useBudgetStatus.ts`:

```typescript
import { useQuery } from '@tanstack/react-query';
import { useAuthStore } from '../stores/authStore';
import { getBudgetStatus } from '../api/orgs';

/**
 * Phase 7: subscribe to the org's current-month MTD spend + month bucket.
 * Pairs with `useGetOrgDefaults` (which returns the budget cap) to render
 * the Budget status card on the OrgSettings page.
 */
export function useGetBudgetStatus() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'budgetStatus'],
    queryFn: () => getBudgetStatus(),
    enabled: !!slug,
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/hooks/useBudgetStatus.ts
git commit -m "phase7: add useGetBudgetStatus React Query hook"
```

---

## Task 9: Frontend — `budgetColor.ts` pure helper + unit tests

Pure functions that decide the bar color and percentage from raw inputs. Shared by OrgSettings card + Keys column.

**Files:**
- Create: `web/src/lib/budgetColor.ts`
- Create: `web/src/lib/budgetColor.test.ts`

- [ ] **Step 1: Write the failing unit tests**

Create `web/src/lib/budgetColor.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { budgetBarColor, budgetUsedPct } from './budgetColor';

describe('budgetUsedPct', () => {
  it('returns null when budget is null', () => {
    expect(budgetUsedPct(100, null)).toBeNull();
  });

  it('returns null when budget is 0', () => {
    expect(budgetUsedPct(0, 0)).toBeNull();
    expect(budgetUsedPct(100, 0)).toBeNull();
  });

  it('returns 0 when accrued is 0', () => {
    expect(budgetUsedPct(0, 100)).toBe(0);
  });

  it('returns the percentage at boundary points', () => {
    expect(budgetUsedPct(50, 100)).toBe(50);
    expect(budgetUsedPct(80, 100)).toBe(80);
    expect(budgetUsedPct(100, 100)).toBe(100);
    expect(budgetUsedPct(105, 100)).toBe(105);
  });
});

describe('budgetBarColor', () => {
  it('returns muted class when pct is null', () => {
    expect(budgetBarColor(null)).toBe('bg-muted');
  });

  it('returns green below 60%', () => {
    expect(budgetBarColor(0)).toBe('bg-emerald-500');
    expect(budgetBarColor(30)).toBe('bg-emerald-500');
    expect(budgetBarColor(59)).toBe('bg-emerald-500');
  });

  it('returns amber at 60% inclusive to 80% exclusive', () => {
    expect(budgetBarColor(60)).toBe('bg-amber-500');
    expect(budgetBarColor(79)).toBe('bg-amber-500');
  });

  it('returns orange at 80% inclusive to 100% inclusive', () => {
    expect(budgetBarColor(80)).toBe('bg-orange-500');
    expect(budgetBarColor(99)).toBe('bg-orange-500');
    expect(budgetBarColor(100)).toBe('bg-orange-500');
  });

  it('returns red over 100%', () => {
    expect(budgetBarColor(101)).toBe('bg-red-500');
    expect(budgetBarColor(105)).toBe('bg-red-500');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- src/lib/budgetColor.test.ts`
Expected: FAIL — cannot resolve `./budgetColor`.

- [ ] **Step 3: Implement the helper**

Create `web/src/lib/budgetColor.ts`:

```typescript
/**
 * Phase 7 budget-observability color helpers.
 *
 * Shared by the OrgSettings "Budget status" card and the Keys-table MTD
 * column. Thresholds match the design spec:
 *   - null pct (no budget set) → muted gray, no bar fill
 *   - < 60% → green
 *   - 60–79% → amber
 *   - 80–100% → orange
 *   - > 100% → red
 *
 * Returned strings are Tailwind class names; consumers apply them directly
 * to the bar's `className`. The classes used here must exist in the project's
 * Tailwind setup (verified: `bg-emerald-500`, `bg-amber-500`, `bg-orange-500`,
 * `bg-red-500`, and `bg-muted` are all standard DaisyUI / Tailwind tokens).
 */

export function budgetUsedPct(accruedUnits: number, budgetUnits: number | null): number | null {
  if (budgetUnits === null || budgetUnits === 0) return null;
  return (accruedUnits / budgetUnits) * 100;
}

export function budgetBarColor(usedPct: number | null): string {
  if (usedPct === null) return 'bg-muted';
  if (usedPct > 100) return 'bg-red-500';
  if (usedPct >= 80) return 'bg-orange-500';
  if (usedPct >= 60) return 'bg-amber-500';
  return 'bg-emerald-500';
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- src/lib/budgetColor.test.ts`
Expected: PASS — 12 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/budgetColor.ts web/src/lib/budgetColor.test.ts
git commit -m "phase7: add budgetColor helpers + unit tests"
```

---

## Task 10: Frontend — `ProgressBar.tsx` component + unit tests

A tiny reusable presentational component: renders a track + a fill whose width and color come from props.

**Files:**
- Create: `web/src/components/ui/ProgressBar.tsx`
- Create: `web/src/components/ui/ProgressBar.test.tsx`

- [ ] **Step 1: Write the failing component tests**

Create `web/src/components/ui/ProgressBar.test.tsx`:

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ProgressBar } from './ProgressBar';

describe('ProgressBar', () => {
  it('renders a track with a fill', () => {
    const { container } = render(<ProgressBar pct={50} colorClass="bg-emerald-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement | null;
    expect(fill).not.toBeNull();
    expect(fill!.style.width).toBe('50%');
    expect(fill!.className).toContain('bg-emerald-500');
  });

  it('clamps pct above 100 to width 100%', () => {
    const { container } = render(<ProgressBar pct={150} colorClass="bg-red-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement;
    expect(fill.style.width).toBe('100%');
  });

  it('clamps negative pct to 0%', () => {
    const { container } = render(<ProgressBar pct={-5} colorClass="bg-emerald-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement;
    expect(fill.style.width).toBe('0%');
  });

  it('applies the color class to the fill, not the track', () => {
    const { container } = render(<ProgressBar pct={75} colorClass="bg-amber-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement;
    const track = container.querySelector('[data-testid="progress-track"]') as HTMLElement;
    expect(fill.className).toContain('bg-amber-500');
    expect(track.className).not.toContain('bg-amber-500');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- src/components/ui/ProgressBar.test.tsx`
Expected: FAIL — cannot resolve `./ProgressBar`.

- [ ] **Step 3: Implement the component**

Create `web/src/components/ui/ProgressBar.tsx`:

```typescript
/**
 * Phase 7: reusable color-coded progress bar.
 *
 * Purely presentational — caller decides pct and color. Used by the
 * OrgSettings Budget status card (large) and the Keys-table MTD column
 * (mini). The track is always muted; the fill carries the semantic color.
 */

export function ProgressBar({
  pct,
  colorClass,
  size = 'md',
}: {
  pct: number;
  colorClass: string;
  size?: 'sm' | 'md';
}) {
  const clamped = Math.max(0, Math.min(100, pct));
  const trackHeight = size === 'sm' ? 'h-1.5' : 'h-2.5';
  return (
    <div
      data-testid="progress-track"
      className={`w-full ${trackHeight} rounded-full bg-muted overflow-hidden`}
    >
      <div
        data-testid="progress-fill"
        className={`${colorClass} h-full rounded-full transition-all`}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- src/components/ui/ProgressBar.test.tsx`
Expected: PASS — 4 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/ui/ProgressBar.tsx web/src/components/ui/ProgressBar.test.tsx
git commit -m "phase7: add reusable ProgressBar component + unit tests"
```

---

## Task 11: Frontend — i18n keys for budget status + Keys column

Add the new translation keys for both `en` and `zh`.

**Files:**
- Modify: `web/src/i18n/en.json`
- Modify: `web/src/i18n/zh.json`

- [ ] **Step 1: Add keys to `en.json`**

Open `web/src/i18n/en.json`. Find the existing `"keys": { ... }` block and add `"mtdThisMonth"` inside `"table"` (around line 232, between `monthlyBudget` and `fallback`):

```json
      "monthlyBudget": "Monthly Budget",
      "mtdThisMonth": "MTD this month",
      "fallback": "Fallback",
```

Then find the `"orgSettings": { ... }` block and add a new `"budgetStatus"` subsection **as a sibling of `"defaults"`** (around line 991, immediately after the closing brace of `defaults`):

```json
    "defaults": {
      ... existing keys unchanged ...
    },
    "budgetStatus": {
      "title": "Budget status",
      "usedOf": "{{accrued}} used of {{limit}}",
      "unlimited": "Unlimited — no monthly cap",
      "overBudget": "Over budget by {{amount}}"
    },
```

> **For the implementer:** the four `budgetStatus.*` keys use `{{var}}` interpolation consumed by i18next. Don't touch the existing `"danger"` block — it stays where it is.

- [ ] **Step 2: Add keys to `zh.json`**

Open `web/src/i18n/zh.json`. Make the same structural additions (translate the values):

In `"keys"."table"`, add (between `monthlyBudget` and `fallback`):

```json
      "mtdThisMonth": "本月消费",
```

In `"orgSettings"`, immediately after `"defaults"`:

```json
    "budgetStatus": {
      "title": "预算状态",
      "usedOf": "已使用 {{accrued}} / {{limit}}",
      "unlimited": "无限制 — 无月度上限",
      "overBudget": "超出预算 {{amount}}"
    },
```

- [ ] **Step 3: Validate both JSON files parse**

Run:
```bash
source ~/.nvm/nvm.sh && cd web && node -e "JSON.parse(require('fs').readFileSync('src/i18n/en.json','utf8')); JSON.parse(require('fs').readFileSync('src/i18n/zh.json','utf8')); console.log('OK')"
```
Expected: prints `OK`. If it errors, fix the JSON syntax (usually a trailing comma).

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "phase7: i18n keys for budget status card + Keys MTD column"
```

---

## Task 12: Frontend — `<BudgetStatusSection />` on OrgSettings page

New subsection below `<DefaultsSection />`. Reads both `useGetOrgDefaults` (for the cap) and `useGetBudgetStatus` (for the accrued), composes them into a card with a progress bar.

**Files:**
- Modify: `web/src/pages/OrgSettings.tsx`

- [ ] **Step 1: Add the imports**

Open `web/src/pages/OrgSettings.tsx`. At the top of the file, extend the existing imports:

Add to the existing import from `'../hooks/useOrgDefaults'` (or as a new line right below it):

```typescript
import { useGetBudgetStatus } from '../hooks/useBudgetStatus';
```

Add to the existing imports near the top (with the other UI imports):

```typescript
import { ProgressBar } from '../components/ui/ProgressBar';
import { budgetBarColor, budgetUsedPct } from '../lib/budgetColor';
```

- [ ] **Step 2: Place the new subsection in the JSX**

Find the line `{/* Defaults section — admin can edit; member is read-only. */}` followed by `<DefaultsSection canEdit={canEdit} />` (around line 247-248). Insert a new line **immediately below** `<DefaultsSection />`:

```tsx
        {/* Defaults section — admin can edit; member is read-only. */}
        <DefaultsSection canEdit={canEdit} />

        {/* Phase 7: Budget status — read-only MTD card. Admin+ and member both see it. */}
        <BudgetStatusSection />
```

- [ ] **Step 3: Add the new component definition at the bottom of the file**

At the bottom of `web/src/pages/OrgSettings.tsx`, after the existing `function DefaultsSection(...)` definition (around line 463), add:

```tsx
function BudgetStatusSection() {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const { data: defaults } = useGetOrgDefaults();
  const { data: status, isLoading, isError } = useGetBudgetStatus();

  // Units conversion: budget from defaults (USD → units for parity comparison),
  // accrued from status (already units). Both share the 10^8 convention.
  const UNITS_PER_USD = 100_000_000;
  const budgetUsd = defaults?.default_budget_monthly_usd ?? null;
  const budgetUnits = budgetUsd !== null ? Math.round(budgetUsd * UNITS_PER_USD) : null;
  const accruedUnits = status?.accrued_units ?? 0;
  const usedPct = budgetUsedPct(accruedUnits, budgetUnits);

  // USD formatting — 2 decimals per spec.
  const accruedUsd = accruedUnits / UNITS_PER_USD;
  const accruedStr = `$${accruedUsd.toFixed(2)}`;
  const limitStr = budgetUsd !== null ? `$${budgetUsd.toFixed(2)}` : '';
  const overByUsd = usedPct !== null && usedPct > 100 ? accruedUsd - budgetUsd! : 0;

  return (
    <motion.section
      initial={reducedMotion ? false : { opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: EASE }}
      className="rounded-xl border border-base-300 bg-base-100 p-6 mt-6"
    >
      <div className="flex items-baseline justify-between mb-4">
        <h2 className="text-xl font-semibold">{t('orgSettings.budgetStatus.title')}</h2>
        {status?.month_bucket && (
          <span className="text-xs text-base-content/50 font-mono">{status.month_bucket}</span>
        )}
      </div>

      {isLoading ? (
        <div className="text-base-content/60">{t('orgSettings.defaults.loading')}</div>
      ) : isError ? (
        <div className="text-error">{t('orgSettings.defaults.loadError')}</div>
      ) : budgetUsd === null ? (
        <p className="text-sm text-base-content/70">{t('orgSettings.budgetStatus.unlimited')}</p>
      ) : (
        <div className="space-y-2">
          <p className="text-sm text-base-content/80">
            {t('orgSettings.budgetStatus.usedOf', { accrued: accruedStr, limit: limitStr })}
          </p>
          <ProgressBar pct={usedPct ?? 0} colorClass={budgetBarColor(usedPct)} />
          <div className="flex items-center justify-between text-xs text-base-content/50">
            <span>
              {usedPct !== null ? `${usedPct.toFixed(1)}%` : '—'}
            </span>
            {overByUsd > 0 && (
              <span className="text-red-500">
                {t('orgSettings.budgetStatus.overBudget', {
                  amount: `$${overByUsd.toFixed(2)}`,
                })}
              </span>
            )}
          </div>
        </div>
      )}
    </motion.section>
  );
}
```

- [ ] **Step 4: Typecheck + run vitest**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit && npm test -- --run`
Expected: typecheck clean; existing tests still pass.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/OrgSettings.tsx
git commit -m "phase7: add BudgetStatusSection to OrgSettings page

Read-only card showing org-wide MTD vs default budget with color-coded
progress bar. Loads budget cap and accrued in parallel via React Query."
```

---

## Task 13: Frontend — MTD column on Keys table

New column in the existing `<table>` showing per-key USD + mini-bar + %.

**Files:**
- Modify: `web/src/pages/Keys.tsx`

- [ ] **Step 1: Add the imports**

Open `web/src/pages/Keys.tsx`. Extend the existing imports:

```typescript
import { ProgressBar } from '../components/ui/ProgressBar';
import { budgetBarColor, budgetUsedPct } from '../lib/budgetColor';
```

(`useState`, `useTranslation`, etc. are already imported — don't duplicate.)

- [ ] **Step 2: Add a new `<th>` for the MTD column**

Find the `<thead><tr>` block (around line 109-117). After the `monthlyBudget` `<th>` and before the `fallback` `<th>`, insert:

```tsx
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.mtdThisMonth')}</th>
```

So the surrounding lines become:

```tsx
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.monthlyBudget')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.mtdThisMonth')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.fallback')}</th>
```

- [ ] **Step 3: Add a new `<td>` for each row**

Find the existing `<td>` for `budget_monthly` (around line 138):

```tsx
                    <td className="font-mono text-sm text-base-content/55">{key.budget_monthly != null ? formatCurrency(key.budget_monthly, symbol, 2) : t('keys.unlimited')}</td>
```

Immediately **below** that `<td>`, add a new `<td>` for MTD:

```tsx
                    <td className="font-mono text-sm text-base-content/55">
                      {(() => {
                        const UNITS_PER_USD = 100_000_000;
                        const budgetUnits =
                          key.budget_monthly !== null
                            ? Math.round(key.budget_monthly * UNITS_PER_USD)
                            : null;
                        const pct = budgetUsedPct(key.mtd_units ?? 0, budgetUnits);
                        const accruedUsd = (key.mtd_units ?? 0) / UNITS_PER_USD;
                        return (
                          <div className="flex items-center gap-2 min-w-[140px]">
                            <span className="tabular-nums">
                              {formatCurrency(accruedUsd, symbol, 2)}
                            </span>
                            <div className="flex-1">
                              <ProgressBar pct={pct ?? 0} colorClass={budgetBarColor(pct)} size="sm" />
                            </div>
                            <span className="text-xs text-base-content/45 tabular-nums w-10 text-right">
                              {pct !== null ? `${Math.round(pct)}%` : '—'}
                            </span>
                          </div>
                        );
                      })()}
                    </td>
```

- [ ] **Step 4: Bump the empty-state colSpan**

Find the empty-state row (around line 145). The `colSpan={7}` needs to become `colSpan={8}` since we added a column:

```tsx
                    <td colSpan={8} className="text-center py-16">
```

- [ ] **Step 5: Typecheck + run vitest**

Run: `source ~/.nvm/nvm.sh && cd web && npx tsc --noEmit && npm test -- --run`
Expected: typecheck clean; existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/Keys.tsx
git commit -m "phase7: add MTD column to Keys table

Per-key current-month accrued spend with mini color-coded progress bar.
Reads the new mtd_units field added in Phase 7's extended keys endpoint."
```

---

## Task 14: E2E — `web/e2e/budget-status.spec.ts`

End-to-end test that exercises the full read path: login → fire a request → assert OrgSettings card and Keys MTD column render non-zero values. Uses the same graceful-degradation pattern as Phase 6's `budget-enforcement.spec.ts` (skip-notice if upstream is unreachable).

**Files:**
- Create: `web/e2e/budget-status.spec.ts`

- [ ] **Step 1: Write the e2e test**

Create `web/e2e/budget-status.spec.ts`:

```typescript
import { test, expect, request } from '@playwright/test';

/**
 * Phase 7 E2E: budget observability.
 *
 * Flow: login → set a small org default budget → create a key → fire one
 * request (allowed) → visit OrgSettings → assert Budget status card is
 * visible and shows non-zero accrued → visit Keys → assert the MTD column
 * shows non-zero for the created key → cleanup.
 *
 * Mirrors Phase 6's `budget-enforcement.spec.ts` graceful-degradation
 * pattern: if upstream is unreachable, cost is never recorded, both MTD
 * fields stay at 0, and the test logs a skip-notice instead of failing.
 */

const ADMIN_USER = process.env.E2E_ADMIN_USER ?? 'admin';
const ADMIN_PASS = process.env.E2E_ADMIN_PASS ?? 'admin123456';
const RUN_TAG = process.env.E2E_RUN_TAG ?? String(Date.now());

const BACKEND = 'http://localhost:8080';
const DEV_SERVER = 'http://localhost:5173';

test('budget-status renders accrued in OrgSettings + Keys table', async ({ browser }) => {
  const context = await browser.newContext();
  const page = await context.newPage();

  // --- 1. UI login so localStorage is seeded. ---
  await page.goto(`${DEV_SERVER}/login`);
  await page.getByPlaceholder('Username').fill(ADMIN_USER);
  await page.getByPlaceholder('Password').fill(ADMIN_PASS);
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL('**/dashboard');

  const token = await page.evaluate(
    () => localStorage.getItem('llm_gateway_admin_token') ?? null,
  );
  expect(token).toBeTruthy();
  const slug = (await page.url()).match(/\/([^/]+)\/dashboard$/)?.[1];
  expect(slug).toBeTruthy();

  // --- 2. Management API context. ---
  const apiContext = await request.newContext({
    baseURL: BACKEND,
    extraHTTPHeaders: { authorization: `Bearer ${token}` },
  });

  // Snapshot org defaults so `finally` can restore them.
  const beforeResp = await apiContext.get(`/api/v1/${slug}/defaults`);
  expect(beforeResp.ok()).toBeTruthy();
  const beforeDefaults = await beforeResp.json();

  let keyId: string | null = null;
  let costRecorded = false;

  try {
    // --- 3. Create a key with no per-key budget. ---
    const keyResp = await apiContext.post(`/api/v1/${slug}/keys`, {
      data: { name: `e2e-mtd-${RUN_TAG}`, rate_limit: null, budget_monthly: null },
    });
    expect(keyResp.ok()).toBeTruthy();
    const keyBody = await keyResp.json();
    expect(keyBody.key).toBeTruthy();
    const apiKey: string = keyBody.key;
    keyId = keyBody.id;

    // --- 4. Set a generous default budget so the request is allowed. ---
    const putResp = await apiContext.put(`/api/v1/${slug}/defaults`, {
      data: { default_rate_limit_rpm: null, default_budget_monthly_usd: 100.0 },
    });
    expect(putResp.ok()).toBeTruthy();

    // --- 5. Fire one request via the proxy path (API key auth). ---
    const proxyCtx = await request.newContext({
      baseURL: BACKEND,
      extraHTTPHeaders: { authorization: `Bearer ${apiKey}` },
    });
    const proxyResp = await proxyCtx.post('/v1/chat/completions', {
      data: { model: 'gpt-test', messages: [{ role: 'user', content: 'hi' }] },
    });
    // Don't assert success — upstream may be unreachable in CI.
    // Either way, give the backend a moment to record usage (async worker).
    if (proxyResp.ok()) {
      // Wait briefly for the async record_usage worker to flush.
      // If cost was recorded, budget-status should show non-zero.
      await page.waitForTimeout(1500);
      const statusResp = await apiContext.get(`/api/v1/${slug}/budget-status`);
      const statusBody = await statusResp.json();
      costRecorded = (statusBody.accrued_units ?? 0) > 0;
    }

    // --- 6. OrgSettings page: Budget status card must render. ---
    await page.goto(`${DEV_SERVER}/${slug}/settings`);
    await expect(page.getByText('Budget status').first()).toBeVisible();
    // The card shows "YYYY-MM" month_bucket somewhere — match the shape.
    await expect(page.locator('text=/^\\d{4}-\\d{2}$/').first()).toBeVisible();

    if (costRecorded) {
      // usedOf text contains a $ amount followed by "used of".
      await expect(page.locator('text=/\\$[\\d.]+ used of \\$[\\d.]+/')).toBeVisible();
    } else {
      console.log('[e2e budget-status] upstream may be unreachable; cost not recorded; skipping non-zero assertion');
    }

    // --- 7. Keys page: MTD column header must render. ---
    await page.goto(`${DEV_SERVER}/${slug}/keys`);
    await expect(page.getByRole('columnheader', { name: /MTD this month/i })).toBeVisible();
    // The created key row must be present.
    await expect(page.getByText(`e2e-mtd-${RUN_TAG}`)).toBeVisible();

    // --- 8. Cleanup the test key. ---
    await apiContext.delete(`/api/v1/${slug}/keys/${keyId}`);
    keyId = null;
  } finally {
    // --- 9. ALWAYS restore org defaults + delete key on failure. ---
    if (keyId) {
      await apiContext.delete(`/api/v1/${slug}/keys/${keyId}`).catch(() => {});
    }
    await apiContext.put(`/api/v1/${slug}/defaults`, {
      data: {
        default_rate_limit_rpm: beforeDefaults.default_rate_limit_rpm ?? null,
        default_budget_monthly_usd: beforeDefaults.default_budget_monthly_usd ?? null,
      },
    }).catch(() => {});
    await apiContext.dispose();
    await context.close();
  }
});
```

- [ ] **Step 2: Run the e2e (manual — requires backend + dev server running)**

The implementer should NOT block on this step. e2e in this project has historically required an admin user seeded in the dev DB (Phase 6 accepted this as `DONE_WITH_CONCERNS`). Document the skip in the commit message if the env can't run it. To attempt:

```bash
# Terminal 1: backend
cargo run &
# Terminal 2: dev server
source ~/.nvm/nvm.sh && cd web && npm run dev &
# Terminal 3: e2e
source ~/.nvm/nvm.sh && cd web && npm run test:e2e -- budget-status
```

Expected if env is set up: test passes. Expected if admin user isn't seeded: login step fails — log as DONE_WITH_CONCERNS.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/budget-status.spec.ts
git commit -m "phase7: e2e for budget-status UI

Graceful-degradation pattern: asserts card renders + month_bucket shows,
skips non-zero assertion if upstream is unreachable (no cost recorded)."
```

---

## Task 15: CHANGELOG entry

Add the Phase 7 entry under `[Unreleased] → Added`, immediately after the Phase 6 block.

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Locate the insertion point**

Open `CHANGELOG.md`. Find the `## [Unreleased]` section, then its `### Added` subsection. Locate the most recent Phase 6 block (search for `Phase 6 (budget enforcement)`).

- [ ] **Step 2: Add the Phase 7 block**

Immediately **after** the Phase 6 block, add:

```markdown
- **Phase 7 (budget observability):**
  - **New UI:** OrgSettings gets a "Budget status" subsection showing org MTD total (sum across all keys) against the org-default budget, with a color-coded progress bar (green <60%, yellow 60-80%, orange 80-100%, red >100%). The Keys table gets an "MTD this month" column showing per-key spend with the same color coding.
  - **New endpoint:** `GET /api/v1/{slug}/budget-status` returns `{ accrued_units, month_bucket }` (i64 subunits, UTC calendar month). Member-gated (parity with `GET /{slug}/defaults`).
  - **Extended endpoint:** `GET /api/v1/{slug}/keys` now includes `mtd_units: i64` per key. Additive, non-breaking — existing API consumers ignore the new field.
  - **New storage methods:** `Storage::get_org_month_to_date_spend(org_id)` and `Storage::list_keys_paginated_with_mtd(org_id, page, page_size)`. Both read the existing `budget_counters` table (from Phase 6) — no schema changes.
  - **No behavior change:** enforcement remains as shipped in Phase 6 (post-completion, fail-open on storage errors). This phase is purely read-side observability.
```

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "phase7: changelog entry for budget observability"
```

---

## Self-Review

(Completed inline before offering execution choice.)

**Spec coverage:**
- Decision "Both OrgSettings + Keys table" → Tasks 12 (OrgSettings card) + 13 (Keys column). ✓
- Decision "Org rollup only" → Task 1 `get_org_month_to_date_spend` returns one number, no per-key breakdown in OrgSettings. ✓
- Decision "Extend /keys + new /budget-status" → Tasks 3 (`/budget-status`), 4 (extend `KeyResponse`). ✓
- Decision "Color-coded progress bars" → Tasks 9 (`budgetColor.ts`), 10 (`ProgressBar.tsx`). ✓
- Decision "Read-time SUM over budget_counters" → Task 1 SQL uses `SUM(bc.accrued)`. ✓
- Decision "LEFT JOIN in existing query" → Task 2 SQL uses `LEFT JOIN budget_counters`. ✓
- Decision "USD, 2 decimal places" → Tasks 12 + 13 use `toFixed(2)` / `formatCurrency(..., 2)`. ✓
- Decision "UTC calendar month" → Tasks 1 + 2 use `chrono::Utc::now().format("%Y-%m")`; Task 3 derives `month_bucket` from `Utc::now()`. ✓
- API shape `/budget-status` → `{ accrued_units, month_bucket }`, no `budget` field (Task 3 + Task 6). ✓
- API shape extended `/keys` → `mtd_units: i64` additive (Task 4 + Task 7). ✓
- Auth: member+ can read (Task 3 docstring; Task 5 test #3 covers cross-org 403; test #4 covers no-bearer 401). ✓
- Storage unit tests (spec lists 6 tests; we cover all 6 across Tasks 1 + 2): empty org, sums across keys, excludes other months, no cross-org leak, list_keys includes mtd, list_keys zero when no spend (covered by `key-no-cost` assertion in Task 2 step 3). ✓
- API integration tests (spec lists 5 tests): accrued, zero-for-fresh-org, 403 non-member, 401 unauthenticated, list_keys includes mtd — all in Task 5. ✓
- Frontend unit: `budgetColor.test.ts` (Task 9) + `ProgressBar.test.tsx` (Task 10). ✓
- E2E `web/e2e/budget-status.spec.ts` (Task 14). ✓
- CHANGELOG entry (Task 15). ✓
- "No upgrade note" — correct; additive + read-only + no schema change. Task 15 changelog text reflects this. ✓

**Placeholder scan:** No `TBD`, `TODO`, `fill in details`, `similar to Task N`, or stubbed code blocks. The one place that says "for the implementer" (Task 2 Step 6, Task 11 Step 1) calls out a concrete verification step — read the source of truth and adjust field list — not a placeholder.

**Type consistency:**
- `get_org_month_to_date_spend(org_id) -> Result<i64, DbErr>` — declared Task 1 Step 3, used Task 1 Step 4 and Task 3 Step 1. ✓
- `list_keys_paginated_with_mtd(org_id, page, page_size) -> Result<PaginatedResponse<ApiKeyWithMtd>, DbErr>` — declared Task 2 Step 5, used Task 2 Step 6 and Task 4 Step 2. ✓
- `ApiKeyWithMtd { key: ApiKey, mtd_units: i64 }` — declared Task 2 Step 1, consumed Task 2 Step 6 + Task 4 Step 1. ✓
- `BudgetStatusResponse { accrued_units: i64, month_bucket: String }` — declared Task 3 Step 1, serialized via JSON; frontend type `BudgetStatus { accrued_units: number; month_bucket: string }` (Task 6) mirrors the shape. ✓
- `KeyResponse.mtd_units: i64` (Rust, Task 4) ↔ `ApiKey.mtd_units: number` (TS, Task 7). ✓
- `budgetBarColor(usedPct: number | null): string` + `budgetUsedPct(accruedUnits, budgetUnits: number | null): number | null` — declared Task 9 Step 3, consumed Task 12 Step 3 + Task 13 Step 3. ✓
- `ProgressBar` props `{ pct, colorClass, size? }` — declared Task 10 Step 3, consumed with the same prop names in Tasks 12 + 13. ✓
- i18n key names: `orgSettings.budgetStatus.{title,usedOf,unlimited,overBudget}` and `keys.table.mtdThisMonth` — added Task 11, consumed Tasks 12 + 13. ✓

No type drift found.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-07-10-saas-phase7-budget-observability.md`.** Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks (spec compliance first, code quality second), fast iteration
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
