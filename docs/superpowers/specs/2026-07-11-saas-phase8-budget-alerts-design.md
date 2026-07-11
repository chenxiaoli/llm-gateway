# SaaS Phase 8: Budget Alerts — Design

**Targets release:** v2.5.0
**Built on top of:** Phase 7 (`budget-status` observability, `2026-07-10-saas-phase7-budget-observability-design.md`), Phase 6 (`budget_counters` + enforcement, `2026-07-10-saas-phase6-budget-enforcement-design.md`), Phase 4 (email subsystem + verified-email gate, `2026-07-09-saas-phase4-email-and-email-bound-invitations-design.md`)
**Date:** 2026-07-11

## Problem

Phase 6 enforces monthly budgets. Phase 7 makes month-to-date (MTD) spend visible in the UI. But enforcement is reactive — the first signal an operator gets is a `429 budget_exceeded` from the proxy, by which point a key is already blocked.

Phase 8 closes that gap with **email alerts** at two thresholds: 80% (warning) and 100% (over budget). When MTD crosses a threshold, every admin and owner of the org with a verified email receives a notification. One alert per `(org, month, threshold)` — duplicates suppressed.

The data is already there: `budget_counters` (Phase 6) has the accruals, `org_settings.default_budget_monthly_usd` (Phase 5) has the cap, `members` (Phase 1) has the recipient list, Phase 4's email stack handles delivery. Phase 8 wires a detection-and-dispatch loop on top.

## Goal

Send exactly one alert email per `(org, calendar month, threshold)` when the org's MTD spend crosses 80% or 100% of its `default_budget_monthly_usd`. Recipients: every admin and owner of the org with a verified email address.

## Non-Goals

1. **Per-key budget alerts.** Each key can have its own budget; alerts on per-key budgets would need a different dedup key (`(key_id, month, threshold)` not `(org_id, month, threshold)`). Future phase.
2. **Configurable thresholds.** v1 ships fixed 80%/100%. Custom thresholds per-org is a config-form + validation lift we don't need yet.
3. **Slack / webhook channels.** Email-only for v1.
4. **Re-alerting after mid-month budget changes.** Once alerted for `(org, month, threshold)`, no re-alert even if the admin bumps the budget. Dedup row stays for the month.
5. **Alert UI / "alerts sent" history page.** No frontend in this phase.
6. **Digest emails** ("daily summary of your org's spend"). Different shape; future phase.
7. **Per-recipient opt-out.** Recipient list is policy-derived (admins+owners), not user-settable. If a specific admin doesn't want alerts, they can filter on subject.
8. **Email bounce / feedback handling.** Out of scope; relies on SMTP-level feedback if it matters.
9. **Retry queue with backoff for permanently-failed sends.** The `sent_at = NULL` retry mechanism only fires when another usage event hits the same threshold window. For low-traffic orgs this could mean a long delay; acceptable for v1.
10. **Pre-dispatch cost estimation, hard org-level ceiling, historical dashboards, materialized `org_budget_counters`.** All carried forward as future-phase candidates from Phase 7's out-of-scope list.

## Decisions Locked (from brainstorming)

| Decision | Choice | Alternatives rejected |
|---|---|---|
| Trigger model | **Fixed 80% + 100% thresholds** | Configurable percentages per-org (more UI/test surface, no clear demand); Custom USD thresholds (different mental model); Both percentage + USD (overkill for v1) |
| Recipients | **All admins + owners of the org** | Only the crossing user (can miss the people who care); Per-user opt-in (more code, more friction); One configured email (limits visibility) |
| Channels | **Email only** | Email + Slack webhook (doubles project); Email + generic webhook (same scope cost) |
| Detection | **Inline in usage-worker, after `record_usage` commits** | Periodic sweep job (no existing cron in project, alert latency); Inside `record_usage` transaction (breaks storage-coupling boundary) |
| Architecture | **Inline send in usage-worker** | Dedicated alert-worker via NATS (extra crate, extra stream, deployment complexity); Inline + retry queue (same retry infra cost, more moving parts) |
| Dedup model | **`(org_id, month_bucket, threshold)` with two-phase `sent_at`** | Pure INSERT-ON-CONFLICT (loses alerts on send failure); Select-then-insert (duplicate emails on race) |
| Retry trigger | **Future request re-arms via `sent_at IS NULL` row** | Periodic sweep for unsent claims (no scheduler); Manual retry (no path) |
| Failure mode | **Log + ack the NATS message** | NAK + redeliver (would loop every usage event on persistent SMTP outage) |
| Per-key vs org | **Org-default budget only** | Both (different dedup key, different UX); Per-key only (loses the org-wide signal) |

