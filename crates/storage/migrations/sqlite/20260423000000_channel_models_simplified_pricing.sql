-- Simplify channel_models: use pricing_policy_id like other tables
-- Remove per-field price columns (cost_policy_id, billing_type, input_price, output_price, request_price)
-- These are now handled entirely through pricing_policies

-- Add the new pricing_policy_id column (matches the naming convention used on models, channels)
ALTER TABLE channel_models ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);

-- Copy existing cost_policy_id values to the new column
UPDATE channel_models SET pricing_policy_id = cost_policy_id WHERE cost_policy_id IS NOT NULL;

-- Remove the old per-field pricing columns (only safe once all data uses pricing_policies)
ALTER TABLE channel_models DROP COLUMN cost_policy_id;
ALTER TABLE channel_models DROP COLUMN billing_type;
ALTER TABLE channel_models DROP COLUMN input_price;
ALTER TABLE channel_models DROP COLUMN output_price;
ALTER TABLE channel_models DROP COLUMN request_price;
