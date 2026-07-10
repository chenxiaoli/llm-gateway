-- Materialized month-to-date spend counter, updated atomically with each
-- usage_records insert. Enables O(1) budget enforcement check at request
-- time without scanning usage_records per request.
--
-- Month bucket is UTC calendar month ('YYYY-MM') per Phase 6 design decision.
--
-- No data backfill: counter starts empty. New requests populate it. Historical
-- spend (pre-Phase-6) is intentionally NOT counted toward future-month
-- enforcement. See Phase 6 spec, "Data Model" section.
--
-- Drift risk: if usage_records is ever inserted outside record_usage(), this
-- counter will not reflect those new rows. Detect by comparing counter totals
-- to SUM(usage_records.cost) grouped by month if inconsistency is suspected.
CREATE TABLE IF NOT EXISTS budget_counters (
    key_id       TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    month_bucket TEXT NOT NULL,
    accrued      BIGINT NOT NULL DEFAULT 0,
    updated_at   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (key_id, month_bucket)
);

CREATE INDEX IF NOT EXISTS idx_budget_counters_month ON budget_counters(month_bucket);
