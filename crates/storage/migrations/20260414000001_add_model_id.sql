-- Add id column to models table (using name as initial value)
ALTER TABLE models ADD COLUMN id TEXT;

-- Backfill id from name
UPDATE models SET id = name WHERE id IS NULL;

-- Recreate table with id as primary key (SQLite doesn't support ALTER TABLE for primary key)
PRAGMA foreign_keys = OFF;

CREATE TABLE models_new (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    billing_type TEXT NOT NULL CHECK(billing_type IN ('token', 'request')),
    input_price REAL NOT NULL DEFAULT 0,
    output_price REAL NOT NULL DEFAULT 0,
    request_price REAL NOT NULL DEFAULT 0,
    model_type  TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);

INSERT INTO models_new (id, name, provider_id, billing_type, input_price, output_price, request_price, model_type, enabled, created_at)
SELECT id, name, provider_id, billing_type, input_price, output_price, request_price, model_type, enabled, created_at FROM models;

DROP TABLE models;
ALTER TABLE models_new RENAME TO models;

PRAGMA foreign_keys = ON;
