-- Step 1: Create pricing_policies table
CREATE TABLE IF NOT EXISTS pricing_policies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    billing_type TEXT NOT NULL,
    config      TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pricing_policies_type ON pricing_policies(billing_type);

-- Step 2: Add columns to channels
ALTER TABLE channels ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channels ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;

-- Step 3: Add columns to channel_models
ALTER TABLE channel_models ADD COLUMN cost_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channel_models ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;

-- Step 4: Add column to models (pricing_policy_id)
ALTER TABLE models ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);

-- Step 5: Migrate existing model billing data to policies
INSERT INTO pricing_policies (id, name, billing_type, config, created_at, updated_at)
SELECT
    'policy-' || id,
    name || ' Policy',
    CASE billing_type
        WHEN 'token' THEN 'per_token'
        WHEN 'request' THEN 'per_request'
        ELSE 'per_token'
    END,
    CASE billing_type
        WHEN 'token' THEN json_object('input_per_1k', input_price, 'output_per_1k', output_price)
        WHEN 'request' THEN json_object('price_per_call', request_price)
        ELSE json_object('input_per_1k', 0, 'output_per_1k', 0)
    END,
    created_at,
    created_at
FROM models WHERE billing_type IS NOT NULL;

-- Step 6: Link models to their policies
UPDATE models SET pricing_policy_id = 'policy-' || id WHERE pricing_policy_id IS NULL AND billing_type IS NOT NULL;

-- Step 7: Remove billing_type from models (SQLite requires recreate)
PRAGMA foreign_keys = OFF;

CREATE TABLE models_new AS SELECT
    id, name, provider_id, model_type, pricing_policy_id,
    input_price, output_price, request_price, enabled, created_at
FROM models;
DROP TABLE models;
ALTER TABLE models_new RENAME TO models;

PRAGMA foreign_keys = ON;