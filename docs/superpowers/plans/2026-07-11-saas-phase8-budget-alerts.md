# Phase 8 — Budget Alerts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Send exactly one alert email per `(org, calendar month, threshold)` when an org's month-to-date spend crosses 80% or 100% of its `default_budget_monthly_usd`. Recipients: every admin and owner of the org with a verified email address. Detection is inline in the existing `usage-worker`, fired after `record_usage` succeeds.

**Architecture:** One new Postgres table (`budget_alerts_sent` — a dedup ledger keyed on `(org_id, month_bucket, threshold)` with a two-phase `sent_at` column that doubles as both claim marker and success marker). Four new `Storage` trait methods (snapshot fetch, atomic claim, mark-sent, recipient list). One new email ctx (`BudgetAlertCtx`) + `render_budget_alert` on Phase 4's `TemplateRegistry`. The `usage-worker` crate gets refactored to `lib.rs + main.rs` so its logic is integration-testable, gains a new `budget_alerts.rs` module with a `check_budget_alerts` orchestrator and an integer-safe `passes_threshold` helper, and is wired into the worker's main loop after `record_usage`. No new HTTP endpoints. No new NATS streams. No frontend changes.

**Tech Stack:** Rust (sqlx Postgres, async-trait, chrono for UTC month bucketing, handlebars for templates); the existing `Mailer` trait + `SmtpMailer`/`FileMailer`/`NoopMailer` from `llm-gateway-email`.

**Spec:** `docs/superpowers/specs/2026-07-11-saas-phase8-budget-alerts-design.md`

---

## File Structure

**Backend (Rust):**

- `crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.sql` — **new file**: `budget_alerts_sent` dedup table
- `crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.down.sql` — **new file**: `DROP TABLE IF EXISTS budget_alerts_sent`
- `crates/storage/src/types.rs` — add `BudgetAlertSnapshot` struct
- `crates/storage/src/lib.rs` — add four trait methods: `get_org_budget_for_alerts`, `try_claim_budget_alert`, `mark_budget_alert_sent`, `list_org_admin_emails`
- `crates/storage/src/postgres.rs` — impl the four methods + 8 new tests in the existing `invitation_tests` mod (sibling to the Phase 6/7 budget tests)
- `crates/email/src/templates.rs` — add `BudgetAlertCtx` struct + `render_budget_alert` method on `TemplateRegistry`; register two new templates in `load`; add a unit test
- `crates/email/templates/budget_alert.txt.hbs` — **new file**: plain-text email template
- `crates/email/templates/budget_alert.html.hbs` — **new file**: HTML email template
- `crates/usage-worker/src/lib.rs` — **new file**: extracts the worker loop from `main.rs` so it's testable
- `crates/usage-worker/src/main.rs` — thin wrapper: load config, construct deps (including new `Mailer` + `TemplateRegistry`), call `run_usage_worker`
- `crates/usage-worker/src/budget_alerts.rs` — **new file**: `check_budget_alerts` orchestrator + `passes_threshold` / `format_usd` helpers + co-located unit tests
- `crates/usage-worker/tests/budget_alert_flow.rs` — **new file**: 5 integration-test scenarios using a `RecordingMailer` test double
- `crates/usage-worker/Cargo.toml` — add `llm-gateway-email` dep; declare both `lib` and `bin` targets

**Docs:**

- `CHANGELOG.md` — Phase 8 entry under `## [Unreleased] → Added`, immediately after the Phase 7 block

---

## Task 1: Migration — `budget_alerts_sent` dedup table

Pure dedup ledger. One row per `(org, month, threshold)` that has been claimed for alerting. Two-phase `sent_at` column plays both "claim" (insert) and "success marker" (update) roles.

**Files:**
- Create: `crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.sql`
- Create: `crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.down.sql`

- [ ] **Step 1: Write the up migration**

Create `crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.sql`:

```sql
-- Phase 8: dedup ledger for budget alerts.
-- One row per (org, month, threshold) that has been claimed for alerting.
-- sent_at plays two roles:
--   1. After INSERT (claim): NULL means "claimed but not yet sent" — the
--      caller owes a send + mark_budget_alert_sent.
--   2. After UPDATE (mark sent): NOT NULL means "successfully sent" — no
--      future caller can re-claim this slot.
-- This is what lets a failed send be retried on the next threshold-crossing
-- request (the row stays with sent_at = NULL until someone succeeds).
CREATE TABLE IF NOT EXISTS budget_alerts_sent (
    org_id        TEXT        NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    month_bucket  TEXT        NOT NULL,            -- 'YYYY-MM' UTC calendar month (matches Phase 6 budget_counters)
    threshold     SMALLINT    NOT NULL,            -- 80 or 100 (per the fixed-thresholds decision)
    claimed_at    TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    sent_at       TIMESTAMP WITH TIME ZONE,        -- NULL until email send succeeds
    PRIMARY KEY (org_id, month_bucket, threshold)
);

-- Speeds the "any unsent claim in this org/month?" re-arm check performed
-- by try_claim_budget_alert's UPDATE ... WHERE sent_at IS NULL RETURNING.
CREATE INDEX IF NOT EXISTS budget_alerts_sent_unsent_idx
    ON budget_alerts_sent (org_id, month_bucket)
    WHERE sent_at IS NULL;
```

- [ ] **Step 2: Write the down migration**

Create `crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.down.sql`:

```sql
DROP TABLE IF EXISTS budget_alerts_sent;
```

- [ ] **Step 3: Run the migration to verify it applies cleanly**

Run: `cargo run -p llm-gateway-gateway -- migrate 2>&1 | tail -20 || cargo test -p llm-gateway-storage --no-run 2>&1 | tail -5`

Expected: migration applies with no SQL errors. (If no `migrate` subcommand exists, the test run in Step 3 of later tasks exercises the migrator automatically via `#[sqlx::test(migrator = "crate::MIGRATOR")]`.)

- [ ] **Step 4: Commit**

```bash
git -C /workspace/llm-gateway add crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.sql crates/storage/migrations/postgres/20260803000000_budget_alerts_sent.down.sql
git -C /workspace/llm-gateway commit -m "feat(phase8): add budget_alerts_sent dedup migration"
```

---

## Task 2: Storage type — `BudgetAlertSnapshot`

The struct returned by `get_org_budget_for_alerts` — fetches budget + accrued + org name/slug in one round-trip so the worker can't observe a torn state.

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add the struct**

Open `crates/storage/src/types.rs`. Find the existing `OrgDefaults` struct (search for `pub struct OrgDefaults`). Add this new struct immediately **below** the `OrgDefaults` definition:

```rust
/// Phase 8: snapshot for budget alert evaluation, fetched in a single
/// round-trip so the worker can't observe a torn state between budget and
/// accrued. `budget_units` is `None` when the org has no default budget set
/// (unlimited) — callers must skip alerting in that case.
#[derive(Debug, Clone)]
pub struct BudgetAlertSnapshot {
    /// 10^8 subunits per USD. None = unlimited (no `default_budget_monthly_usd`).
    pub budget_units: Option<i64>,
    /// Current MTD spend across all the org's keys, same units.
    pub accrued_units: i64,
    /// Org display name — used in the email subject and body.
    pub org_name: String,
    /// Org slug — used to build the dashboard URL.
    pub org_slug: String,
}
```

- [ ] **Step 2: Verify the crate compiles**

Run: `cargo build -p llm-gateway-storage 2>&1 | tail -10`

Expected: clean build (the new struct is unused at this point — that's fine, Rust doesn't warn on unused pub items).

- [ ] **Step 3: Commit**

```bash
git -C /workspace/llm-gateway add crates/storage/src/types.rs
git -C /workspace/llm-gateway commit -m "feat(phase8): add BudgetAlertSnapshot storage type"
```

---

## Task 3: Storage — `get_org_budget_for_alerts` trait method + impl + tests

Single round-trip query that joins `org_settings` + `orgs` + `budget_counters` to produce a `BudgetAlertSnapshot`. The CTE form ensures budget and accrued share a snapshot timestamp (no torn read).

**Files:**
- Modify: `crates/storage/src/lib.rs` (add trait method declaration)
- Modify: `crates/storage/src/postgres.rs` (add impl + tests)

- [ ] **Step 1: Add the trait method declaration**