## Architecture

### Data flow

```
proxy request → usage event published to NATS
  → usage-worker consumes
    → storage.record_usage(...)        [Phase 6: updates budget_counters atomically]
    → storage.check_budget_alerts(...)  [NEW Phase 8 step]
        → get_org_budget_for_alerts(org_id)
            returns BudgetAlertSnapshot { budget_units, accrued_units, org_name, org_slug }
        → if budget_units is None → skip (unlimited)
        → if budget_units == 0    → skip (defensive — see Edge Cases)
        → list_org_admin_emails(org_id)  [verified admins+owners]
        → if empty → log + skip
        → for threshold in [80, 100]:
            if accrued * 100 >= budget * threshold:
              try_claim_budget_alert(org_id, month, threshold)
                → None: another worker owns it / already sent → skip
                → Some(()): we won the claim → must send + mark sent
                  → render email template
                  → mailer.send(msg).await
                    → Ok:  mark_budget_alert_sent(org_id, month, threshold)
                    → Err: log warn, leave sent_at = NULL (future request retries)
    → existing per-user account deduction  [unchanged]
    → msg.ack()                            [unchanged]
```

**No new HTTP endpoints.** No new NATS streams. No new worker crates. The usage-worker gains an `Arc<dyn Mailer>` and one new code path.

### Component boundaries

| Component | Responsibility |
|---|---|
| `crates/storage/migrations/postgres/20260712000001_budget_alerts_sent.sql` | New `budget_alerts_sent` table (dedup ledger) |
| `crates/storage/src/lib.rs` | New trait methods + AppConfig extension for `[app] base_url` |
| `crates/storage/src/postgres.rs` | SQL impls for the four new methods (snapshot fetch, claim, mark-sent, recipient list) |
| `crates/storage/src/types.rs` | New `BudgetAlertSnapshot` struct |
| `crates/email/templates/budget_alert.{txt,html}.hbs` | New email template pair |
| `crates/usage-worker/src/main.rs` | Construct mailer at startup; call `check_budget_alerts` after `record_usage` success |
| `crates/usage-worker/src/budget_alerts.rs` | New file: `check_budget_alerts` orchestrator + threshold-comparison helper |
| `crates/usage-worker/Cargo.toml` | Add `llm-gateway-email` dependency |
| `crates/api/tests/phase8_budget_alerts.rs` | New integration tests for the alert flow |

### Why no new worker crate

The existing `usage-worker` already observes every cost event. Adding a third long-running worker (alongside `audit-worker`) just to send emails doubles operational surface for ~1 line of extra latency per usage event — and that latency only hits when a threshold is actually crossed (≤ 2 events per org per month). The threshold check is also stateless: it consults the DB for the current snapshot, so there's no in-memory state to lose on restart.

### Why no new NATS stream

The dedup logic is DB-mediated via `budget_alerts_sent`. Even if two workers race on the same threshold, the `INSERT ... ON CONFLICT DO NOTHING` pattern guarantees exactly one of them wins the claim. NATS-level partitioning would be belt-and-suspenders, adding operational complexity (new stream, new consumer, redelivery semantics to tune) without changing correctness.

## Data Model

### New table: `budget_alerts_sent`

Pure dedup ledger. One row per `(org, month, threshold)` that has been claimed for alerting.

