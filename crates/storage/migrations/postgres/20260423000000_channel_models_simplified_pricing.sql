-- Simplify channel_models: use pricing_policy_id like other tables
-- Remove per-field price columns (cost_policy_id, billing_type, input_price, output_price, request_price)
-- These are now handled entirely through pricing_policies

-- Add the new pricing_policy_id column
ALTER TABLE channel_models ADD COLUMN IF NOT EXISTS pricing_policy_id TEXT REFERENCES pricing_policies(id);

-- Copy existing cost_policy_id values to the new column (only if column exists)
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'channel_models' AND column_name = 'cost_policy_id') THEN
        UPDATE channel_models SET pricing_policy_id = cost_policy_id WHERE cost_policy_id IS NOT NULL;
    END IF;
END $$;

-- Remove the old per-field pricing columns
ALTER TABLE channel_models DROP COLUMN IF EXISTS cost_policy_id;
ALTER TABLE channel_models DROP COLUMN IF EXISTS billing_type;
ALTER TABLE channel_models DROP COLUMN IF EXISTS input_price;
ALTER TABLE channel_models DROP COLUMN IF EXISTS output_price;
ALTER TABLE channel_models DROP COLUMN IF EXISTS request_price;