Open `crates/storage/src/lib.rs`. Find the existing `set_org_defaults` method (it's the last method in the `Storage` trait, around line 369). Add the new declaration **above** the closing `}` of the trait:

```rust
    /// Phase 8: snapshot for budget alert evaluation. Fetches the org's
    /// default budget (subunits, None if unlimited), current MTD spend, plus
    /// name+slug for the email template — in one round-trip so the worker
    /// can't observe a torn state.
    async fn get_org_budget_for_alerts(
        &self,
        org_id: &str,
    ) -> Result<crate::types::BudgetAlertSnapshot, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 2: Write the failing storage tests first**

Open `crates/storage/src/postgres.rs`. Find the Phase 6 test mod — search for `// ---- Phase 6: budget_counters ----`. The new tests go in the **same `invitation_tests` mod**, as siblings of the Phase 6 budget tests. Find the test `budget_counters_concurrent_inserts` and add the new tests immediately **after the closing `}` of the next test** (i.e., at the end of the budget-related test cluster, before any unrelated test or the mod's closing brace).

To locate the insertion point precisely: search for `budget_counters_concurrent_inserts` — the test after it (or the closing brace of the mod if it's the last test) is where you insert. Add these three tests:

```rust
    // ---- Phase 8: budget_alerts_sent support ----

    /// Helper: set (or clear) the org-default budget directly via SQL. Used by
    /// Phase 8 storage tests so we can exercise the `None`/`Some($0)`/`Some($50)`
    /// branches without going through the typed `set_org_defaults` facade.
    async fn set_org_default_budget(storage: &PostgresStorage, org_id: &str, budget_usd: Option<&str>) {
        match budget_usd {
            Some(v) => {
                sqlx::query(
                    "INSERT INTO org_settings (org_id, key, value) VALUES ($1, 'default_budget_monthly_usd', $2)
                     ON CONFLICT (org_id, key) DO UPDATE SET value = EXCLUDED.value",
                )
                .bind(org_id)
                .bind(v)
                .execute(&storage.pool)
                .await
                .expect("set_org_default_budget");
            }
            None => {
                sqlx::query("DELETE FROM org_settings WHERE org_id = $1 AND key = 'default_budget_monthly_usd'")
                    .bind(org_id)
                    .execute(&storage.pool)
                    .await
                    .expect("set_org_default_budget (delete)");
            }
        }
    }

    /// No budget set → snapshot.budget_units is None (unlimited signal).
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_budget_for_alerts_returns_none_for_no_budget(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-alert-no-budget", "NoBudget Org").await;
        // No default_budget_monthly_usd row set.
        let snap = storage.get_org_budget_for_alerts(&org.id).await.expect("snapshot");
        assert!(snap.budget_units.is_none(), "no budget set → None");
        assert_eq!(snap.accrued_units, 0, "no spend → 0");
        assert_eq!(snap.org_name, "NoBudget Org");
        assert_eq!(snap.org_slug, "org-alert-no-budget");
    }

    /// $50 budget + $20 spend → Some(5_000_000_000) budget, 2_000_000_000 accrued.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_budget_for_alerts_returns_budget_and_accrued(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-alert-set", "SetBudget Org").await;
        let key_id = make_test_key_for_budget(&storage, &org.id, "key-alert-set").await;

        // Set $50 budget (stored as the string "50" — same convention as Phase 5/7).
        set_org_default_budget(&storage, &org.id, Some("50")).await;

        // Record $20 spend this month.
        let twenty_usd = crate::money::usd_to_units(20.0);
        storage
            .record_usage(&org.id, &mk_usage(&org.id, &key_id, twenty_usd, chrono::Utc::now()))
            .await
            .expect("record_usage");

        let snap = storage.get_org_budget_for_alerts(&org.id).await.expect("snapshot");
        assert_eq!(snap.budget_units, Some(5_000_000_000), "$50 budget in subunits");
        assert_eq!(snap.accrued_units, 2_000_000_000, "$20 accrued in subunits");
        assert_eq!(snap.org_name, "SetBudget Org");
        assert_eq!(snap.org_slug, "org-alert-set");
    }

    /// Budget set but zero usage → accrued_units = 0 (no NULL handling issues).
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn get_org_budget_for_alerts_zero_accrued_when_no_spend(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-alert-empty", "EmptySpend Org").await;
        set_org_default_budget(&storage, &org.id, Some("10")).await;
        // No usage recorded.
        let snap = storage.get_org_budget_for_alerts(&org.id).await.expect("snapshot");
        assert_eq!(snap.budget_units, Some(1_000_000_000));
        assert_eq!(snap.accrued_units, 0, "no spend → 0 (not NULL)");
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p llm-gateway-storage --no-default-features get_org_budget_for_alerts 2>&1 | tail -15`

Expected: compile error — `get_org_budget_for_alerts` is not yet implemented on `PostgresStorage`. (The trait method exists from Step 1, but no impl.)

- [ ] **Step 4: Write the impl**

Open `crates/storage/src/postgres.rs`. Find the existing `get_org_month_to_date_spend` impl (search for `async fn get_org_month_to_date_spend`). Add the new impl immediately **below** the closing `}` of `get_org_month_to_date_spend`:

```rust
    /// Phase 8: snapshot for budget alert evaluation. CTE form so the budget
    /// and accrued share a snapshot timestamp — no torn read.
    async fn get_org_budget_for_alerts(
        &self,
        org_id: &str,
    ) -> Result<crate::types::BudgetAlertSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        let row: BudgetAlertRow = sqlx::query_as::<_, BudgetAlertRow>(
            r#"
            WITH budget AS (
                SELECT NULLIF(value, '')::numeric AS budget_usd
                FROM org_settings
                WHERE org_id = $1 AND key = 'default_budget_monthly_usd'
            ),
            org AS (
                SELECT name, slug FROM orgs WHERE id = $1
            )
            SELECT
                o.name        AS org_name,
                o.slug        AS org_slug,
                CASE
                    WHEN b.budget_usd IS NULL THEN NULL
                    ELSE CAST(b.budget_usd * 100000000 AS BIGINT)
                END           AS budget_units,
                COALESCE((
                    SELECT CAST(SUM(bc.accrued) AS BIGINT)
                    FROM budget_counters bc
                    JOIN api_keys ak ON ak.id = bc.key_id
                    WHERE ak.org_id = $1
                      AND bc.month_bucket = to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM')
                ), 0)         AS accrued_units
            FROM org o
            LEFT JOIN budget b ON TRUE
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::types::BudgetAlertSnapshot {
            budget_units: row.budget_units,
            accrued_units: row.accrued_units,
            org_name: row.org_name,
            org_slug: row.org_slug,
        })
    }
```

Then add the `BudgetAlertRow` helper struct at the top of the file, **next to the other `*Row` helpers**. Search for `struct PgKeyWithMtdRow` (the Phase 7 row helper) and add this just below it:

```rust
/// Phase 8: row shape for the `get_org_budget_for_alerts` CTE. budget_units
/// is nullable to round-trip the SQL NULL → Rust None encoding.
#[derive(sqlx::FromRow)]
struct BudgetAlertRow {
    org_name: String,
    org_slug: String,
    budget_units: Option<i64>,
    accrued_units: i64,
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p llm-gateway-storage --no-default-features get_org_budget_for_alerts 2>&1 | tail -10`

Expected: `3 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git -C /workspace/llm-gateway add crates/storage/src/lib.rs crates/storage/src/postgres.rs
git -C /workspace/llm-gateway commit -m "feat(phase8): storage get_org_budget_for_alerts + tests"
```

---

## Task 4: Storage — `list_org_admin_emails` trait method + impl + tests

Verified-email admins+owners of the org. Mirrors Phase 4's verified-email gate.

**Files:**
- Modify: `crates/storage/src/lib.rs` (add trait method declaration)
- Modify: `crates/storage/src/postgres.rs` (add impl + tests)

- [ ] **Step 1: Add the trait method declaration**

Open `crates/storage/src/lib.rs`. Add this declaration **immediately below** the `get_org_budget_for_alerts` declaration you added in Task 3:

```rust
    /// Phase 8: verified emails of all admins+owners in the org. Skips members
    /// with no email or unverified email (matches Phase 4's verified-email
    /// gate). Used to address budget-alert emails.
    async fn list_org_admin_emails(
        &self,
        org_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 2: Write the failing storage tests first**

Open `crates/storage/src/postgres.rs`. Add these two tests **below** the `get_org_budget_for_alerts_zero_accrued_when_no_spend` test from Task 3 (still inside the same Phase 8 cluster):

```rust
    /// Helper: create a user with a specific email + verification state, then
    /// add them to the org as the given role. Used by Phase 8 recipient tests.
    async fn make_test_user_with_email(
        storage: &PostgresStorage,
        username: &str,
        email: Option<&str>,
        verified: bool,
    ) -> crate::types::User {
        let now = chrono::Utc::now();
        let user = crate::types::User {
            id: username.to_string(),
            username: username.to_string(),
            password: "x".to_string(),
            platform_role: None,
            current_org_id: None,
            enabled: true,
            refresh_token: None,
            created_at: now,
            updated_at: now,
            email: email.map(|s| s.to_string()),
            email_verified_at: verified.then(now),
            requires_email_verification: false,
            password_changed_at: now,
        };
        storage.create_user(&user).await.expect("create_user_with_email")
    }

    /// Helper: insert a members row directly via SQL. Used because the typed
    /// API goes through upsert_member which assumes an existing user.
    async fn add_test_member(storage: &PostgresStorage, user_id: &str, org_id: &str, role: &str) {
        sqlx::query(
            "INSERT INTO members (user_id, org_id, role, created_by)
             VALUES ($1, $2, $3, $1)
             ON CONFLICT (user_id, org_id) DO UPDATE SET role = EXCLUDED.role",
        )
        .bind(user_id)
        .bind(org_id)
        .bind(role)
        .execute(&storage.pool)
        .await
        .expect("add_test_member");
    }

    /// Returns verified emails of admins+owners only; members and unverified
    /// users are excluded.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_org_admin_emails_returns_admins_and_owners(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-recipients", "Recipients Org").await;
        // Owner created by make_test_org has no email — excluded.
        // Add: verified admin, verified owner, member, unverified admin.
        let admin = make_test_user_with_email(&storage, "admin1", Some("admin1@example.com"), true).await;
        let owner = make_test_user_with_email(&storage, "owner2", Some("owner2@example.com"), true).await;
        let member = make_test_user_with_email(&storage, "member1", Some("member1@example.com"), true).await;
        let unverified = make_test_user_with_email(&storage, "admin2", Some("admin2@example.com"), false).await;

        add_test_member(&storage, &admin.id, &org.id, "admin").await;
        add_test_member(&storage, &owner.id, &org.id, "owner").await;
        add_test_member(&storage, &member.id, &org.id, "member").await;
        add_test_member(&storage, &unverified.id, &org.id, "admin").await;

        let mut emails = storage.list_org_admin_emails(&org.id).await.expect("list");
        emails.sort();
        assert_eq!(emails, vec!["admin1@example.com".to_string(), "owner2@example.com".to_string()],
            "only verified admins+owners");
    }

    /// Org with no eligible recipients → empty vec, not an error.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_org_admin_emails_empty_for_org_with_no_admins(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-no-recipients", "NoRecipients Org").await;
        // Owner has no email; no other members.
        let emails = storage.list_org_admin_emails(&org.id).await.expect("list");
        assert!(emails.is_empty(), "no eligible recipients → empty vec");
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p llm-gateway-storage --no-default-features list_org_admin_emails 2>&1 | tail -10`

Expected: compile error — `list_org_admin_emails` not yet implemented.

- [ ] **Step 4: Write the impl**

Open `crates/storage/src/postgres.rs`. Add this impl **immediately below** the `get_org_budget_for_alerts` impl from Task 3:

```rust
    /// Phase 8: verified emails of all admins+owners. Mirrors Phase 4's
    /// verified-email gate (email IS NOT NULL AND email_verified_at IS NOT NULL).
    async fn list_org_admin_emails(
        &self,
        org_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT u.email
            FROM members m
            JOIN users u ON u.id = m.user_id
            WHERE m.org_id = $1
              AND m.role IN ('admin', 'owner')
              AND u.email IS NOT NULL
              AND u.email_verified_at IS NOT NULL
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(e,)| e).collect())
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p llm-gateway-storage --no-default-features list_org_admin_emails 2>&1 | tail -10`

Expected: `2 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git -C /workspace/llm-gateway add crates/storage/src/lib.rs crates/storage/src/postgres.rs
git -C /workspace/llm-gateway commit -m "feat(phase8): storage list_org_admin_emails + tests"
```

---

## Task 5: Storage — `try_claim_budget_alert` + `mark_budget_alert_sent` trait methods + impls + tests

The two-phase claim mechanism. INSERT-on-conflict-do-nothing + UPDATE-where-sent_at-IS-NULL-returning is what guarantees exactly-one-wins under concurrency and re-arms failed sends.

**Files:**
- Modify: `crates/storage/src/lib.rs` (add trait method declarations)
- Modify: `crates/storage/src/postgres.rs` (add impls + tests)

- [ ] **Step 1: Add the trait method declarations**

Open `crates/storage/src/lib.rs`. Add these two declarations **immediately below** the `list_org_admin_emails` declaration from Task 4:

```rust
    /// Phase 8: atomically claim the right to send an alert for (org, month, threshold).
    /// Implementation does INSERT ... ON CONFLICT DO NOTHING inside a transaction
    /// that also re-arms any pre-existing claim whose sent_at IS NULL (failed send
    /// on a prior attempt). Returns `Some(())` if this caller owns the claim and
    /// must send + mark sent; `None` if another caller already owns a sent claim.
    async fn try_claim_budget_alert(
        &self,
        org_id: &str,
        month_bucket: &str,
        threshold: i16,
    ) -> Result<Option<()>, Box<dyn std::error::Error + Send + Sync>>;

    /// Phase 8: mark a claimed alert as successfully sent. Called after the
    /// email send returns Ok. Idempotent — no-op if the row doesn't exist or
    /// is already marked sent.
    async fn mark_budget_alert_sent(
        &self,
        org_id: &str,
        month_bucket: &str,
        threshold: i16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 2: Write the failing storage tests first**

Open `crates/storage/src/postgres.rs`. Add these three tests **below** the `list_org_admin_emails_empty_for_org_with_no_admins` test from Task 4 (still inside the Phase 8 cluster):

```rust
    /// Two-phase claim: first caller gets Some, marks sent, second caller gets None.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn try_claim_budget_alert_first_caller_wins(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-claim-race", "ClaimRace Org").await;
        let month = "2026-07";

        let first = storage.try_claim_budget_alert(&org.id, month, 80).await.expect("claim #1");
        assert!(first.is_some(), "first caller must win the claim");

        // First caller marks sent — releases no lock, but signals "done".
        storage.mark_budget_alert_sent(&org.id, month, 80).await.expect("mark sent");

        let second = storage.try_claim_budget_alert(&org.id, month, 80).await.expect("claim #2");
        assert!(second.is_none(), "second caller after sent_at set must get None");
    }

    /// Re-arm: claim but don't mark sent → next try_claim returns Some again.
    /// This is the retry mechanism for failed sends.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn try_claim_budget_alert_retries_after_failed_send(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-claim-retry", "ClaimRetry Org").await;
        let month = "2026-07";

        // First claim — pretend the worker crashed before marking sent.
        let first = storage.try_claim_budget_alert(&org.id, month, 80).await.expect("claim #1");
        assert!(first.is_some());

        // Second claim (no mark_sent in between) — must re-arm to Some.
        let second = storage.try_claim_budget_alert(&org.id, month, 80).await.expect("claim #2");
        assert!(second.is_some(), "unsent claim must be re-armed to a new caller");

        // After actual send + mark, third caller gets None.
        storage.mark_budget_alert_sent(&org.id, month, 80).await.expect("mark sent");
        let third = storage.try_claim_budget_alert(&org.id, month, 80).await.expect("claim #3");
        assert!(third.is_none(), "after sent, no more re-arm");
    }

    /// mark_budget_alert_sent on a non-existent row is a no-op (idempotent).
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn mark_budget_alert_sent_is_idempotent(pool: sqlx::PgPool) {
        let storage = crate::postgres::PostgresStorage::from_pool(pool);
        let org = make_test_org(&storage, "org-mark-idempotent", "MarkIdempotent Org").await;
        let month = "2026-07";
        // No prior claim. Mark should succeed without error.
        storage.mark_budget_alert_sent(&org.id, month, 80).await.expect("mark on nonexistent row");
        // Still no error on a second call.
        storage.mark_budget_alert_sent(&org.id, month, 80).await.expect("mark again");

        // Sanity: the row was never created (idempotent mark, not insert).
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM budget_alerts_sent WHERE org_id = $1")
            .bind(&org.id)
            .fetch_one(&storage.pool)
            .await
            .expect("count");
        assert_eq!(count, 0, "idempotent mark must not create a row");
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p llm-gateway-storage --no-default-features try_claim_budget_alert 2>&1 | tail -10`

Expected: compile error — `try_claim_budget_alert` and `mark_budget_alert_sent` not yet implemented.

- [ ] **Step 4: Write the impls**

Open `crates/storage/src/postgres.rs`. Add these two impls **immediately below** the `list_org_admin_emails` impl from Task 4:

```rust
    /// Phase 8: atomic two-phase claim. INSERT-on-conflict-do-nothing creates
    /// the row if absent; UPDATE-where-sent_at-IS-NULL-returning re-arms any
    /// existing unsent claim to this caller. Empty RETURNING → another caller
    /// already sent → None.
    async fn try_claim_budget_alert(
        &self,
        org_id: &str,
        month_bucket: &str,
        threshold: i16,
    ) -> Result<Option<()>, Box<dyn std::error::Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        // Phase 1: try to insert. ON CONFLICT DO NOTHING so a no-op if the row exists.
        sqlx::query(
            "INSERT INTO budget_alerts_sent (org_id, month_bucket, threshold)
             VALUES ($1, $2, $3)
             ON CONFLICT (org_id, month_bucket, threshold) DO NOTHING",
        )
        .bind(org_id)
        .bind(month_bucket)
        .bind(threshold)
        .execute(&mut *tx)
        .await?;

        // Phase 2: re-arm any unsent claim to us. If the row's sent_at is still
        // NULL (either we just inserted it, or a prior worker failed to send),
        // UPDATE returns it → we own the send. If sent_at is NOT NULL (someone
        // already sent), UPDATE matches nothing → None.
        let claimed: Option<(String,)> = sqlx::query_as(
            "UPDATE budget_alerts_sent
                SET claimed_at = NOW()
              WHERE org_id = $1 AND month_bucket = $2 AND threshold = $3
                AND sent_at IS NULL
             RETURNING org_id",
        )
        .bind(org_id)
        .bind(month_bucket)
        .bind(threshold)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(claimed.map(|_| ()))
    }

    /// Phase 8: mark a claimed alert as sent. Idempotent — no-op if the row
    /// doesn't exist (never claimed) or is already marked sent.
    async fn mark_budget_alert_sent(
        &self,
        org_id: &str,
        month_bucket: &str,
        threshold: i16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "UPDATE budget_alerts_sent
                SET sent_at = NOW()
              WHERE org_id = $1 AND month_bucket = $2 AND threshold = $3
                AND sent_at IS NULL",
        )
        .bind(org_id)
        .bind(month_bucket)
        .bind(threshold)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p llm-gateway-storage --no-default-features try_claim_budget_alert 2>&1 | tail -10 && cargo test -p llm-gateway-storage --no-default-features mark_budget_alert_sent 2>&1 | tail -10`

Expected: `3 passed; 0 failed` for the first command, `1 passed; 0 failed` for the second (the `try_claim_budget_alert_retries_after_failed_send` test exercises mark as well).

- [ ] **Step 6: Run the full storage test suite to catch regressions**

Run: `cargo test -p llm-gateway-storage --no-default-features 2>&1 | tail -15`

Expected: all tests pass (Phase 8 additions don't conflict with Phase 6/7 budget tests).

- [ ] **Step 7: Commit**

```bash
git -C /workspace/llm-gateway add crates/storage/src/lib.rs crates/storage/src/postgres.rs
git -C /workspace/llm-gateway commit -m "feat(phase8): storage try_claim_budget_alert + mark_budget_alert_sent + tests"
```

---

## Task 6: Email — `BudgetAlertCtx` struct + `render_budget_alert` method + unit test

Extend Phase 4's `TemplateRegistry` with a fourth typed ctx + render method. Mirrors the `VerificationCtx`/`InvitationCtx`/`PasswordResetCtx` pattern: the ctx struct is `Serialize`, the render method stamps `to: ctx.recipient_email` directly into the `EmailMessage`, and the registry handles template loading via `include_str!`.

**Files:**
- Modify: `crates/email/src/templates.rs`

- [ ] **Step 1: Write the failing unit test first**

Open `crates/email/src/templates.rs`. Find the existing `renders_password_reset` test in the `#[cfg(test)] mod tests` block. Add this new test immediately **below** it (still inside the `tests` mod):

```rust
    #[test]
    fn renders_budget_alert_at_threshold() {
        let r = registry();
        let ctx = BudgetAlertCtx {
            org_name: "Acme Corp".into(),
            org_slug: "acme-corp".into(),
            recipient_email: "oncall@acme.example".into(),
            accrued_usd: "$40.00".into(),
            budget_usd: "$50.00".into(),
            percent: 80,
            month_bucket: "2026-07".into(),
            dashboard_url: "https://gateway.example.com/acme-corp/settings".into(),
            public_base_url: "https://gateway.example.com".into(),
        };
        let msg = r.render_budget_alert(ctx).unwrap();
        assert_eq!(msg.to, "oncall@acme.example");
        assert_eq!(msg.subject, "Test: Acme Corp budget at 80%");
        assert!(msg.text_body.contains("Acme Corp"), "org_name missing from text_body");
        assert!(msg.text_body.contains("$40.00"), "accrued_usd missing from text_body");
        assert!(msg.text_body.contains("$50.00"), "budget_usd missing from text_body");
        assert!(msg.text_body.contains("80%"), "percent missing from text_body");
        assert!(msg.text_body.contains("2026-07"), "month_bucket missing from text_body");
        assert!(msg.text_body.contains("/acme-corp/settings"), "dashboard_url missing from text_body");
        let html = msg.html_body.expect("html body");
        assert!(html.contains("Acme Corp"));
        assert!(html.contains("$40.00"));
        assert!(html.contains("80%"));
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p llm-gateway-email renders_budget_alert_at_threshold 2>&1 | tail -10`

Expected: compile error — `BudgetAlertCtx` not yet defined; `render_budget_alert` method not yet on `TemplateRegistry`.

- [ ] **Step 3: Add the ctx struct**

Open `crates/email/src/templates.rs`. Find the existing `PasswordResetCtx` struct definition. Add the new struct immediately **below** it:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct BudgetAlertCtx {
    pub org_name: String,
    pub org_slug: String,
    pub recipient_email: String,
    /// Pre-formatted dollar string, e.g. "$40.00". Pre-computed by the caller
    /// to keep integer/float formatting out of the template.
    pub accrued_usd: String,
    /// Pre-formatted dollar string, e.g. "$50.00".
    pub budget_usd: String,
    /// Threshold that fired (80 or 100) — not the actual computed percentage.
    pub percent: i16,
    /// "YYYY-MM" UTC calendar month.
    pub month_bucket: String,
    /// Fully-qualified dashboard URL (`{public_base_url}/{slug}/settings`).
    pub dashboard_url: String,
    /// Origin only, no trailing slash. Mirrors Phase 4's other ctx structs.
    pub public_base_url: String,
}
```

- [ ] **Step 4: Register the templates in `TemplateRegistry::load`**

Still in `crates/email/src/templates.rs`. Find the `TemplateRegistry::load` function. Inside the function body, immediately **below** the existing `password_reset.html` registration, add two new registrations:

```rust
        hb.register_template_string("budget_alert.txt", include_str!("../templates/budget_alert.txt.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
        hb.register_template_string("budget_alert.html", include_str!("../templates/budget_alert.html.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
```

- [ ] **Step 5: Add the `render_budget_alert` method**

Still in `crates/email/src/templates.rs`. Find the existing `render_password_reset` method on `TemplateRegistry`. Add the new method immediately **below** it:

```rust
    pub fn render_budget_alert(&self, ctx: BudgetAlertCtx) -> Result<EmailMessage, EmailError> {
        Ok(EmailMessage {
            to: ctx.recipient_email.clone(),
            subject: format!("{}: {} budget at {}%", self.from_name, ctx.org_name, ctx.percent),
            text_body: self.hb.render("budget_alert.txt", &ctx)?,
            html_body: Some(self.hb.render("budget_alert.html", &ctx)?),
        })
    }
```

- [ ] **Step 6: Run the test to verify it still fails (now with template-not-found)**

Run: `cargo test -p llm-gateway-email renders_budget_alert_at_threshold 2>&1 | tail -10`

Expected: compile clean now, but the test panics with a template-not-found error: `Template "budget_alert.txt" not found` (because Task 7 creates the .hbs files). This is the expected next-step failure.

- [ ] **Step 7: Commit (test will pass after Task 7; we commit the code now)**

```bash
git -C /workspace/llm-gateway add crates/email/src/templates.rs
git -C /workspace/llm-gateway commit -m "feat(phase8): BudgetAlertCtx + render_budget_alert on TemplateRegistry"
```

---

## Task 7: Email templates — `budget_alert.txt.hbs` + `budget_alert.html.hbs`

Two new Handlebars templates referenced by Task 6's `include_str!` calls.

**Files:**
- Create: `crates/email/templates/budget_alert.txt.hbs`
- Create: `crates/email/templates/budget_alert.html.hbs`

- [ ] **Step 1: Write the plain-text template**

Create `crates/email/templates/budget_alert.txt.hbs`:

```hbs
{{org_name}} has used {{accrued_usd}} of its {{budget_usd}} monthly budget ({{percent}}%).

Current spend: {{accrued_usd}}
Monthly budget: {{budget_usd}}
Month: {{month_bucket}}

Review spending at {{dashboard_url}}

— The Team
```

- [ ] **Step 2: Write the HTML template**

Create `crates/email/templates/budget_alert.html.hbs`:

```hbs
<!doctype html>
<html>
  <body style="font-family: -apple-system, BlinkMacSystemFont, sans-serif; color: #111;">
    <h2>{{org_name}} budget at {{percent}}%</h2>
    <p>
      {{org_name}} has used <strong>{{accrued_usd}}</strong>
      of its <strong>{{budget_usd}}</strong> monthly budget.
    </p>
    <table style="border-collapse: collapse; margin: 12px 0;">
      <tr>
        <td style="padding: 4px 12px 4px 0; color: #555;">Current spend:</td>
        <td style="padding: 4px 0;"><strong>{{accrued_usd}}</strong></td>
      </tr>
      <tr>
        <td style="padding: 4px 12px 4px 0; color: #555;">Monthly budget:</td>
        <td style="padding: 4px 0;"><strong>{{budget_usd}}</strong></td>
      </tr>
      <tr>
        <td style="padding: 4px 12px 4px 0; color: #555;">Month:</td>
        <td style="padding: 4px 0;">{{month_bucket}}</td>
      </tr>
    </table>
    <p>
      <a href="{{dashboard_url}}"
         style="display: inline-block; padding: 10px 20px; background: #2563eb; color: #fff; text-decoration: none; border-radius: 4px;">
        Review spending
      </a>
    </p>
    <p style="color: #555; font-size: 12px;">
      Or paste this link into your browser: {{dashboard_url}}
    </p>
  </body>
</html>
```

- [ ] **Step 3: Run the email test from Task 6 — now passes**

Run: `cargo test -p llm-gateway-email renders_budget_alert_at_threshold 2>&1 | tail -10`

Expected: `1 passed; 0 failed`.

- [ ] **Step 4: Run the full email test suite to catch regressions**

Run: `cargo test -p llm-gateway-email 2>&1 | tail -10`

Expected: all Phase 4 tests still pass (`renders_verification_url`, `renders_invitation_recipient`, `renders_password_reset`, `renders_budget_alert_at_threshold`).

- [ ] **Step 5: Commit**

```bash
git -C /workspace/llm-gateway add crates/email/templates/budget_alert.txt.hbs crates/email/templates/budget_alert.html.hbs
git -C /workspace/llm-gateway commit -m "feat(phase8): budget_alert email templates"
```

---

## Task 8: Usage-worker — `lib.rs + main.rs` split (no behavior change)

Refactor the pure-binary crate so its logic is reachable from integration tests in `tests/`. The `run_usage_worker` function moves to `lib.rs`; `main.rs` becomes a thin wrapper that loads config, constructs deps, and calls `run_usage_worker` in a supervisor loop.

This task is **purely structural** — no behavior change, no new dependencies on email yet. That comes in Task 11. Keeping the structural change separate makes a regression easy to localize.

**Files:**
- Create: `crates/usage-worker/src/lib.rs`
- Modify: `crates/usage-worker/src/main.rs` (becomes a thin wrapper)
- Modify: `crates/usage-worker/Cargo.toml` (declare both `lib` and `bin` targets)

- [ ] **Step 1: Add the `lib` target to `Cargo.toml`**

Open `crates/usage-worker/Cargo.toml`. The current file declares only a `[[bin]]`. Add a `[lib]` section **above** the `[[bin]]` section:

```toml
[lib]
name = "llm_gateway_usage_worker"
path = "src/lib.rs"

[[bin]]
name = "llm-gateway-usage-worker"
path = "src/main.rs"
```

- [ ] **Step 2: Create `src/lib.rs` with the worker loop extracted from `main.rs`**

Create `crates/usage-worker/src/lib.rs`:

```rust
//! Usage worker library — extracts the NATS consume loop from `main.rs` so
//! integration tests can reach `run_usage_worker` directly.
//!
//! Phase 8 adds `check_budget_alerts` (see `budget_alerts.rs`) and threads a
//! `Mailer` + `TemplateRegistry` + `public_base_url` into `run_usage_worker`.
//! Until Task 11 wires those in, this module is a pure extract of the
//! pre-Phase-8 main loop.

use futures::StreamExt;
use llm_gateway_nats_publisher::AckKind;
use llm_gateway_storage::{
    DeductBalance, DeductBalanceResult, Protocol, Storage, TransactionType, UsageRecord,
};
use std::sync::Arc;

/// One consume iteration. Returns when the NATS consumer stream ends or the
/// supervisor restarts the loop. Package-private to the crate so the binary
/// wrapper and integration tests can both reach it.
pub async fn run_usage_worker(
    storage: Arc<dyn Storage>,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
) {
    tracing::info!("[USAGE-WORKER] Starting");

    let consumer = match nats.create_usage_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[USAGE-WORKER] Failed to create consumer: {}", e);
            return;
        }
    };

    let mut messages = match consumer.messages().await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("[USAGE-WORKER] Failed to subscribe: {}", e);
            return;
        }
    };

    while let Some(Ok(msg)) = messages.next().await {
        let parse_result: Result<llm_gateway_nats_publisher::UsageEvent, _> =
            serde_json::from_slice(&msg.payload);
        let event = match parse_result {
            Ok(e) => {
                tracing::info!(
                    "[USAGE-WORKER] received usage event request_id={} cost={}",
                    e.request_id,
                    e.cost
                );
                e
            }
            Err(e) => {
                tracing::warn!("[USAGE-WORKER] Failed to deserialize: {}", e);
                let _ = msg.ack().await;
                continue;
            }
        };

        let record = UsageRecord {
            id: event.id,
            org_id: event.org_id.clone(),
            request_id: Some(event.request_id),
            key_id: event.key_id,
            user_id: event.user_id,
            model_name: event.model_name,
            provider_id: event.provider_id,
            channel_id: event.channel_id,
            protocol: match event.protocol.as_str() {
                "anthropic" => Protocol::Anthropic,
                _ => Protocol::Openai,
            },
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
            cache_read_tokens: event.cache_read_tokens,
            cache_creation_tokens: event.cache_creation_tokens,
            cost: event.cost,
            pricing_policy: event.pricing_policy,
            weighted_tokens: event.weighted_tokens,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.created_at)
                .map(|dt| dt.to_utc())
                .unwrap_or_else(|_| chrono::Utc::now()),
        };

        if record.cost == 0 {
            tracing::debug!(
                "[USAGE-WORKER] skipping cost=0 request_id={:?}",
                record.request_id
            );
            let _ = msg.ack().await;
            continue;
        }

        if let Err(e) = storage.record_usage(&record.org_id, &record).await {
            tracing::warn!("[USAGE-WORKER] Failed to record usage: {}", e);
            let _ = msg.ack_with(AckKind::Nak(None)).await;
            continue;
        }

        tracing::info!(
            "[USAGE-WORKER] successfully recorded usage request_id={:?}",
            record.request_id
        );

        // Per-request balance deduction (unchanged from pre-Phase-8).
        deduct_user_balance(&storage, &record).await;

        let _ = msg.ack().await;
    }

    tracing::info!("[USAGE-WORKER] Exiting");
}

/// Per-request user-balance deduction. Extracted from the inline block so the
/// main loop reads cleanly. Behavior is identical to pre-Phase-8.
async fn deduct_user_balance(storage: &Arc<dyn Storage>, record: &UsageRecord) {
    let Some(user_id) = record.user_id.as_ref() else { return };
    if record.cost <= 0 {
        return;
    }
    let account = match storage.get_account_by_user_id(&record.org_id, user_id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            tracing::debug!("[USAGE] No account for user={}, skipping deduction", user_id);
            return;
        }
        Err(e) => {
            tracing::error!("[USAGE] Failed to lookup account for user={}: {}", user_id, e);
            return;
        }
    };
    let Some(rid) = record.request_id.as_deref() else {
        tracing::warn!(
            "[USAGE] No request_id for usage record {}, skipping deduction",
            record.id
        );
        return;
    };
    match storage.get_transaction_by_request_id(&record.org_id, rid).await {
        Ok(None) => {
            let req = DeductBalance {
                account_id: account.id,
                amount: record.cost,
                transaction_type: TransactionType::Debit,
                description: Some(format!("{} - {}", record.model_name, rid)),
                reference_id: None,
                request_id: Some(rid.to_string()),
            };
            match storage.deduct_balance(&record.org_id, &req).await {
                Ok(DeductBalanceResult::Success(_)) => {}
                Ok(DeductBalanceResult::InsufficientBalance {
                    current_balance,
                    requested,
                }) => {
                    tracing::warn!(
                        "[USAGE] Insufficient balance for user={}, balance={}, cost={}",
                        user_id,
                        current_balance,
                        requested
                    );
                }
                Ok(DeductBalanceResult::AccountNotFound) => {
                    tracing::warn!("[USAGE] Account not found for user={}", user_id);
                }
                Err(e) => {
                    tracing::error!("[USAGE] Deduction failed for request_id={}: {}", rid, e);
                }
            }
        }
        Ok(Some(_)) => {
            tracing::debug!("[USAGE] Already deducted for request_id={}", rid);
        }
        Err(e) => {
            tracing::error!("[USAGE] Idempotency check failed for request_id={}: {}", rid, e);
        }
    }
}
```

- [ ] **Step 3: Rewrite `src/main.rs` as a thin wrapper**

Replace the entire contents of `crates/usage-worker/src/main.rs` with:

```rust
use llm_gateway_storage::{postgres::PostgresStorage, AppConfig, Storage};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let config_str = std::fs::read_to_string("config.toml")?;
    let config_str = shellexpand::env(&config_str)?.to_string();
    let config: AppConfig = toml::from_str(&config_str)?;

    // Connect to PostgreSQL
    let url = config.database.url.as_deref().ok_or("database.url is required")?;
    tracing::info!("Connecting to PostgreSQL: {}", url.split('@').last().unwrap_or("***"));
    let storage: Arc<dyn Storage> = {
        let db = PostgresStorage::new(url).await?;
        db.run_migrations().await?;
        Arc::new(db)
    };

    // Connect to NATS
    let nats_cfg = config.nats.as_ref().ok_or("[nats] section is required")?;
    let nats = Arc::new(
        llm_gateway_nats_publisher::NatsPublisher::new(
            &nats_cfg.url,
            nats_cfg.token.clone(),
            nats_cfg.credentials_file.clone(),
        )
        .await?,
    );
    tracing::info!("Connected to NATS: {}", nats_cfg.url);

    // Run with supervisor
    loop {
        llm_gateway_usage_worker::run_usage_worker(storage.clone(), nats.clone()).await;
        tracing::warn!("[USAGE-WORKER] exited, restarting in 5s");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
```

- [ ] **Step 4: Build the workspace to verify the refactor compiles**

Run: `cargo build -p llm-gateway-usage-worker 2>&1 | tail -15`

Expected: clean build. The `llm_gateway_usage_worker` lib name (underscores — derived from the crate name `llm-gateway-usage-worker` per Rust's crate-name convention) is what `main.rs` references.

- [ ] **Step 5: Commit**

```bash
git -C /workspace/llm-gateway add crates/usage-worker/Cargo.toml crates/usage-worker/src/lib.rs crates/usage-worker/src/main.rs
git -C /workspace/llm-gateway commit -m "refactor(phase8): split usage-worker into lib + bin for testability"
```

---

## Task 9: Usage-worker `budget_alerts.rs` — orchestrator + helpers + unit tests

The new module that owns the threshold-check + claim + send + mark-sent dance. Includes pure-function helpers `passes_threshold` and `format_usd` that are unit-tested inline.

**Files:**
- Modify: `crates/usage-worker/Cargo.toml` (add `llm-gateway-email` dep)
- Create: `crates/usage-worker/src/budget_alerts.rs`
- Modify: `crates/usage-worker/src/lib.rs` (add `pub mod budget_alerts;`)

- [ ] **Step 1: Add `llm-gateway-email` dependency**

Open `crates/usage-worker/Cargo.toml`. In the `[dependencies]` section, immediately **below** the `llm-gateway-storage` line, add:

```toml
llm-gateway-email = { path = "../email" }
```

- [ ] **Step 2: Write the failing unit tests for `passes_threshold` + `format_usd` first**

Create `crates/usage-worker/src/budget_alerts.rs` with only the tests + empty stubs for now:

```rust
//! Phase 8: budget-alert orchestrator. After `record_usage` succeeds, this
//! module evaluates whether the org's MTD just crossed an alert threshold,
//! and if so, claims + sends + marks-sent one alert per
//! (org, month, threshold).

use std::sync::Arc;
use llm_gateway_email::{Mailer, templates::{BudgetAlertCtx, TemplateRegistry}};
use llm_gateway_storage::{BudgetAlertSnapshot, Storage};

/// After record_usage succeeds, evaluate whether the org's MTD just crossed
/// an alert threshold. Every failure is logged and swallowed — usage is
/// already recorded, so the NATS message must still be acked.
pub async fn check_budget_alerts(
    storage: &Arc<dyn Storage>,
    mailer: &Arc<dyn Mailer>,
    templates: &TemplateRegistry,
    public_base_url: &str,
    org_id: &str,
) {
    // Body filled in Step 4 below.
}

/// Integer-safe ratio check: accrued/budget >= threshold/100.
/// Uses only integer math — no float drift at the boundary.
pub(crate) fn passes_threshold(accrued: i64, budget: i64, threshold: i16) -> bool {
    accrued.saturating_mul(100) >= budget.saturating_mul(threshold as i64)
}

/// Format 10^8 subunits as a "$D.cc" string with exactly two decimals.
pub(crate) fn format_usd(units: i64) -> String {
    let dollars = units / 100_000_000;
    let cents = (units % 100_000_000) / 1_000_000;
    format!("${}.${:02}", dollars, cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_threshold_at_exact_boundary() {
        // accrued * 100 == budget * threshold → fires (>=)
        assert!(passes_threshold(40_000_000_000, 50_000_000_000, 80));
        assert!(passes_threshold(50_000_000_000, 50_000_000_000, 100));
    }

    #[test]
    fn passes_threshold_just_below_boundary() {
        assert!(!passes_threshold(39_999_999_999, 50_000_000_000, 80));
        assert!(!passes_threshold(49_999_999_999, 50_000_000_000, 100));
    }

    #[test]
    fn passes_threshold_over_budget() {
        // 150% passes both 80 and 100
        assert!(passes_threshold(75_000_000_000, 50_000_000_000, 80));
        assert!(passes_threshold(75_000_000_000, 50_000_000_000, 100));
    }

    #[test]
    fn format_usd_renders_two_decimals() {
        assert_eq!(format_usd(0), "$0.00");
        assert_eq!(format_usd(100_000_000), "$1.00");
        assert_eq!(format_usd(40_500_000_000), "$405.00");
        assert_eq!(format_usd(99_900_000), "$0.99");
    }
}
```

- [ ] **Step 3: Register the module in `lib.rs`**

Open `crates/usage-worker/src/lib.rs`. Add this line at the **top of the file**, above all other items:

```rust
pub mod budget_alerts;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p llm-gateway-usage-worker budget_alerts 2>&1 | tail -15`

Expected: `4 passed; 0 failed` (the `check_budget_alerts` function is an empty stub but the four unit tests on `passes_threshold` and `format_usd` pass).

- [ ] **Step 5: Fill in the `check_budget_alerts` orchestrator**

Replace the empty `check_budget_alerts` body in `crates/usage-worker/src/budget_alerts.rs` with the full implementation. Replace just the function (keep the signature and doc comment):

```rust
pub async fn check_budget_alerts(
    storage: &Arc<dyn Storage>,
    mailer: &Arc<dyn Mailer>,
    templates: &TemplateRegistry,
    public_base_url: &str,
    org_id: &str,
) {
    if let Err(e) = run_check(storage, mailer, templates, public_base_url, org_id).await {
        tracing::warn!("[BUDGET-ALERT] check failed for org={}: {}", org_id, e);
    }
}

async fn run_check(
    storage: &Arc<dyn Storage>,
    mailer: &Arc<dyn Mailer>,
    templates: &TemplateRegistry,
    public_base_url: &str,
    org_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let snapshot = storage.get_org_budget_for_alerts(org_id).await?;
    let Some(budget_units) = snapshot.budget_units else {
        return Ok(()); // unlimited — no alerting
    };
    if budget_units == 0 {
        return Ok(()); // defensive — see spec Edge Cases
    }

    let month_bucket = chrono::Utc::now().format("%Y-%m").to_string();
    let recipients = storage.list_org_admin_emails(org_id).await?;
    if recipients.is_empty() {
        tracing::info!("[BUDGET-ALERT] org={} no eligible recipients; skipping", org_id);
        return Ok(());
    }

    for threshold in [80_i16, 100_i16] {
        if !passes_threshold(snapshot.accrued_units, budget_units, threshold) {
            continue;
        }
        let owned = storage.try_claim_budget_alert(org_id, &month_bucket, threshold).await?;
        let Some(()) = owned else { continue };
        match send_alerts(
            mailer,
            templates,
            public_base_url,
            &snapshot,
            threshold,
            &month_bucket,
            &recipients,
        )
        .await
        {
            Ok(()) => {
                if let Err(e) = storage
                .mark_budget_alert_sent(org_id, &month_bucket, threshold)
                .await
                {
                    tracing::warn!("[BUDGET-ALERT] failed to mark sent: {}", e);
                }
            }
            Err(e) => {
                // Leave sent_at NULL so a future request retries via the same row.
                tracing::warn!(
                    "[BUDGET-ALERT] send failed org={} threshold={}: {}",
                    org_id,
                    threshold,
                    e
                );
            }
        }
    }
    Ok(())
}

async fn send_alerts(
    mailer: &Arc<dyn Mailer>,
    templates: &TemplateRegistry,
    public_base_url: &str,
    snapshot: &BudgetAlertSnapshot,
    threshold: i16,
    month_bucket: &str,
    recipients: &[String],
) -> Result<(), llm_gateway_email::EmailError> {
    let accrued_usd = format_usd(snapshot.accrued_units);
    let budget_usd = format_usd(snapshot.budget_units.expect("caller checked Some"));
    let dashboard_path = format!("/{}/settings", snapshot.org_slug);
    let dashboard_url = format!(
        "{}{}",
        public_base_url.trim_end_matches('/'),
        dashboard_path
    );

    for recipient in recipients {
        let ctx = BudgetAlertCtx {
            org_name: snapshot.org_name.clone(),
            org_slug: snapshot.org_slug.clone(),
            recipient_email: recipient.clone(),
            accrued_usd: accrued_usd.clone(),
            budget_usd: budget_usd.clone(),
            percent: threshold,
            month_bucket: month_bucket.to_string(),
            dashboard_url: dashboard_url.clone(),
            public_base_url: public_base_url.to_string(),
        };
        let msg = templates.render_budget_alert(ctx)?;
        // Surface error to caller: leaving sent_at NULL triggers retry on next request.
        mailer.send(msg).await?;
    }
    Ok(())
}
```

- [ ] **Step 6: Build the worker crate to verify everything compiles**

Run: `cargo build -p llm-gateway-usage-worker 2>&1 | tail -15`

Expected: clean build.

- [ ] **Step 7: Run the unit tests again to confirm no regressions**

Run: `cargo test -p llm-gateway-usage-worker budget_alerts 2>&1 | tail -10`

Expected: `4 passed; 0 failed`.

- [ ] **Step 8: Commit**

```bash
git -C /workspace/llm-gateway add crates/usage-worker/Cargo.toml crates/usage-worker/src/budget_alerts.rs crates/usage-worker/src/lib.rs
git -C /workspace/llm-gateway commit -m "feat(phase8): usage-worker budget_alerts module + unit tests"
```

---

## Task 10: Usage-worker integration tests — `tests/budget_alert_flow.rs`

End-to-end tests that exercise storage + `check_budget_alerts` + a recording mailer. Five scenarios covering the alert flow + all four skip-conditions from the spec.

**Files:**
- Create: `crates/usage-worker/tests/budget_alert_flow.rs`
- Modify: `crates/usage-worker/Cargo.toml` (add `tokio` test features + dev-deps)

- [ ] **Step 1: Add test dependencies to `Cargo.toml`**

Open `crates/usage-worker/Cargo.toml`. Add this `[dev-dependencies]` section at the bottom (if one doesn't already exist):

```toml
[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
sqlx = { workspace = true, features = ["postgres", "runtime-tokio-rustls", "macros", "migrate"] }
```

Note: `sqlx` is needed because the integration tests use `#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]` to get a fresh DB per test.

- [ ] **Step 2: Write the `RecordingMailer` test double + all five tests**

Create `crates/usage-worker/tests/budget_alert_flow.rs`:

```rust
//! Integration tests for Phase 8 budget-alert flow. Exercises storage +
//! check_budget_alerts + a recording mailer in a real Postgres instance.

use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use llm_gateway_email::{EmailError, EmailMessage, Mailer, templates::TemplateRegistry};
use llm_gateway_storage::postgres::PostgresStorage;
use llm_gateway_storage::Storage;

/// Records every `send` call so tests can assert on recipient list + content.
#[derive(Default)]
struct RecordingMailer {
    sends: Mutex<Vec<EmailMessage>>,
}

impl RecordingMailer {
    fn snapshot(&self) -> Vec<EmailMessage> {
        self.sends.lock().expect("sends lock").clone()
    }
}

#[async_trait]
impl Mailer for RecordingMailer {
    async fn send(&self, msg: EmailMessage) -> Result<(), EmailError> {
        self.sends.lock().expect("sends lock").push(msg);
        Ok(())
    }
}

/// FailMailer: always returns Err. Used to exercise the "leave sent_at NULL"
/// retry path.
struct FailMailer;

#[async_trait]
impl Mailer for FailMailer {
    async fn send(&self, _msg: EmailMessage) -> Result<(), EmailError> {
        Err(EmailError::Smtp("simulated failure".into()))
    }
}

/// Helper: register a user + give them a role in the org + verify their email.
async fn seed_admin(
    storage: &PostgresStorage,
    username: &str,
    email: &str,
    org_id: &str,
    role: &str,
    verified: bool,
) {
    let now = chrono::Utc::now();
    let user = llm_gateway_storage::User {
        id: username.to_string(),
        username: username.to_string(),
        password: "x".to_string(),
        platform_role: None,
        current_org_id: None,
        enabled: true,
        refresh_token: None,
        created_at: now,
        updated_at: now,
        email: Some(email.to_string()),
        email_verified_at: verified.then(|| now),
        requires_email_verification: false,
        password_changed_at: now,
    };
    storage.create_user(&user).await.expect("create_user");
    sqlx::query(
        "INSERT INTO members (user_id, org_id, role, created_by)
         VALUES ($1, $2, $3, $1)
         ON CONFLICT (user_id, org_id) DO UPDATE SET role = EXCLUDED.role",
    )
    .bind(username)
    .bind(org_id)
    .bind(role)
    .execute(&storage.pool)
    .await
    .expect("insert member");
}

async fn set_budget(storage: &PostgresStorage, org_id: &str, budget_usd: &str) {
    sqlx::query(
        "INSERT INTO org_settings (org_id, key, value) VALUES ($1, 'default_budget_monthly_usd', $2)
         ON CONFLICT (org_id, key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(org_id)
    .bind(budget_usd)
    .execute(&storage.pool)
    .await
    .expect("set budget");
}

async fn assert_unsent_claim_count(storage: &PostgresStorage, org_id: &str, expected: i64) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM budget_alerts_sent WHERE org_id = $1 AND sent_at IS NULL",
    )
    .bind(org_id)
    .fetch_one(&storage.pool)
    .await
    .expect("count unsent");
    assert_eq!(count, expected, "unsent claim count mismatch");
}

async fn assert_sent_claim_count(storage: &PostgresStorage, org_id: &str, expected: i64) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM budget_alerts_sent WHERE org_id = $1 AND sent_at IS NOT NULL",
    )
    .bind(org_id)
    .fetch_one(&storage.pool)
    .await
    .expect("count sent");
    assert_eq!(count, expected, "sent claim count mismatch");
}

fn templates() -> TemplateRegistry {
    TemplateRegistry::load("noreply@example.com".into(), "LLM Gateway".into())
        .expect("templates load")
}

/// Happy path: $10 budget, $8 spend → 80% alert fires + sent to 1 admin.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn budget_alert_flow_e2e(pool: sqlx::PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    // Create org + admin (bypassing make_test_org because that helper is
    // crate-private to llm-gateway-storage).
    let owner_id = "owner-flow";
    sqlx::query(
        "INSERT INTO users (id, username, password, created_at, updated_at)
         VALUES ($1, $1, 'x', NOW(), NOW())",
    )
    .bind(owner_id)
    .execute(&storage.pool)
    .await
    .expect("insert owner");
    sqlx::query(
        "INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
         VALUES ('org-flow', 'org-flow', 'Flow Org', $1, NOW(), NOW())",
    )
    .bind(owner_id)
    .execute(&storage.pool)
    .await
    .expect("insert org");

    // Add a verified admin recipient.
    seed_admin(&storage, "alice", "alice@example.com", "org-flow", "admin", true).await;

    // $10 budget.
    set_budget(&storage, "org-flow", "10").await;

    // $8 spend this month — crosses 80%, not 100%.
    sqlx::query(
        "INSERT INTO api_keys (id, org_id, name, key_hash, enabled, created_at, updated_at)
         VALUES ('key-flow', 'org-flow', 'k', 'hash', true, NOW(), NOW())",
    )
    .execute(&storage.pool)
    .await
    .expect("insert key");
    let eight_usd = llm_gateway_storage::money::usd_to_units(8.0);
    sqlx::query(
        "INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at)
         VALUES ('key-flow', to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM'), $1, NOW())",
    )
    .bind(eight_usd)
    .execute(&storage.pool)
    .await
    .expect("seed budget_counters");

    let mailer = Arc::new(RecordingMailer::default());
    let tmpl = templates();
    // Pass an upcast clone to check_budget_alerts; keep the concrete Arc so we
    // can call snapshot() after the call returns.
    llm_gateway_usage_worker::budget_alerts::check_budget_alerts(
        &storage,
        &(mailer.clone() as Arc<dyn Mailer>),
        &tmpl,
        "https://gateway.example.com",
        "org-flow",
    )
    .await;

    let sends = mailer.snapshot();

    // 80% threshold fires; 100% does not ($8 < $10). One recipient → one send.
    assert_eq!(sends.len(), 1, "exactly one email for 80% threshold");
    assert_eq!(sends[0].to, "alice@example.com");
    assert!(sends[0].subject.contains("Flow Org"), "subject has org name");
    assert!(sends[0].subject.contains("80%"), "subject has threshold");
    assert!(sends[0].text_body.contains("$8.00"), "body has accrued");
    assert!(sends[0].text_body.contains("$10.00"), "body has budget");
    assert!(
        sends[0]
            .text_body
            .contains("https://gateway.example.com/org-flow/settings"),
        "body has dashboard URL"
    );

    // Dedup row exists with sent_at NOT NULL.
    assert_sent_claim_count(&storage, "org-flow", 1).await;
    assert_unsent_claim_count(&storage, "org-flow", 0).await;
}

/// No budget → no alert, no dedup row.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn budget_alert_skips_when_no_budget(pool: sqlx::PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    sqlx::query("INSERT INTO users (id, username, password, created_at, updated_at) VALUES ('o', 'o', 'x', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    sqlx::query("INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at) VALUES ('org-nb', 'org-nb', 'NB', 'o', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    seed_admin(&storage, "bob", "bob@example.com", "org-nb", "admin", true).await;
    // No default_budget_monthly_usd set.
    let mailer = Arc::new(RecordingMailer::default());
    let tmpl = templates();
    llm_gateway_usage_worker::budget_alerts::check_budget_alerts(
        &storage,
        &(mailer.clone() as Arc<dyn Mailer>),
        &tmpl,
        "https://x.example",
        "org-nb",
    )
    .await;
    // Verify no emails were sent + no dedup rows were created.
    assert!(mailer.snapshot().is_empty(), "no emails should fire without a budget");
    assert_sent_claim_count(&storage, "org-nb", 0).await;
    assert_unsent_claim_count(&storage, "org-nb", 0).await;
}

/// Admin's email is unverified → no alert sent, claim left at 0.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn budget_alert_skips_recipient_unverified(pool: sqlx::PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    sqlx::query("INSERT INTO users (id, username, password, created_at, updated_at) VALUES ('o', 'o', 'x', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    sqlx::query("INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at) VALUES ('org-uv', 'org-uv', 'UV', 'o', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    seed_admin(&storage, "carol", "carol@example.com", "org-uv", "admin", false).await;
    set_budget(&storage, "org-uv", "10").await;

    // $20 spend — would normally fire both 80% and 100%. But no verified
    // recipient → no claim at all.
    sqlx::query("INSERT INTO api_keys (id, org_id, name, key_hash, enabled, created_at, updated_at) VALUES ('k', 'org-uv', 'k', 'h', true, NOW(), NOW())").execute(&storage.pool).await.unwrap();
    let twenty = llm_gateway_storage::money::usd_to_units(20.0);
    sqlx::query("INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at) VALUES ('k', to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM'), $1, NOW())").bind(twenty).execute(&storage.pool).await.unwrap();

    let mailer = Arc::new(RecordingMailer::default());
    let tmpl = templates();
    llm_gateway_usage_worker::budget_alerts::check_budget_alerts(
        &storage,
        &(mailer.clone() as Arc<dyn Mailer>),
        &tmpl,
        "https://x.example",
        "org-uv",
    )
    .await;
    assert!(mailer.snapshot().is_empty(), "unverified recipient → no send");
    assert_sent_claim_count(&storage, "org-uv", 0).await;
    assert_unsent_claim_count(&storage, "org-uv", 0).await;
}

/// Budget = $0 → defensive skip (no infinite-alert loop).
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn budget_alert_skips_zero_budget(pool: sqlx::PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    sqlx::query("INSERT INTO users (id, username, password, created_at, updated_at) VALUES ('o', 'o', 'x', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    sqlx::query("INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at) VALUES ('org-zero', 'org-zero', 'Zero', 'o', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    seed_admin(&storage, "dave", "dave@example.com", "org-zero", "admin", true).await;
    set_budget(&storage, "org-zero", "0").await;
    // $5 spend — would pass `accrued*100 >= 0*threshold` trivially without the
    // zero-budget skip.
    sqlx::query("INSERT INTO api_keys (id, org_id, name, key_hash, enabled, created_at, updated_at) VALUES ('k', 'org-zero', 'k', 'h', true, NOW(), NOW())").execute(&storage.pool).await.unwrap();
    let five = llm_gateway_storage::money::usd_to_units(5.0);
    sqlx::query("INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at) VALUES ('k', to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM'), $1, NOW())").bind(five).execute(&storage.pool).await.unwrap();

    let mailer = Arc::new(RecordingMailer::default());
    let tmpl = templates();
    llm_gateway_usage_worker::budget_alerts::check_budget_alerts(
        &storage,
        &(mailer.clone() as Arc<dyn Mailer>),
        &tmpl,
        "https://x.example",
        "org-zero",
    )
    .await;
    assert!(mailer.snapshot().is_empty(), "zero budget → defensive skip");
    assert_sent_claim_count(&storage, "org-zero", 0).await;
    assert_unsent_claim_count(&storage, "org-zero", 0).await;
}

/// Org has only `member` role users → no eligible recipients → no claim.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn budget_alert_skips_when_no_recipients(pool: sqlx::PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    sqlx::query("INSERT INTO users (id, username, password, created_at, updated_at) VALUES ('o', 'o', 'x', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    sqlx::query("INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at) VALUES ('org-mem', 'org-mem', 'Mem', 'o', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    seed_admin(&storage, "eve", "eve@example.com", "org-mem", "member", true).await;
    set_budget(&storage, "org-mem", "10").await;
    sqlx::query("INSERT INTO api_keys (id, org_id, name, key_hash, enabled, created_at, updated_at) VALUES ('k', 'org-mem', 'k', 'h', true, NOW(), NOW())").execute(&storage.pool).await.unwrap();
    let twenty = llm_gateway_storage::money::usd_to_units(20.0);
    sqlx::query("INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at) VALUES ('k', to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM'), $1, NOW())").bind(twenty).execute(&storage.pool).await.unwrap();

    let mailer = Arc::new(RecordingMailer::default());
    let tmpl = templates();
    llm_gateway_usage_worker::budget_alerts::check_budget_alerts(
        &storage,
        &(mailer.clone() as Arc<dyn Mailer>),
        &tmpl,
        "https://x.example",
        "org-mem",
    )
    .await;
    assert!(mailer.snapshot().is_empty(), "no eligible recipients → no send");
    assert_sent_claim_count(&storage, "org-mem", 0).await;
    assert_unsent_claim_count(&storage, "org-mem", 0).await;
}

/// Failed send → claim row stays unsent → next call re-arms + retries.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn budget_alert_retries_after_failed_send(pool: sqlx::PgPool) {
    let storage = Arc::new(PostgresStorage::from_pool(pool));
    sqlx::query("INSERT INTO users (id, username, password, created_at, updated_at) VALUES ('o', 'o', 'x', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    sqlx::query("INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at) VALUES ('org-retry', 'org-retry', 'Retry', 'o', NOW(), NOW())").execute(&storage.pool).await.unwrap();
    seed_admin(&storage, "frank", "frank@example.com", "org-retry", "admin", true).await;
    set_budget(&storage, "org-retry", "10").await;
    sqlx::query("INSERT INTO api_keys (id, org_id, name, key_hash, enabled, created_at, updated_at) VALUES ('k', 'org-retry', 'k', 'h', true, NOW(), NOW())").execute(&storage.pool).await.unwrap();
    let eight = llm_gateway_storage::money::usd_to_units(8.0);
    sqlx::query("INSERT INTO budget_counters (key_id, month_bucket, accrued, updated_at) VALUES ('k', to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM'), $1, NOW())").bind(eight).execute(&storage.pool).await.unwrap();

    // First call with FailMailer → claim exists, sent_at stays NULL.
    let failer = Arc::new(FailMailer) as Arc<dyn Mailer>;
    let tmpl = templates();
    llm_gateway_usage_worker::budget_alerts::check_budget_alerts(
        &storage, &failer, &tmpl, "https://x.example", "org-retry",
    )
    .await;
    assert_unsent_claim_count(&storage, "org-retry", 1).await;
    assert_sent_claim_count(&storage, "org-retry", 0).await;

    // Second call with a working mailer → re-arms via NULL sent_at + sends successfully.
    let worker = Arc::new(RecordingMailer::default());
    llm_gateway_usage_worker::budget_alerts::check_budget_alerts(
        &storage,
        &(worker.clone() as Arc<dyn Mailer>),
        &tmpl,
        "https://x.example",
        "org-retry",
    )
    .await;
    assert_eq!(worker.snapshot().len(), 1, "retry should send exactly one email");
    assert_unsent_claim_count(&storage, "org-retry", 0).await;
    assert_sent_claim_count(&storage, "org-retry", 1).await;
}
```

- [ ] **Step 3: Run the integration tests**

Run: `cargo test -p llm-gateway-usage-worker --test budget_alert_flow 2>&1 | tail -25`

Expected: `6 passed; 0 failed` (e2e + 4 skips + 1 retry). Requires a running Postgres with `DATABASE_URL` set (per the project's existing Phase 4-7 integration-test convention — the `#[sqlx::test]` macro provisions a fresh DB per test).

If Postgres is unavailable in CI, the `sqlx::test` macro will skip the tests with a warning. Locally, `export DATABASE_URL=postgres://...` per the project's existing test setup.

- [ ] **Step 4: Commit**

```bash
git -C /workspace/llm-gateway add crates/usage-worker/Cargo.toml crates/usage-worker/tests/budget_alert_flow.rs
git -C /workspace/llm-gateway commit -m "test(phase8): usage-worker integration tests for budget-alert flow"
```

---

## Task 11: Wire `check_budget_alerts` into the worker's main loop + Cargo.toml deps

This is where Phase 8 becomes user-visible: after `record_usage` succeeds, the worker calls `check_budget_alerts`. Also: construct the mailer + templates at startup (mirror of `gateway/main.rs:82-120`), and read `server.public_base_url`.

**Files:**
- Modify: `crates/usage-worker/src/lib.rs` (extend `run_usage_worker` signature + call `check_budget_alerts`)
- Modify: `crates/usage-worker/src/main.rs` (construct mailer + templates + `public_base_url`, pass into `run_usage_worker`)

- [ ] **Step 1: Extend the `run_usage_worker` signature**

Open `crates/usage-worker/src/lib.rs`. Change the signature of `run_usage_worker` from:

```rust
pub async fn run_usage_worker(
    storage: Arc<dyn Storage>,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
) {
```

to:

```rust
pub async fn run_usage_worker(
    storage: Arc<dyn Storage>,
    nats: Arc<llm_gateway_nats_publisher::NatsPublisher>,
    mailer: Arc<dyn llm_gateway_email::Mailer>,
    templates: Arc<llm_gateway_email::templates::TemplateRegistry>,
    public_base_url: String,
) {
```

- [ ] **Step 2: Call `check_budget_alerts` after `record_usage` succeeds**

Still in `crates/usage-worker/src/lib.rs`. Find the existing block:

```rust
        tracing::info!(
            "[USAGE-WORKER] successfully recorded usage request_id={:?}",
            record.request_id
        );

        // Per-request balance deduction (unchanged from pre-Phase-8).
        deduct_user_balance(&storage, &record).await;
```

Insert the `check_budget_alerts` call between the success log and the deduction:

```rust
        tracing::info!(
            "[USAGE-WORKER] successfully recorded usage request_id={:?}",
            record.request_id
        );

        // Phase 8: evaluate budget-alert thresholds. Failures are swallowed
        // inside check_budget_alerts (usage is already recorded — the message
        // must still be acked).
        budget_alerts::check_budget_alerts(
            &storage,
            &mailer,
            &templates,
            &public_base_url,
            &record.org_id,
        )
        .await;

        // Per-request balance deduction (unchanged from pre-Phase-8).
        deduct_user_balance(&storage, &record).await;
```

- [ ] **Step 3: Construct the mailer + templates + `public_base_url` in `main.rs`**

Open `crates/usage-worker/src/main.rs`. Find the existing block (added in Task 8):

```rust
    // Connect to NATS
    let nats_cfg = config.nats.as_ref().ok_or("[nats] section is required")?;
    let nats = Arc::new(
        llm_gateway_nats_publisher::NatsPublisher::new(
            &nats_cfg.url,
            nats_cfg.token.clone(),
            nats_cfg.credentials_file.clone(),
        )
        .await?,
    );
    tracing::info!("Connected to NATS: {}", nats_cfg.url);

    // Run with supervisor
    loop {
        llm_gateway_usage_worker::run_usage_worker(storage.clone(), nats.clone()).await;
```

Replace this block with:

```rust
    // Connect to NATS
    let nats_cfg = config.nats.as_ref().ok_or("[nats] section is required")?;
    let nats = Arc::new(
        llm_gateway_nats_publisher::NatsPublisher::new(
            &nats_cfg.url,
            nats_cfg.token.clone(),
            nats_cfg.credentials_file.clone(),
        )
        .await?,
    );
    tracing::info!("Connected to NATS: {}", nats_cfg.url);

    // Phase 8: construct the mailer + templates + public_base_url for
    // budget-alert dispatch. Mirrors gateway/main.rs:82-120.
    let templates = Arc::new(
        llm_gateway_email::templates::TemplateRegistry::load(
            config.email.from_address.clone(),
            config.email.from_name.clone(),
        )
        .map_err(|e| format!("failed to load email templates: {e}"))?,
    );

    let mailer: Arc<dyn llm_gateway_email::Mailer> = match config.email.transport.as_str() {
        "noop" => Arc::new(llm_gateway_email::noop::NoopMailer::new()),
        "file" => {
            std::fs::create_dir_all(&config.email.file_output_dir)
                .map_err(|e| format!("creating email output dir {}: {e}", config.email.file_output_dir))?;
            Arc::new(llm_gateway_email::file::FileMailer::new(
                &config.email.file_output_dir,
                config.email.from_address.clone(),
                config.email.from_name.clone(),
            ))
        }
        "smtp" => {
            let host = config.email.smtp_host.clone()
                .ok_or_else(|| "[email] smtp_host is required when transport = \"smtp\"".to_string())?;
            let port = config.email.smtp_port.unwrap_or(587);
            let cfg = llm_gateway_email::smtp::SmtpMailerConfig {
                host: host.clone(),
                port,
                username: config.email.smtp_username.clone(),
                password: config.email.smtp_password.clone(),
                use_tls: config.email.smtp_use_tls,
                from_address: config.email.from_address.clone(),
                from_name: config.email.from_name.clone(),
            };
            Arc::new(
                llm_gateway_email::smtp::SmtpMailer::new(cfg)
                    .map_err(|e| format!("constructing SMTP mailer for {host}: {e}"))?,
            )
        }
        other => return Err(format!("unknown [email] transport: {other}").into()),
    };

    let public_base_url = config
        .server
        .public_base_url
        .clone()
        .unwrap_or_else(|| "http://localhost:5173".to_string());

    // Run with supervisor
    loop {
        llm_gateway_usage_worker::run_usage_worker(
            storage.clone(),
            nats.clone(),
            mailer.clone(),
            templates.clone(),
            public_base_url.clone(),
        )
        .await;
```

- [ ] **Step 4: Build the workspace to verify the wiring compiles**

Run: `cargo build -p llm-gateway-usage-worker 2>&1 | tail -15`

Expected: clean build.

- [ ] **Step 5: Run the full worker test suite to confirm no regressions**

Run: `cargo test -p llm-gateway-usage-worker 2>&1 | tail -15`

Expected: all 4 unit tests + 6 integration tests pass.

- [ ] **Step 6: Run the full workspace test suite to catch cross-crate regressions**

Run: `cargo test --workspace --no-default-features 2>&1 | tail -25`

Expected: all tests pass. (Phase 8 only adds code; no existing tests should regress. If a Phase 4/6/7 test breaks, the cause is likely in the lib refactor from Task 8 — bisect by reverting Task 8 temporarily and checking if the breakage persists.)

- [ ] **Step 7: Commit**

```bash
git -C /workspace/llm-gateway add crates/usage-worker/src/lib.rs crates/usage-worker/src/main.rs
git -C /workspace/llm-gateway commit -m "feat(phase8): wire check_budget_alerts into usage-worker main loop"
```

---

## Task 12: CHANGELOG entry

User-facing documentation of the new behavior, immediately after the Phase 7 block.

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add the Phase 8 entry**

Open `CHANGELOG.md`. Find the Phase 7 block (search for `- **Phase 7 (budget observability):**`). The Phase 7 block ends at the line `  - **No behavior change:** enforcement remains as shipped in Phase 6 ...`. Add the Phase 8 block immediately **below** that line (and **above** the `### Changed` heading that follows):

```markdown
- **Phase 8 (budget alerts):**
  - **Behavior change:** when an org's month-to-date spend crosses 80% or 100% of its `default_budget_monthly_usd`, the gateway emails all of the org's admins and owners (verified addresses only). One alert per (org, month, threshold) — duplicates suppressed via a new `budget_alerts_sent` dedup table.
  - **Detection:** inline in the existing `usage-worker`. After `record_usage` succeeds, the worker evaluates thresholds and dispatches emails. Adds ~1 round-trip per usage event, only when a threshold is actually crossed (≤ 2 events per org per month).
  - **Configuration:** reuses the existing `server.public_base_url` config key (Phase 4, used for invitation links) to build the dashboard link in the email body. No new config keys.
  - **No new endpoints, no schema changes to existing tables, no frontend changes.** Additive only.
  - **Limitations:** alerts fire against the org-default budget only (not per-key budgets). Re-alerting after mid-month budget changes is not supported. Slack/webhook channels deferred.
```

- [ ] **Step 2: Verify the entry is in the right place**

Run: `grep -A 5 "Phase 8" /workspace/llm-gateway/CHANGELOG.md`

Expected: shows the Phase 8 block, followed by the existing `### Changed` heading.

- [ ] **Step 3: Commit**

```bash
git -C /workspace/llm-gateway add CHANGELOG.md
git -C /workspace/llm-gateway commit -m "docs(phase8): add Phase 8 budget-alerts entry to CHANGELOG"
```

---

## Verification (post-implementation)

After all 12 tasks land:

- [ ] **Run the full workspace test suite**

```bash
cargo test --workspace --no-default-features 2>&1 | tail -30
```

Expected: 0 failures. Phase 8 added: 8 storage tests + 1 email test + 4 worker unit tests + 6 worker integration tests = 19 new tests, all passing.

- [ ] **Run the frontend test suite to confirm no UI regressions**

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run 2>&1 | tail -20
```

Expected: 0 failures. Phase 8 doesn't touch the frontend, but the lib refactor in Task 8 touches the worker's runtime — confirm nothing in the e2e harness broke.

- [ ] **Manual smoke check (optional but recommended)**

Start the stack with `[email] transport = "file"` and `file_output_dir = "./outbox"`. Seed an org with a $10 budget and a verified admin. Send $8 worth of requests through the gateway. Confirm:

1. `./outbox/` contains one `.eml` file with `Subject: LLM Gateway: <org-name> budget at 80%`.
2. The body contains the org name, `$8.00`, `$10.00`, the current `YYYY-MM`, and a dashboard URL pointing at `<server.public_base_url>/<slug>/settings`.
3. Send $2 more in requests. Confirm a second `.eml` arrives with `100%` in the subject.
4. Send another $5. Confirm no third email — the dedup row for `(org, month, 80)` and `(org, month, 100)` is already sent.