```sql
-- 20260712000001_budget_alerts_sent.sql
CREATE TABLE budget_alerts_sent (
    org_id        TEXT        NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    month_bucket  TEXT        NOT NULL,            -- 'YYYY-MM' UTC calendar month (matches Phase 6 budget_counters)
    threshold     SMALLINT    NOT NULL,            -- 80 or 100 (per the fixed-thresholds decision)
    claimed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at       TIMESTAMPTZ,                     -- NULL until email send succeeds
    PRIMARY KEY (org_id, month_bucket, threshold)
);

CREATE INDEX budget_alerts_sent_unsent_idx
    ON budget_alerts_sent (org_id, month_bucket)
    WHERE sent_at IS NULL;    -- speeds the "any unclaimed in this org/month?" retry scan
```

Companion `.down.sql`: `DROP TABLE IF EXISTS budget_alerts_sent;`

**Design rationale:**

- **Two-phase `sent_at` column** lets one row play both "claim" (insert) and "success marker" (update) roles. Closes the failure-mode gap that pure INSERT-ON-CONFLICT leaves open.
- **No FK to `api_keys` or `users`.** Table is keyed on `orgs` only — alerts are org-scoped, not key-scoped.
- **`ON DELETE CASCADE` from `orgs`** keeps the table clean when an org is deleted.
- **No `recipient_user_ids[]` column.** Recipient list is derived (admins+owners with verified emails), not stored. Storing it would create a staleness problem if membership changes between alert time and audit time. If we ever need audit traceability of "who got this email," a separate `budget_alert_recipients` join table is the right shape — deferred.

### No changes to existing tables

Phase 8 is additive. No new columns on `orgs`, `api_keys`, `budget_counters`, `members`, or `users`.

## Storage trait additions

Four new methods on the `Storage` trait (`crates/storage/src/lib.rs`):

```rust
/// Phase 8: snapshot for budget alert evaluation. Fetches the org's
/// default budget (subunits, None if unlimited), current MTD spend, plus
/// name+slug for the email template — in one round-trip so the worker
/// can't observe a torn state.
async fn get_org_budget_for_alerts(
    &self,
    org_id: &str,
) -> Result<BudgetAlertSnapshot, Box<dyn std::error::Error + Send + Sync>>;

/// Phase 8: atomically claim the right to send an alert for (org, month, threshold).
/// Implementation does INSERT ... ON CONFLICT DO NOTHING inside a transaction
/// that also re-arms any pre-existing claim whose sent_at IS NULL (failed send
/// on a prior attempt). Returns Some(()) if this caller owns the claim and
/// must send + mark sent; None if another caller already owns a sent claim.
async fn try_claim_budget_alert(
    &self,
    org_id: &str,
    month_bucket: &str,
    threshold: i16,
) -> Result<Option<()>, Box<dyn std::error::Error + Send + Sync>>;

/// Phase 8: mark a claimed alert as successfully sent. Called after the
/// email send returns Ok. Idempotent — no-op if the row doesn't exist.
async fn mark_budget_alert_sent(
    &self,
    org_id: &str,
    month_bucket: &str,
    threshold: i16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Phase 8: verified emails of all admins+owners in the org. Skips members
/// with no email or unverified email (matches Phase 4's verified-email gate).
async fn list_org_admin_emails(
    &self,
    org_id: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
```

New type in `crates/storage/src/types.rs`:

```rust
/// Phase 8: snapshot for budget alert evaluation. budget_units is None when
/// the org has no default budget set (unlimited) — callers skip alerting.
#[derive(Debug, Clone)]
pub struct BudgetAlertSnapshot {
    pub budget_units: Option<i64>,    // 10^8 subunits per USD; None = unlimited
    pub accrued_units: i64,           // current MTD spend, same units
    pub org_name: String,             // for email template
    pub org_slug: String,             // for dashboard link in email
}
```

### SQL impls

**`get_org_budget_for_alerts`** — one round-trip, CTE-joined so budget and accrued share a snapshot timestamp:

```sql
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
LEFT JOIN budget b ON TRUE;
```

