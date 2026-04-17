-- SQLite migration: Remove provider_id and pricing_policy_id from models table
-- Workaround for SQLite: recreate table without the columns (DROP COLUMN not supported in older SQLite)

CREATE TABLE models_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    model_type TEXT,
    billing_type TEXT NOT NULL DEFAULT 'per_token'
        CHECK(billing_type IN ('per_token', 'per_request', 'per_character', 'tiered_token', 'hybrid')),
    input_price REAL NOT NULL DEFAULT 0,
    output_price REAL NOT NULL DEFAULT 0,
    request_price REAL NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

INSERT INTO models_new (id, name, model_type, billing_type, input_price, output_price, request_price, enabled, created_at)
SELECT id, name, model_type, billing_type, input_price, output_price, request_price, enabled, created_at FROM models;

DROP TABLE models;

ALTER TABLE models_new RENAME TO models;

CREATE INDEX IF NOT EXISTS idx_models_name ON models(name);