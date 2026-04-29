-- PostgreSQL migration: Add cache_creation_tokens to usage_records
ALTER TABLE usage_records ADD COLUMN IF NOT EXISTS cache_creation_tokens BIGINT;