Decode rule: if `budget_units` column is SQL NULL → snapshot.budget_units = None (unlimited). Else Some(value). `accrued_units` is always a number (0 when no rows).

**`try_claim_budget_alert`** — atomic two-phase claim that re-arms on prior failure:

```sql
BEGIN;
-- Lock the (org, month) slot so concurrent callers serialize per-org.
-- Two callers crossing the same threshold in the same request batch
-- both need to know "did I win?" — this is what guarantees exactly-one.
INSERT INTO budget_alerts_sent (org_id, month_bucket, threshold)
VALUES ($1, $2, $3)
ON CONFLICT (org_id, month_bucket, threshold) DO NOTHING;

-- If a row exists with sent_at IS NULL, either:
--   (a) we just inserted it (claim by us), or
--   (b) a prior worker claimed it but failed to send — re-arm to us.
-- Update claimed_at to "now" to signal ownership transfer, then return
-- the row so the caller knows it owns the send.
UPDATE budget_alerts_sent
   SET claimed_at = NOW()
 WHERE org_id = $1 AND month_bucket = $2 AND threshold = $3
   AND sent_at IS NULL
RETURNING org_id;
COMMIT;
```

If the `RETURNING` yields a row → caller owns the send (`Some(())`). If empty → another worker already marked `sent_at` for this slot → caller skips (`None`).

**`mark_budget_alert_sent`:**

```sql
UPDATE budget_alerts_sent
   SET sent_at = NOW()
 WHERE org_id = $1 AND month_bucket = $2 AND threshold = $3
   AND sent_at IS NULL;
```

Idempotent; no-op if the row doesn't exist or is already marked sent.

**`list_org_admin_emails`:**

```sql
SELECT u.email
FROM members m
JOIN users u ON u.id = m.user_id
WHERE m.org_id = $1
  AND m.role IN ('admin', 'owner')
  AND u.email IS NOT NULL
  AND u.email_verified_at IS NOT NULL;
```

## Worker Logic

### `crates/usage-worker/src/budget_alerts.rs` (new file)

