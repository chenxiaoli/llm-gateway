-- PostgreSQL migration: Remove provider_id and pricing_policy_id from models table
ALTER TABLE models DROP COLUMN IF EXISTS provider_id;
ALTER TABLE models DROP COLUMN IF EXISTS pricing_policy_id;