-- Materialized month-to-date spend counter, updated atomically with each
-- usage_records insert. Enables O(1) budget enforcement check at request
-- time without scanning usage_records per request.
--
-- Month bucket is UTC calendar month ('YYYY-MM') per Phase 6 design decision.
-- Drift risk: if usage_records ever backfills outside record_usage(), this
-- counter lags. A future reconciliation job can recompute from SUM(cost).
CREATE TABLE IF NOT EXISTS budget_counters (
    key_id       TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    month_bucket TEXT NOT NULL,
    accrued      BIGINT NOT NULL DEFAULT 0,
    updated_at   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (key_id, month_bucket)
);

CREATE INDEX IF NOT EXISTS idx_budget_counters_month ON budget_counters(month_bucket);