```rust
use std::sync::Arc;
use llm_gateway_email::{Mailer, EmailMessage};
use llm_gateway_storage::Storage;

/// After record_usage succeeds, evaluate whether the org's MTD just crossed
/// an alert threshold. Every failure is logged and swallowed — usage is
/// already recorded, so the NATS message must still be acked.
pub async fn check_budget_alerts(
    storage: &Arc<dyn Storage>,
    mailer: &Arc<dyn Mailer>,
    email_templates: &llm_gateway_email::Templates,
    app_base_url: Option<&str>,
    from_name: &str,
    org_id: &str,
) {
    if let Err(e) = run_check(storage, mailer, email_templates, app_base_url, from_name, org_id).await {
        tracing::warn!("[BUDGET-ALERT] check failed for org={}: {}", org_id, e);
    }
}

async fn run_check(
    storage: &Arc<dyn Storage>,
    mailer: &Arc<dyn Mailer>,
    email_templates: &llm_gateway_email::Templates,
    app_base_url: Option<&str>,
    from_name: &str,
    org_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let snapshot = storage.get_org_budget_for_alerts(org_id).await?;
    let Some(budget_units) = snapshot.budget_units else { return Ok(()); };
    if budget_units == 0 { return Ok(()); }

    let month_bucket = format!("{}", chrono::Utc::now().format("%Y-%m"));
    let recipients = storage.list_org_admin_emails(org_id).await?;
    if recipients.is_empty() {
        tracing::info!("[BUDGET-ALERT] org={} no eligible recipients; skipping", org_id);
        return Ok(());
    }

    for threshold in [80_i16, 100_i16] {
        if passes_threshold(snapshot.accrued_units, budget_units, threshold) {
            if storage.try_claim_budget_alert(org_id, &month_bucket, threshold).await?.is_some() {
                match send_alert(
                    mailer, email_templates, app_base_url, from_name,
                    &snapshot, threshold, &recipients,
                ).await {
                    Ok(()) => {
                        if let Err(e) = storage.mark_budget_alert_sent(org_id, &month_bucket, threshold).await {
                            tracing::warn!("[BUDGET-ALERT] failed to mark sent: {}", e);
                        }
                    }
                    Err(e) => {
                        // Leave sent_at NULL so a future request retries via the same row.
                        tracing::warn!(
                            "[BUDGET-ALERT] send failed org={} threshold={}: {}",
                            org_id, threshold, e,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

/// Integer-safe ratio check: accrued/budget >= threshold/100.
/// Uses only integer math — no float drift at the boundary.
pub(crate) fn passes_threshold(accrued: i64, budget: i64, threshold: i16) -> bool {
    // Caller guarantees budget > 0.
    accrued.saturating_mul(100) >= budget.saturating_mul(threshold as i64)
}

async fn send_alert(
    mailer: &Arc<dyn Mailer>,
    email_templates: &llm_gateway_email::Templates,
    app_base_url: Option<&str>,
    from_name: &str,
    snapshot: &BudgetAlertSnapshot,  // imported from storage::types
    threshold: i16,
    recipients: &[String],
) -> Result<(), llm_gateway_email::EmailError> {
    let accrued_usd = format_usd(snapshot.accrued_units);
    let budget_usd = format_usd(snapshot.budget_units.unwrap());  // safe: caller checked Some
    let dashboard_path = format!("/{}/settings", snapshot.org_slug);
    let dashboard_url = match app_base_url {
        Some(base) => format!("{}{}", base.trim_end_matches('/'), dashboard_path),
        None => dashboard_path,
    };

    let text = email_templates.render("budget_alert.txt", &serde_json::json!({
        "from_name":    from_name,
        "org_name":     snapshot.org_name,
        "org_slug":     snapshot.org_slug,
        "accrued_usd":  accrued_usd,
        "budget_usd":   budget_usd,
        "percent":      threshold,
        "month_bucket": chrono::Utc::now().format("%Y-%m").to_string(),
        "dashboard_url": dashboard_url,
    }))?;

    let html = email_templates.render("budget_alert.html", &serde_json::json!({
        // same vars
        ...same JSON as above...
    }))?;

    let subject = format!("{}: {} budget at {}%", from_name, snapshot.org_name, threshold);

    for recipient in recipients {
        let msg = EmailMessage {
            to: recipient.clone(),
            subject: subject.clone(),
            text_body: text.clone(),
            html_body: Some(html.clone()),
        };
        mailer.send(msg).await?;  // surfacing error to caller leaves sent_at NULL for retry
    }
    Ok(())
}

fn format_usd(units: i64) -> String {
    // 10^8 subunits per USD, 2 decimal places.
    let dollars = units / 100_000_000;
    let cents = (units % 100_000_000) / 1_000_000;
    format!("${}.${:02}", dollars, cents)
}
```

### `crates/usage-worker/src/main.rs` changes

Three additions:

1. **Construct mailer at startup** (mirror of `gateway/main.rs:90-117`):
   ```rust
   let mailer: Arc<dyn llm_gateway_email::Mailer> = match config.email.transport.as_str() { ... };
   let email_templates = llm_gateway_email::Templates::load(...)?;
   let app_base_url = config.app.as_ref().and_then(|a| a.base_url.clone());
   let from_name = config.email.from_name.clone();
   ```

2. **Thread the mailer + templates into `run_usage_worker`** signature.

3. **Call `check_budget_alerts` after `record_usage` succeeds** (between record_usage and the existing per-user deduction):
   ```rust
   if let Err(e) = storage.record_usage(&record.org_id, &record).await {
       // existing NAK path
   }

   // NEW: Phase 8
   crate::budget_alerts::check_budget_alerts(
       &storage, &mailer, &email_templates,
       app_base_url.as_deref(), &from_name,
       &record.org_id,
   ).await;

   // existing per-user deduction path
   ```

4. **Add `llm-gateway-email` to `crates/usage-worker/Cargo.toml`.**

