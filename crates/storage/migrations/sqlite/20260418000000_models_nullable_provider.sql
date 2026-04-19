-- Make provider_id nullable in models table
-- SQLite doesn't support ALTER COLUMN, so we recreate the table
ALTER TABLE models RENAME TO models_old;

CREATE TABLE IF NOT EXISTS models (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL UNIQUE,
    provider_id      TEXT REFERENCES providers(id),
    model_type       TEXT,
    pricing_policy_id TEXT REFERENCES pricing_policies(id),
    billing_type     TEXT NOT NULL DEFAULT 'per_token'
        CHECK(billing_type IN ('per_token', 'per_request', 'per_character', 'tiered_token', 'hybrid')),
    input_price      REAL NOT NULL DEFAULT 0,
    output_price     REAL NOT NULL DEFAULT 0,
    request_price    REAL NOT NULL DEFAULT 0,
    enabled          BOOLEAN NOT NULL DEFAULT 1,
    created_at       TEXT NOT NULL
);

INSERT INTO models (id, name, provider_id, model_type, pricing_policy_id, billing_type, input_price, output_price, request_price, enabled, created_at)
SELECT id, name, NULL, model_type, pricing_policy_id, billing_type, input_price, output_price, request_price, enabled, created_at
FROM models_old;

DROP TABLE models_old;