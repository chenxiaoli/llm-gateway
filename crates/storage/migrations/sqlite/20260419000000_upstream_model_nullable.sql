-- Make upstream_model_name nullable in channel_models table
-- SQLite doesn't support ALTER COLUMN, so we recreate the table
ALTER TABLE channel_models RENAME TO channel_models_old;

CREATE TABLE IF NOT EXISTS channel_models (
    id                  TEXT PRIMARY KEY,
    channel_id          TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    model_id            TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    upstream_model_name TEXT,
    priority_override   INTEGER,
    cost_policy_id      TEXT REFERENCES pricing_policies(id),
    markup_ratio        REAL NOT NULL DEFAULT 1.0,
    billing_type        TEXT,
    input_price         REAL,
    output_price        REAL,
    request_price       REAL,
    enabled             BOOLEAN NOT NULL DEFAULT 1,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(channel_id, model_id)
);

INSERT INTO channel_models (id, channel_id, model_id, upstream_model_name, priority_override, cost_policy_id, markup_ratio, billing_type, input_price, output_price, request_price, enabled, created_at, updated_at)
SELECT id, channel_id, model_id, NULL, priority_override, cost_policy_id, markup_ratio, billing_type, input_price, output_price, request_price, enabled, created_at, updated_at
FROM channel_models_old;

DROP TABLE channel_models_old;