### `AppConfig` extension

New optional section in `crates/storage/src/lib.rs`:

```rust
#[derive(Deserialize, Default)]
pub struct AppSection {
    pub base_url: Option<String>,  // e.g. "https://gateway.example.com"
}

// Add to AppConfig:
#[serde(default)]
pub app: AppSection,
```

If `app.base_url` is unset, the email template uses a relative URL (`/{slug}/settings`) — functional in single-tenant deployments where the gateway and frontend share an origin. Documented in CHANGELOG as recommended-for-production.

## Email Content

### Templates

Two new files in `crates/email/templates/`:

- `budget_alert.txt.hbs`
- `budget_alert.html.hbs`

### Subject

`{{from_name}}: {{org_name}} budget at {{percent}}%`

Examples:
- `LLM Gateway: Acme Corp budget at 80%`
- `LLM Gateway: Acme Corp budget at 100%`

### Plain-text body (`budget_alert.txt.hbs`)

```
{{org_name}} has used {{accrued_usd}} of its {{budget_usd}} monthly budget ({{percent}}%).

Current spend: {{accrued_usd}}
Monthly budget: {{budget_usd}}
Month: {{month_bucket}}

Review spending at {{dashboard_url}}

— {{from_name}}
```

### HTML body

Same content as plain-text, wrapped in Phase 4's existing HTML email shell (copy `password_reset.html.hbs` as the structural template, swap the body section). 100% threshold renders a red `Over budget` badge above the spend row; 80% threshold renders an amber `Approaching limit` badge. Both badges are simple `<span>` elements with inline styles — no new CSS assets.

### Template variables

| Variable | Source | Example |
|---|---|---|
| `org_name` | `BudgetAlertSnapshot.org_name` | `Acme Corp` |
| `org_slug` | `BudgetAlertSnapshot.org_slug` | `acme-corp` |
| `accrued_usd` | `format_usd(snapshot.accrued_units)` | `$40.00` |
| `budget_usd` | `format_usd(snapshot.budget_units)` | `$50.00` |
| `percent` | threshold that fired (80 or 100) — not the actual computed percentage, which could be e.g. 82.4% | `80` |
| `month_bucket` | `chrono::Utc::now().format("%Y-%m")` | `2026-07` |
| `dashboard_url` | `{app.base_url}/{slug}/settings` if configured, else `/{slug}/settings` | `https://gateway.example.com/acme-corp/settings` |
| `from_name` | `[email] from_name` config | `LLM Gateway` |

### Mailer reuse

