-- Add pricing_policy snapshot and weighted_tokens to usage_records
ALTER TABLE usage_records ADD COLUMN IF NOT EXISTS pricing_policy JSONB;
ALTER TABLE usage_records ADD COLUMN IF NOT EXISTS weighted_tokens BIGINT NOT NULL DEFAULT 0;