No changes to the `Mailer` trait or `SmtpMailer`. The worker calls `mailer.send(msg).await` directly (not the `dispatch_with_retry` helper from Phase 4 — that one is fire-and-forget and doesn't surface errors to the caller, but Phase 8 needs the result to decide `mark_budget_alert_sent` vs leave-NULL).

## Edge Cases + Failure Handling

### Threshold semantics

| State | 80% alert | 100% alert |
|---|---|---|
| No org default budget (`NULL`) | skip (unlimited) | skip |
| Budget = `$0` | skip (defensive — see below) | skip |
| MTD exactly at 80% boundary | fires (≥ comparison) | — |
| MTD exactly at 100% boundary | — | fires |
| First request pushes MTD from 79% → 81% | fires | skips |
| First request pushes MTD from 99% → 101% | fires (if not already sent) | fires |
| One request pushes MTD from 50% → 110% | fires | fires |

The `budget_units == 0` skip exists because the integer check `accrued * 100 >= 0 * threshold` is always true → would alert on the first dollar, every month, forever. Admins who explicitly set `$0` as a budget already get Phase 6's 429 on every request, so the alert adds noise without value.

### Recipients edge cases

| State | Behavior |
|---|---|
| Org has 0 admins/owners | log `[BUDGET-ALERT] no eligible recipients`, claim row still inserted + sent_at stays NULL → future retry when an admin is added |
| Admin has `email = NULL` | excluded from recipients list |
| Admin has `email_verified_at = NULL` | excluded (matches Phase 4's verified-email gate) |
| Admin role = `member` | excluded |
| All admins bounced / invalid | SMTP send fails → claim left NULL → retried next request |
| Recipient list changes between claim and send | fine — send goes to whoever was on the list at send time |

### Failure handling

| Failure | Worker behavior | User-visible consequence |
|---|---|---|
| `record_usage` fails | existing behavior: NAK, NATS redelivers | unchanged from Phase 6 |
| `get_org_budget_for_alerts` fails | log warn, skip alert check, **ack the message** (usage is recorded) | alert lost for this request; future requests still trigger alerts |
| `try_claim_budget_alert` fails | log warn, skip threshold, ack | same |
| `list_org_admin_emails` fails | log warn, skip all thresholds, ack | same |
| `mailer.send()` fails for one recipient | log warn, leave `sent_at = NULL`, ack | future request retries via the same claim row |
| `mailer.send()` fails for **all** recipients | same as above | same |
| `mark_budget_alert_sent` fails | log warn, ack (email already went out — just the bookkeeping update failed) | alert sent but dedup row may be retried; recipient gets one duplicate email next request, then dedup row's `sent_at` gets set |
| Worker restarts mid-send | claim row left with `sent_at = NULL` | next request re-claims via the `sent_at IS NULL` re-arm path |

### The NULL-sent_at retry mechanism

`try_claim_budget_alert` does two operations in one transaction:
1. `INSERT ... ON CONFLICT DO NOTHING` — try to create the row.
2. `UPDATE ... SET claimed_at = NOW() WHERE sent_at IS NULL RETURNING org_id` — if we (or anyone else) hold an unsent claim, take ownership.

Outcomes:
- We just inserted → UPDATE returns our row → `Some(())`.
- A prior worker claimed but failed to send (sent_at NULL) → UPDATE returns the row → `Some(())` (we took ownership).
- Another worker already sent (sent_at NOT NULL) → UPDATE matches nothing → `None`.

This guarantees: failed sends get retried on the next threshold-crossing request, and successful sends are never re-sent.

### NATS message ack semantics

No matter what fails in `check_budget_alerts`, the message gets acked (usage is already persisted). The only failure that NAKs is `record_usage` itself — unchanged from today.

### Concurrent org operations

- **Org deleted between `record_usage` and `check_budget_alerts`:** `get_org_budget_for_alerts` returns no rows → snapshot decode fails → log + ack. The cascade FK on `budget_alerts_sent` cleans up.
- **Admin removed between `list_org_admin_emails` and send:** send goes to whoever was listed; no harm.
- **Budget raised mid-month:** `budget_alerts_sent` rows from the old (lower) budget stay. If admin bumps from $50 → $200, prior 80% alert stays sent; new MTD is 20% of new budget, no new alerts fire. Correct behavior — re-alerting after admin changes is a Non-Goal.
- **Budget lowered mid-month:** prior alerts stay sent. If admin lowers from $200 → $50, MTD may now be >80% → alerts fire on next request. Correct.

## Testing

### Storage unit (`crates/storage/src/postgres.rs`, in `invitation_tests` mod)

| Test | Verifies |
|---|---|
| `get_org_budget_for_alerts_returns_none_for_no_budget` | Fresh org, no `default_budget_monthly_usd` set → snapshot.budget_units is None |
| `get_org_budget_for_alerts_returns_budget_and_accrued` | Set $50 budget, seed $20 spend → snapshot matches (Some(5_000_000_000), 2_000_000_000, name, slug) |
| `get_org_budget_for_alerts_zero_accrued_when_no_spend` | Budget set, no usage records → accrued_units = 0 |
| `try_claim_budget_alert_first_caller_wins` | Two calls in sequence; first returns Some(()), second returns None (after sent_at set) |
| `try_claim_budget_alert_retries_after_failed_send` | Claim → don't mark sent → second call to `try_claim` returns Some(()) (re-arms via NULL sent_at) |
| `mark_budget_alert_sent_sets_timestamp` | Claim → mark → row's sent_at is non-NULL; subsequent `try_claim` returns None |
| `list_org_admin_emails_returns_admins_and_owners` | Seed 1 owner + 1 admin + 1 member + 1 admin-with-unverified-email → returns 2 verified emails |
| `list_org_admin_emails_empty_for_org_with_no_admins` | Edge case: empty result |

Reuses Phase 6/7 test helpers (`seed_org_with_budget_and_key`, `seed_usage_record`).

### API integration (`crates/api/tests/phase8_budget_alerts.rs` — new file)

| Test | Verifies |
|---|---|
| `budget_alert_flow_e2e` | Seed org + admin + verified-email user + $10 budget + $8 spend → call `check_budget_alerts` → assert (a) email dispatched via `TestMailer` recording, (b) dedup row exists with sent_at NOT NULL |
| `budget_alert_skips_when_no_budget` | Same setup but no `default_budget_monthly_usd` → no email dispatched, no dedup row |
| `budget_alert_skips_recipient_unverified` | Admin's `email_verified_at` IS NULL → no email dispatched, dedup row left unsent |
| `budget_alert_skips_zero_budget` | Budget = `$0` → no email, no dedup row (defensive skip) |
| `budget_alert_skips_when_no_recipients` | Org has only `member` role users → no email, dedup row left unsent |

The test constructs a `TestMailer` (in-test mock implementing `Mailer`) that records each `send` call, then asserts on the recorded list — mirrors how Phase 4 tests its mailer.

### Worker unit (`crates/usage-worker/src/budget_alerts.rs` — co-located tests)

Threshold-comparison edge cases (logic-only, no DB):

```rust
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
}
```

### Frontend unit

**No new frontend code in Phase 8.** Alerts are entirely server-side. The existing OrgSettings Budget status card (Phase 7) is the in-app view of the same data — no UI changes.

### E2E

**No e2e in Phase 8.** Email delivery isn't testable in the dev environment (Phase 4 also had this constraint). The integration tests above cover the alerting logic end-to-end at the storage + mailer layer, which is the only new code.

## CHANGELOG Entry

Under `## [Unreleased] → Added`, after the Phase 7 block:

```markdown
- **Phase 8 (budget alerts):**
  - **Behavior change:** when an org's month-to-date spend crosses 80% or 100% of its `default_budget_monthly_usd`, the gateway emails all of the org's admins and owners (verified addresses only). One alert per (org, month, threshold) — duplicates suppressed via a new `budget_alerts_sent` dedup table.
  - **Detection:** inline in the existing `usage-worker`. After `record_usage` succeeds, the worker evaluates thresholds and dispatches emails. Adds ~1 line of latency per usage event, only when a threshold is actually crossed (≤ 2 events per org per month).
  - **Configuration:** new optional `[app] base_url` config key — used for the dashboard link in the email body. Defaults to a relative URL if unset (single-tenant deployments).
  - **No new endpoints, no schema changes to existing tables, no frontend changes.** Additive only.
  - **Limitations:** alerts fire against the org-default budget only (not per-key budgets). Re-alerting after mid-month budget changes is not supported. Slack/webhook channels deferred.
```

**No upgrade note** — additive, no enforcement behavior change, no existing API contract changes. Deploy-safe.

## Out of Scope / Future Work

Carried forward from Phase 7 plus this phase's non-goals:

1. **Per-key budget alerts.** Different dedup key, different UX. Future phase.
2. **Configurable thresholds** (custom percentages per-org).
3. **Slack / webhook channels.**
4. **Re-alerting after mid-month budget changes.**
5. **Alert UI / "alerts sent" history page.**
6. **Digest emails** (daily spend summary).
7. **Per-recipient opt-out.**
8. **Email bounce / feedback handling.**
9. **Retry queue with backoff for permanently-failed sends.**
10. **Pre-dispatch cost estimation, hard org-level ceiling, historical dashboards, materialized `org_budget_counters`** — all from Phase 7's out-of-scope list.
